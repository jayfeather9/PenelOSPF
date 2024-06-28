use std::io::Write;
use std::net::Ipv4Addr;

use nom_derive::Parse;
use pnet::datalink;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;

use crate::config::*;
use crate::database::{DatabaseRequestType, DatabaseResponse};
use crate::interface::{Interface, InterfaceEvent, InterfaceState};
use crate::lsa::LsaCmpResult;
use crate::neighbor::{NbrEvent, NbrState, Neighbor};
use crate::packets::{DBDFlag, OSPFPacket};

pub fn ipv4_to_bits(ip: Ipv4Addr) -> u32 {
    u32::from_be_bytes(ip.octets())
}

impl Interface {
    // Check if the received packet's target ip is the same as
    // the interface's ip or the OSPF multicast address
    fn check_ip(&self, ip: &std::net::Ipv4Addr) -> bool {
        let ip = u32::from_be_bytes(ip.octets());
        let addr = u32::from_be_bytes(self.addr.octets());
        let ospf_multicast = 0xE000_0005; // 224.0.0.5
        (ip == addr) || (ip == ospf_multicast)
    }

    pub async fn receiver(&mut self) {
        let (_, mut rx) = match datalink::channel(&self.pnet_interface, Default::default()) {
            Ok(datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!("An error occurred when creating the channel: {}", e),
        };
        self.handle_event(InterfaceEvent::InterfaceUp).await;
        loop {
            // check if any timer is expired
            self.check_timers().await;
            // check if need to resend dbd
            self.send_dbd_if_need().await;
            // check if there's any nbr change
            if self.check_nbr_change().await {
                self.handle_event(InterfaceEvent::NeighborChange).await;
            }
            self.update_nbr_states().await;
            // send LSR packet if needed
            self.send_lsr_if_need().await;
            print!(".");
            // flush
            std::io::stdout().flush().unwrap();
            match rx.next() {
                Ok(packet) => {
                    let payload_offset = 14usize;
                    let packet = match Ipv4Packet::new(&packet[payload_offset..]) {
                        Some(packet) => packet,
                        None => {
                            println!("Error when parsing ipv4 packet, ignoring");
                            continue;
                        }
                    };
                    if packet.get_next_level_protocol() != IpNextHeaderProtocols::OspfigP
                        || packet.get_version() != 4
                        || !self.check_ip(&packet.get_destination())
                    {
                        continue;
                    }
                    if packet.get_flags() != 0 {
                        println!("Received OSPF packet with flags = {}", packet.get_flags());
                    }
                    let src_ip = packet.get_source();
                    let packet = match OSPFPacket::parse(packet.payload()) {
                        Ok((_, packet)) => packet,
                        Err(e) => {
                            eprintln!("Error while parsing ospf packet: {}", e);
                            continue;
                        }
                    };
                    if src_ip == self.addr {
                        continue;
                    }
                    if packet.get_hdr().packet_type > 1 {
                        println!("Received OSPF packet");
                    }
                    self.handle_packet(packet).await;
                }
                Err(e) => {
                    eprintln!("Error while reading new packet: {}", e);
                }
            }
        }
    }

    pub async fn handle_packet(&mut self, packet: OSPFPacket) {
        // Handle packet
        match packet {
            OSPFPacket::Hello(hello) => {
                self.handle_hello(hello).await;
            }
            OSPFPacket::DBDescription(dbd) => {
                self.handle_dbd(dbd).await;
            }
            OSPFPacket::LinkStateRequest(lsr) => {
                self.handle_lsr(lsr).await;
            }
            OSPFPacket::LinkStateUpdate(lsu) => {
                self.handle_lsu(lsu).await;
            }
            OSPFPacket::LinkStateAcknowledgment(lsa) => {
                println!("Received LinkStateAcknowledgment packet");
            }
        }
    }

    pub async fn handle_hello(&mut self, hello: crate::packets::Hello) {
        // println!("Received Hello packet: {:?}", hello);
        // print!(
        //     "Received Hello packet from {}, it's neighbor = ",
        //     Ipv4Addr::from(hello.header.router_id)
        // );
        // for n in hello.neighbors.iter() {
        //     print!("{}, ", Ipv4Addr::from(*n));
        // }
        // println!();

        // TODO: check 'E' bit, check if hello settings same
        // TODO: for some network, ip = src_ip, for others, ip = router_id
        let router_id = Ipv4Addr::from(hello.header.router_id);
        if self.get_neighbor_index(router_id).is_none() {
            self.neighbor_list.push(Neighbor::new(
                hello.clone(),
                self.dead_interval,
                self.rxmt_interval,
            ));
        }

        let self_clone = self.clone();

        let sender_nbr_idx = self.get_neighbor_index(router_id).unwrap();
        let sender_nbr = &mut self.neighbor_list[sender_nbr_idx];
        sender_nbr.nbr_ip = Ipv4Addr::from(hello.header.router_id);
        sender_nbr.nbr_id = hello.header.router_id;
        let prev_neighbor_dr = sender_nbr.nbr_dr;
        sender_nbr.nbr_dr = Ipv4Addr::from(hello.designated_router);
        let prev_neighbor_bdr = sender_nbr.nbr_bdr;
        sender_nbr.nbr_bdr = Ipv4Addr::from(hello.backup_designated_router);
        sender_nbr.nbr_pri = hello.router_priority;
        sender_nbr.handle_event(NbrEvent::HelloReceived);
        let my_config = self_clone.query_config().await;
        // if self is in hello's neighbor list, 2-way receive, else 1-way receive
        if hello.neighbors.contains(&my_config.router_id) {
            sender_nbr.handle_event(NbrEvent::TwoWayReceived);
        } else {
            // println!("1-way receive, neighbor = {:?}", hello.neighbors);
            sender_nbr.handle_event(NbrEvent::OneWayReceived);
            // for 1-way receive situation, end packet processing
            return;
        }
        // if the neighbor declares itself as DR or BDR when interface is Waiting, interface call BackupSeen event
        // if the neighbor's DR or BDR declare status changes, interface call NeighborChange event
        if (sender_nbr.nbr_dr == sender_nbr.nbr_ip || sender_nbr.nbr_bdr == sender_nbr.nbr_ip)
            && self.state == InterfaceState::Waiting
        {
            self.handle_event(InterfaceEvent::BackupSeen).await;
        } else if ((prev_neighbor_dr == sender_nbr.nbr_ip)
            ^ (sender_nbr.nbr_dr == sender_nbr.nbr_ip))
            || ((prev_neighbor_bdr == sender_nbr.nbr_ip)
                ^ (sender_nbr.nbr_bdr == sender_nbr.nbr_ip))
        {
            self.handle_event(InterfaceEvent::NeighborChange).await;
        }
    }

    pub async fn handle_dbd(&mut self, dbd: crate::packets::DBDescription) {
        println!(
            "Received DBDescription packet from {}, flags {:#?}, seq_num {}",
            Ipv4Addr::from(dbd.header.router_id),
            DBDFlag::from_byte(dbd.flags),
            dbd.dbd_seq_num
        );

        let self_clone = self.clone();

        let router_id = Ipv4Addr::from(dbd.header.router_id);
        if self.get_neighbor_index(router_id).is_none() {
            // get a dbd not in nbr list, drop it
            return;
        }
        let sender_nbr_idx = self.get_neighbor_index(router_id).unwrap();
        let sender_nbr = &mut self.neighbor_list[sender_nbr_idx];
        // check if dbd packet is duplicate
        let dbd_duped = (!sender_nbr.last_rcv_dbd.is_none())
            && (dbd.dbd_seq_num == sender_nbr.last_rcv_dbd.as_ref().unwrap().dbd_seq_num);
        // update last dbd packet
        sender_nbr.last_rcv_dbd = Some(dbd);

        // if state is init, and if after handle TwoWayReceived event,
        // state is ExStart,  then continue processing
        if sender_nbr.state == NbrState::Init {
            sender_nbr.handle_event(NbrEvent::TwoWayReceived);
            if sender_nbr.state != NbrState::ExStart {
                // it should only be ExStart or Twoway
                assert!(sender_nbr.state == NbrState::TwoWay);
                return;
            }
        }

        let dbd = sender_nbr.last_rcv_dbd.as_ref().unwrap();
        let mut dbd_accepted = false;
        match sender_nbr.state {
            NbrState::Down | NbrState::Attempt | NbrState::TwoWay => {
                // packet should be refused
                return;
            }
            NbrState::ExStart => {
                let my_router_id = self_clone.query_config().await.router_id;
                println!(
                    "dbd router id: {}, my router id: {}",
                    router_id,
                    Ipv4Addr::from(my_router_id)
                );
                if dbd.all_flag_set() && sender_nbr.nbr_id > my_router_id {
                    // if all flags are set and neighbor's router id is greater than self's router id
                    // neighbot is master
                    sender_nbr.nbr_is_master = true;
                    sender_nbr.dd_seq_number = dbd.dbd_seq_num;
                    println!(
                        "[Negotiation Done] Neighbor {} is master",
                        sender_nbr.nbr_ip
                    );
                } else if !dbd.get_flag().init
                    && !dbd.get_flag().masterslave
                    && dbd.dbd_seq_num == sender_nbr.dd_seq_number
                    && sender_nbr.nbr_id < my_router_id
                {
                    // if neither init nor master slave flag is set, and sequence number matches
                    // and neighbor's router id is less than self's router id
                    // self is master
                    sender_nbr.nbr_is_master = false;
                    println!(
                        "[Negotiation Done] Neighbor {} is slave, seq = {}",
                        sender_nbr.nbr_ip, sender_nbr.dd_seq_number
                    );
                    dbd_accepted = true;
                } else {
                    // if none of the above conditions are met, packet should be refused
                    println!("Refusing packet");
                    return;
                }
                // if one of the conditions met, set options and call NegotiationDone event
                sender_nbr.nbr_options = dbd.options;
                sender_nbr.handle_event(NbrEvent::NegotiationDone);
                // put all LSA in lsdb to nbr's db summary list
                let all_lsas = match self_clone
                    .query_database(DatabaseRequestType::QueryAllLsa)
                    .await
                {
                    DatabaseResponse::LsaList(lsas) => lsas,
                    _ => panic!("Unexpected response"),
                };
                sender_nbr
                    .db_summary_list
                    .extend(all_lsas.iter().map(|x| x.get_hdr().clone()));
                // now negotiation is done
                if sender_nbr.nbr_is_master {
                    // if i am slave, send the first packet
                    let max_transmit_size =
                        (self_clone.query_config().await.default_mtu as usize - 100) / (20);
                    let real_size =
                        std::cmp::min(max_transmit_size, sender_nbr.db_summary_list.len());
                    let lsa_hdrs = if real_size > 0 {
                        sender_nbr.db_summary_list[..real_size].to_vec()
                    } else {
                        vec![]
                    };
                    let more = sender_nbr.db_summary_list.len() > real_size;

                    self_clone
                        .send_dbd_packet(
                            sender_nbr.nbr_ip,
                            DBDFlag::new(false, more, false).to_byte(),
                            sender_nbr.dd_seq_number,
                            lsa_hdrs.clone(),
                        )
                        .await;
                    sender_nbr.last_sent_dbd = Some((
                        DBDFlag::new(false, more, false).to_byte(),
                        sender_nbr.dd_seq_number,
                        lsa_hdrs,
                    ));
                }
            }
            NbrState::Exchange => {
                if dbd_duped {
                    // for master, if dup, drop the packet
                    // for slave, if dup, retransmit last packet
                    if sender_nbr.nbr_is_master {
                        // retransmit last packet
                        // TODO: test this feature
                        if let Some((flags, seq_num, lsa_hdrs)) = sender_nbr.last_sent_dbd.clone() {
                            self_clone
                                .send_dbd_packet(sender_nbr.nbr_ip, flags, seq_num, lsa_hdrs)
                                .await;
                        }
                    }
                    return;
                }
                if dbd.get_flag().masterslave != sender_nbr.nbr_is_master
                    || dbd.get_flag().init
                    || dbd.options != sender_nbr.nbr_options
                {
                    // if master/slave flag is not same as self's, or init flag is set,
                    // or options don't match, drop the packet & call SeqNumberMismatch event
                    sender_nbr.handle_event(NbrEvent::SeqNumberMismatch);
                    return;
                }
                if (sender_nbr.nbr_is_master && dbd.dbd_seq_num != sender_nbr.dd_seq_number + 1)
                    || (!sender_nbr.nbr_is_master && dbd.dbd_seq_num != sender_nbr.dd_seq_number)
                {
                    // if master and sequence number doesn't match, drop the packet
                    sender_nbr.handle_event(NbrEvent::SeqNumberMismatch);
                    return;
                }
                // accept
                dbd_accepted = true;
            }
            NbrState::Loading | NbrState::Full => {
                if !dbd_duped {
                    // here we should only receive duped packets
                    sender_nbr.handle_event(NbrEvent::SeqNumberMismatch);
                    return;
                }
                if sender_nbr.nbr_is_master {
                    // if i am slave, and i receive a duped packet, retransmit last packet
                    // retransmit last packet
                    // TODO: test this feature
                    if let Some((flags, seq_num, lsa_hdrs)) = sender_nbr.last_sent_dbd.clone() {
                        self_clone
                            .send_dbd_packet(sender_nbr.nbr_ip, flags, seq_num, lsa_hdrs)
                            .await;
                    }
                }
                return;
            }
            _ => {
                // Init should be handled earlier
                assert!(sender_nbr.state != NbrState::Init);
            }
        };

        let dbd = sender_nbr.last_rcv_dbd.as_ref().unwrap();
        if dbd_accepted {
            for lsahdr in dbd.lsa_hdrs.iter() {
                // if lsa is not in database summary list, add it
                if self_clone.query_by_lsa_hdr(lsahdr.clone()).await.is_none() {
                    sender_nbr.link_state_req_list.push(lsahdr.clone());
                }
            }
            println!("Updated lsr list: {:?}", sender_nbr.link_state_req_list);

            // delete all LSA in last_sent_dbd from db_summary_list
            for lsahdr in sender_nbr.last_sent_dbd.as_ref().unwrap().2.iter() {
                sender_nbr.db_summary_list.retain(|x| !x.same_ids(lsahdr));
            }
            if sender_nbr.nbr_is_master {
                // if i am slave, set seq & retransmit
                sender_nbr.dd_seq_number = dbd.dbd_seq_num;
                // TODO: test this feature
                let max_transmit_size =
                    (self_clone.query_config().await.default_mtu as usize - 100) / (20);
                let real_size = std::cmp::min(max_transmit_size, sender_nbr.db_summary_list.len());
                let is_last = sender_nbr.db_summary_list.len() == real_size;
                let lsa_hdrs;
                if sender_nbr.db_summary_list.len() != 0 {
                    lsa_hdrs = sender_nbr.db_summary_list[..real_size].to_vec();
                    sender_nbr.db_summary_list = sender_nbr.db_summary_list[real_size..].to_vec();
                } else {
                    lsa_hdrs = vec![];
                    sender_nbr.db_summary_list.clear();
                }
                let sending_flags =
                    DBDFlag::new(false, !is_last, !sender_nbr.nbr_is_master).to_byte();
                self_clone
                    .send_dbd_packet(
                        sender_nbr.nbr_ip,
                        sending_flags,
                        sender_nbr.dd_seq_number,
                        lsa_hdrs.clone(),
                    )
                    .await;
                sender_nbr.last_sent_dbd =
                    Some((sending_flags, sender_nbr.dd_seq_number, lsa_hdrs));
                if is_last && !dbd.get_flag().more {
                    sender_nbr.handle_event(NbrEvent::ExchangeDone);
                }
            } else {
                // if i am master, increment seq
                sender_nbr.dd_seq_number += 1;
                if !dbd.get_flag().more {
                    // if more bit is unset, call ExchangeDone event
                    sender_nbr.mst_exch_timer.stop();
                    sender_nbr.handle_event(NbrEvent::ExchangeDone);
                } else {
                    // else, send a new DBD packet
                    let max_transmit_size =
                        (self_clone.query_config().await.default_mtu as usize - 100) / (20);
                    let real_size =
                        std::cmp::min(max_transmit_size, sender_nbr.db_summary_list.len());
                    let is_last = sender_nbr.db_summary_list.len() == real_size;
                    let lsa_hdrs;
                    if sender_nbr.db_summary_list.len() != 0 {
                        lsa_hdrs = sender_nbr.db_summary_list[..real_size].to_vec();
                        sender_nbr.db_summary_list =
                            sender_nbr.db_summary_list[real_size..].to_vec();
                    } else {
                        lsa_hdrs = vec![];
                        sender_nbr.db_summary_list.clear();
                    }
                    let sending_flags =
                        DBDFlag::new(false, !is_last, !sender_nbr.nbr_is_master).to_byte();
                    self_clone
                        .send_dbd_packet(
                            sender_nbr.nbr_ip,
                            sending_flags,
                            sender_nbr.dd_seq_number,
                            lsa_hdrs.clone(),
                        )
                        .await;
                    sender_nbr.last_sent_dbd =
                        Some((sending_flags, sender_nbr.dd_seq_number, lsa_hdrs));
                    sender_nbr.mst_exch_timer.start();
                }
            }
        }
    }

    pub async fn handle_lsr(&mut self, lsr: crate::packets::LinkStateRequest) {
        println!("Received LinkStateRequest packet: {:?}", lsr);

        let query_int = self.clone();

        let router_id = Ipv4Addr::from(lsr.header.router_id);
        if self.get_neighbor_index(router_id).is_none() {
            // get a lsr not in nbr list, drop it
            return;
        }
        let sender_nbr_idx = self.get_neighbor_index(router_id).unwrap();
        let sender_nbr = &mut self.neighbor_list[sender_nbr_idx];

        if !(sender_nbr.state == NbrState::Exchange
            || sender_nbr.state == NbrState::Loading
            || sender_nbr.state == NbrState::Full)
        {
            // if not in Exchange or Loading or Full state, drop the packet
            return;
        }

        match query_int.query_multi_lsa(lsr).await {
            Some(lsas) => {
                // TODO: send LinkStateUpdate packet
                println!("Sending requested LSAs: {:?}", lsas);
                self.send_lsu(router_id, lsas).await;
            }
            None => {
                // if not all requested LSAs are found, drop the packet & call BadLSReq event
                sender_nbr.handle_event(NbrEvent::BadLSReq);
            }
        }
    }

    pub async fn handle_lsu(&mut self, lsu: crate::packets::LinkStateUpdate) {
        print!(
            "Received LinkStateUpdate packet from {}, LSAs:",
            Ipv4Addr::from(lsu.header.router_id)
        );
        for lsa in lsu.lsas.iter() {
            print!(
                " [type: {}, ls_id: {}, ad_rtr: {}]",
                lsa.get_hdr().ls_type,
                Ipv4Addr::from(lsa.get_hdr().link_state_id),
                Ipv4Addr::from(lsa.get_hdr().advertising_router)
            );
        }
        println!();

        let self_clone = self.clone();

        let router_id = Ipv4Addr::from(lsu.header.router_id);
        if self.get_neighbor_index(router_id).is_none() {
            // get a lsr not in nbr list, drop it
            return;
        }
        let sender_nbr_idx = self.get_neighbor_index(router_id).unwrap();
        let sender_nbr = &mut self.neighbor_list[sender_nbr_idx];

        for lsa in lsu.lsas.iter() {
            let hdr = lsa.get_hdr();
            // TODO: 1. check the checksum
            // 2. check ls_type
            if hdr.ls_type == 0 || hdr.ls_type > 5 {
                // ls_type is invalid, drop the packet
                continue;
            }
            let mut lsdb_ver = self_clone.query_by_lsa_hdr(hdr.clone()).await;
            // TODO: 3. if is AS-external-LSA (type-5) and i am in stub area, drop the packet
            // 4. if LS age is equal to MaxAge and lsdb doesn't have this LSA
            if hdr.age == MAX_AGE as u16 && lsdb_ver.is_none() {
                // send a LSAck to Ack to this LSA
                self_clone.send_lsack(router_id, vec![hdr.clone()]).await;
                // drop the packet
                continue;
            }
            // 5. if LSA not in lsdb or is newer
            if lsdb_ver.is_some() {
                match lsdb_ver.clone().unwrap().cmp_with(lsa) {
                    LsaCmpResult::Older => {
                        // if in lsdb and lsdb ver is older
                        let old_ver = lsdb_ver.unwrap();
                        // a. if older one and new one arrived in no more than MIN_LS_ARRIVAL(=1) seconds
                        if old_ver.get_hdr().age as u32 == hdr.age as u32 {
                            // drop the packet
                            continue;
                        }
                        // c. delete the current version (done in later action "addorupdate")
                        // treat it as None, do the follow things in code
                        lsdb_ver = None;
                    }
                    LsaCmpResult::Same => {
                        // 7.a. if in nbr's rxmt list then remove it from the list
                        sender_nbr
                            .lsa_retransmission_list
                            .retain(|x| !lsa.same_ids(x));
                        // 7.b. send LSAck
                        self_clone.send_lsack(router_id, vec![hdr.clone()]).await;
                        continue;
                    }
                    LsaCmpResult::Newer => {
                        // 6. if in nbr's lsr list
                        if sender_nbr
                            .link_state_req_list
                            .iter()
                            .any(|x| lsa.same_ids(x))
                        {
                            // call BadLSReq event and stop handling this whole LSU
                            sender_nbr.handle_event(NbrEvent::BadLSReq);
                            return;
                        }
                        // 8. if newer one is in lsdb
                        // if ls_age == MaxAge && ls_seq == MaxSeqNum, drop the packet
                        if hdr.age == MAX_AGE as u16
                            && hdr.sequence_number == MAX_SEQ_NUM_LSA as u32
                        {
                            continue;
                        }
                        // directly send a LSU to update
                        self_clone
                            .send_lsu(router_id, vec![lsdb_ver.unwrap()])
                            .await;
                        continue;
                    }
                }
            }
            // so now this means we have no this LSA or "the new one is newer"
            if lsdb_ver.is_none() {
                // if in nbr's lsr list then remove it from the list
                sender_nbr.link_state_req_list.retain(|x| !lsa.same_ids(x));
                // b. flood the LSA
                self_clone.flood_lsu(vec![lsa.clone()]).await;
                // c. delete current ver. in nbr's rxmt list
                sender_nbr
                    .lsa_retransmission_list
                    .retain(|x| !lsa.same_ids(x));
                // d. add or update the LSA
                self_clone
                    .query_database(DatabaseRequestType::AddOrUpdateLsa(lsa.clone()))
                    .await;
                // e. send LSAck
                self_clone.send_lsack(router_id, vec![hdr.clone()]).await;
                // f. if is self-originated
                // TODO
            }
            if sender_nbr.state == NbrState::Loading && sender_nbr.link_state_req_list.len() == 0 {
                // if all requested LSAs are received, call LoadingDone event
                sender_nbr.handle_event(NbrEvent::LoadingDone);
            }
        }
    }
}
