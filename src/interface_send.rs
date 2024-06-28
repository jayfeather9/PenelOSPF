use std::net::Ipv4Addr;

use crate::interface::Interface;
use crate::lsa::{Lsa, LsaHeader};
use crate::neighbor::NbrState;
use crate::packets::DBDFlag;
use crate::receiver::ipv4_to_bits;
use crate::sender::{SenderRequestType, SenderResponse};

impl Interface {
    pub async fn send_hello(&self) {
        let my_config = self.query_config().await;
        // send hello packet
        let mut hello_packet = crate::packets::Hello {
            header: crate::packets::Header {
                version: 2,
                packet_type: crate::packets::PacketType::Hello as u8,
                packet_length: 0,
                router_id: my_config.router_id,
                area_id: my_config.area_id,
                checksum: 0,
                auth_type: 0,
                auth: 0,
            },
            network_mask: my_config.network_mask,
            hello_interval: my_config.hello_interval as u16,
            options: my_config.options,
            router_priority: my_config.router_priority,
            router_dead_interval: my_config.router_dead_interval,
            designated_router: ipv4_to_bits(self.designated_router),
            backup_designated_router: ipv4_to_bits(self.backup_designated_router),
            neighbors: vec![],
        };
        for nbr in self.neighbor_list.iter() {
            hello_packet.neighbors.push(ipv4_to_bits(nbr.nbr_ip));
        }
        match self
            .query_sender(SenderRequestType::SendOSPFPacket(
                crate::packets::OSPFPacket::Hello(hello_packet),
                (self.addr, Ipv4Addr::new(224, 0, 0, 5)),
                self.pnet_interface.clone(),
            ))
            .await
        {
            SenderResponse::PacketSent => {}
            SenderResponse::Failed => panic!("Error sending hello packet"),
        }
    }

    pub async fn send_dbd_packet(
        &self,
        ip: Ipv4Addr,
        flags: u8,
        dbd_seq_num: u32,
        lsa_hdrs: Vec<LsaHeader>,
    ) {
        let my_config = self.query_config().await;
        let dbd_packet = crate::packets::DBDescription {
            header: crate::packets::Header {
                version: 2,
                packet_type: crate::packets::PacketType::DBD as u8,
                packet_length: 0,
                router_id: my_config.router_id,
                area_id: my_config.area_id,
                checksum: 0,
                auth_type: 0,
                auth: 0,
            },
            interface_mtu: my_config.default_mtu,
            options: my_config.options,
            flags,
            dbd_seq_num,
            lsa_hdrs,
        };
        match self
            .query_sender(SenderRequestType::SendOSPFPacket(
                crate::packets::OSPFPacket::DBDescription(dbd_packet),
                (self.addr, ip),
                self.pnet_interface.clone(),
            ))
            .await
        {
            SenderResponse::PacketSent => {}
            SenderResponse::Failed => panic!("Error sending DBD packet"),
        }
    }

    pub async fn send_dbd_if_need(&mut self) {
        let sdr_clone = self.clone();
        for nbr in self.neighbor_list.iter_mut() {
            if nbr.state == NbrState::ExStart && nbr.exstart_rxmt_timer.is_expired() {
                sdr_clone
                    .send_dbd_packet(
                        nbr.nbr_ip,
                        DBDFlag::get_all_set().to_byte(),
                        nbr.dd_seq_number,
                        vec![],
                    )
                    .await;
                nbr.last_sent_dbd =
                    Some((DBDFlag::get_all_set().to_byte(), nbr.dd_seq_number, vec![]));
                nbr.exstart_rxmt_timer.start();
            } else if nbr.state == NbrState::Exchange && nbr.mst_exch_timer.is_expired() {
                let (flags, seq, lsa_hdrs) = nbr.last_sent_dbd.clone().unwrap();
                sdr_clone
                    .send_dbd_packet(nbr.nbr_ip, flags, seq, lsa_hdrs)
                    .await;
                nbr.mst_exch_timer.start();
            }
        }
    }

    pub async fn send_lsr_if_need(&mut self) {
        use crate::packets::{LinkStateRequest, LinkStateRequestItem};
        let sdr_clone = self.clone();
        for nbr in self.neighbor_list.iter_mut() {
            // only send when loading
            if nbr.state != NbrState::Loading {
                continue;
            }
            if !nbr.lsr_rxmt_timer.is_up() {
                nbr.lsr_rxmt_timer.start_imm();
            }
            if !nbr.lsr_rxmt_timer.is_expired() {
                continue;
            }
            let mut lsa_hdrs = vec![];
            for lsa in nbr.link_state_req_list.iter() {
                lsa_hdrs.push(LinkStateRequestItem {
                    link_state_type: lsa.ls_type as u32,
                    link_state_id: lsa.link_state_id,
                    advertising_router: lsa.advertising_router,
                });
            }
            println!("Sending LSR requesting for {:?}", lsa_hdrs);
            let my_config = sdr_clone.query_config().await;
            let lsr_packet = crate::packets::LinkStateRequest {
                header: crate::packets::Header {
                    version: 2,
                    packet_type: crate::packets::PacketType::LSR as u8,
                    packet_length: 0,
                    router_id: my_config.router_id,
                    area_id: my_config.area_id,
                    checksum: 0,
                    auth_type: 0,
                    auth: 0,
                },
                requests: lsa_hdrs,
            };
            match sdr_clone
                .query_sender(SenderRequestType::SendOSPFPacket(
                    crate::packets::OSPFPacket::LinkStateRequest(lsr_packet),
                    (sdr_clone.addr, nbr.nbr_ip),
                    sdr_clone.pnet_interface.clone(),
                ))
                .await
            {
                SenderResponse::PacketSent => {}
                SenderResponse::Failed => panic!("Error sending LSR packet"),
            }
            nbr.lsr_rxmt_timer.start();
        }
    }

    pub async fn send_lsu(&self, ip: Ipv4Addr, lsas: Vec<Lsa>) {
        let my_config = self.query_config().await;
        let lsu = crate::packets::LinkStateUpdate {
            header: crate::packets::Header {
                version: 2,
                packet_type: crate::packets::PacketType::LSU as u8,
                packet_length: 0,
                router_id: my_config.router_id,
                area_id: my_config.area_id,
                checksum: 0,
                auth_type: 0,
                auth: 0,
            },
            num_lsa: lsas.len() as u32,
            lsas,
        };
        match self
            .query_sender(SenderRequestType::SendOSPFPacket(
                crate::packets::OSPFPacket::LinkStateUpdate(lsu),
                (self.addr, ip),
                self.pnet_interface.clone(),
            ))
            .await
        {
            SenderResponse::PacketSent => {}
            SenderResponse::Failed => panic!("Error sending LSU packet"),
        }
    }

    pub async fn flood_lsu(&self, lsas: Vec<Lsa>) {
        self.send_lsu(Ipv4Addr::new(224, 0, 0, 5), lsas.clone())
            .await;
    }

    pub async fn send_lsack(&self, ip: Ipv4Addr, lsas: Vec<LsaHeader>) {
        let my_config = self.query_config().await;
        let lsack = crate::packets::LinkStateAcknowledgment {
            header: crate::packets::Header {
                version: 2,
                packet_type: crate::packets::PacketType::LSAck as u8,
                packet_length: 0,
                router_id: my_config.router_id,
                area_id: my_config.area_id,
                checksum: 0,
                auth_type: 0,
                auth: 0,
            },
            lsas,
        };
        match self
            .query_sender(SenderRequestType::SendOSPFPacket(
                crate::packets::OSPFPacket::LinkStateAcknowledgment(lsack),
                (self.addr, ip),
                self.pnet_interface.clone(),
            ))
            .await
        {
            SenderResponse::PacketSent => {}
            SenderResponse::Failed => panic!("Error sending LSAck packet"),
        }
    }
}
