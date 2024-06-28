use crate::config::Config;
use crate::database::DatabaseRequest;
use crate::neighbor::{NbrState, Neighbor};
use crate::packets::DBDFlag;
use crate::receiver::ipv4_to_bits;
use crate::sender::SenderRequest;
use crate::timer::Timer;
use pnet::datalink::NetworkInterface;
use pnet::ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub int_type: InterfaceType,
    pub state: InterfaceState,
    pub addr: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub area_id: u32,
    pub hello_interval: u32,
    pub dead_interval: u32,
    pub inf_transit_delay: u32,
    pub router_priority: u8,
    pub hello_timer: Timer,
    pub wait_timer: Timer,
    pub designated_router: Ipv4Addr,
    pub backup_designated_router: Ipv4Addr,
    pub neighbor_list: Vec<Neighbor>,
    pub int_output_cost: u32,
    pub rxmt_interval: u32,
    // AuType
    // AuKey
    pub pnet_interface: NetworkInterface,
    pub db_request_sender: mpsc::Sender<DatabaseRequest>,
    pub sdr_request_sender: mpsc::Sender<crate::sender::SenderRequest>,
    pub last_nbr_state: HashMap<Ipv4Addr, NbrState>,
}

#[derive(Debug, Clone)]
pub enum InterfaceType {
    P2P,
    Broadcast,
    NBMA,
    P2MP,
    Virtual,
}

#[derive(Debug, PartialEq, Clone)]
pub enum InterfaceState {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DR,
    BDR,
    DROther,
}

#[derive(Debug)]
pub enum InterfaceEvent {
    InterfaceUp,
    InterfaceDown,
    UnloopInd,
    LoopInd,
    WaitTimer,
    BackupSeen,
    NeighborChange,
}

impl Interface {
    pub fn from(
        nint: NetworkInterface,
        config: &Config,
        db_req_sender: mpsc::Sender<DatabaseRequest>,
        sdr_req_sender: mpsc::Sender<SenderRequest>,
    ) -> Self {
        let mut addr = Ipv4Addr::from(0);
        let mut mask = Ipv4Addr::from(0);
        for ip in nint.ips.as_slice() {
            if let IpNetwork::V4(ipv4) = ip {
                addr = ipv4.ip();
                mask = ipv4.mask();
                break;
            }
        }
        Interface {
            name: nint.name.clone(),
            int_type: InterfaceType::P2P,
            state: InterfaceState::Down,
            addr,
            mask,
            area_id: config.area_id,
            hello_interval: config.hello_interval,
            dead_interval: config.dead_interval,
            inf_transit_delay: config.inf_transit_delay,
            router_priority: config.router_priority,
            hello_timer: Timer::new(config.hello_interval),
            wait_timer: Timer::new(config.dead_interval),
            designated_router: Ipv4Addr::from(0),
            backup_designated_router: Ipv4Addr::from(0),
            neighbor_list: vec![],
            int_output_cost: 1,
            rxmt_interval: config.rxmt_interval,
            pnet_interface: nint,
            db_request_sender: db_req_sender,
            sdr_request_sender: sdr_req_sender,
            last_nbr_state: HashMap::new(),
        }
    }

    pub fn reset_and_close(&mut self) {
        // reset all variables, close timer, send KellNbr event to neighbors
        self.hello_timer.stop();
        self.wait_timer.stop();
        for nbr in self.neighbor_list.iter_mut() {
            nbr.handle_event(crate::neighbor::NbrEvent::KillNbr);
        }
        self.neighbor_list.clear();
        self.last_nbr_state.clear();
    }

    pub async fn check_timers(&mut self) {
        if self.hello_timer.is_expired() {
            self.send_hello().await;
            self.hello_timer.start();
        }
        if self.wait_timer.is_up() && self.wait_timer.is_expired() {
            self.handle_event(InterfaceEvent::WaitTimer).await;
            self.wait_timer.stop();
        }
        for neighbor in self.neighbor_list.iter_mut() {
            neighbor.check_timers().await;
        }
    }

    pub async fn check_nbr_change(&mut self) -> bool {
        // first, check if need to gen router lsa
        let mut need_gen_router_lsa = false;
        for (ip, state) in self.last_nbr_state.iter() {
            if let Some(nbr) = self.get_neighbor_index(*ip) {
                if self.neighbor_list[nbr].state == NbrState::Full && state != &NbrState::Full {
                    need_gen_router_lsa = true;
                    break;
                }
            }
        }
        if need_gen_router_lsa {
            self.query_gen_router_lsa().await;
            // if need to gen Router LSA && i am dr, gen Network LSA
            if self.state == InterfaceState::DR {
                assert!(self.designated_router == self.addr);
                self.query_gen_network_lsa().await;
            }
        }
        // then, check for neighbor 2-way state change
        let mut old_nbr_live_num = 0;
        // look in self.last_nbr_state
        for (ip, state) in self.last_nbr_state.iter() {
            // if ip in self.neighbor_list
            if let Some(nbr) = self.get_neighbor_index(*ip) {
                old_nbr_live_num += 1;
                if state.two_way_comm_status_changed(self.neighbor_list[nbr].state) {
                    // println!("Neighbor {} state changed from {:?} to {:?}",
                    //     ip, state, self.neighbor_list[nbr].state);
                    return true;
                }
            } else {
                // println!("Neighbor {} not found in interface {}", ip, self.name);
                return true;
            }
        }
        // println!("Old neighbor live number: {}", old_nbr_live_num);
        // println!("Current neighbor live number: {}", self.neighbor_list.len());
        old_nbr_live_num != self.neighbor_list.len()
    }

    pub async fn update_nbr_states(&mut self) {
        self.last_nbr_state.clear();
        for nbr in self.neighbor_list.iter() {
            self.last_nbr_state.insert(nbr.nbr_ip, nbr.state);
        }
    }

    pub fn get_neighbor_index(&self, ip: Ipv4Addr) -> Option<usize> {
        for (i, n) in self.neighbor_list.iter().enumerate() {
            if n.nbr_ip == ip {
                return Some(i);
            }
        }
        None
    }

    pub async fn handle_event(&mut self, event: InterfaceEvent) {
        let before = self.state.clone();
        println!("Interface {} received event {:?}", self.name, event);
        match event {
            InterfaceEvent::InterfaceUp => {
                assert!(self.state == InterfaceState::Down);
                self.state = InterfaceState::Waiting;
                self.wait_timer.start();
                // start hello timer, send hello packets
                self.hello_timer.start_imm();
                // if connect to P2P, P2MP, or virtual link, change to PointToPoint
                // else, if can't become DR or BDR, change to DROther
                // if can be DR && into Broadcast or NBMA, change to Waiting
                self.query_gen_router_lsa().await;
            }
            InterfaceEvent::InterfaceDown => {
                self.state = InterfaceState::Down;
                self.hello_timer.stop();
                self.wait_timer.stop();
                // reset all variables, close timer, send KellNbr event to neighbors
                self.reset_and_close();
                self.query_gen_router_lsa().await;
            }
            InterfaceEvent::UnloopInd => {
                assert!(self.state == InterfaceState::Loopback);
                self.state = InterfaceState::Down;
            }
            InterfaceEvent::LoopInd => {
                self.state = InterfaceState::Loopback;
                // reset all variables, close timer, send KellNbr event to neighbors
                self.reset_and_close();
            }
            InterfaceEvent::WaitTimer => {
                // the wait state before electing DR/BDR is done
                // elect DR/BDR && change to DR/BDR/DRother
                self.elect_dr_bdr().await;
                self.query_gen_router_lsa().await;
            }
            InterfaceEvent::BackupSeen => {
                // Found a backup designated router
                // elect DR/BDR && change to DR/BDR/DRother
                self.elect_dr_bdr().await;
                self.query_gen_router_lsa().await;
            }
            InterfaceEvent::NeighborChange => {
                // A neighbor has changed state, need to re-elect DR/BDR
                // elect DR/BDR && change to DR/BDR/DRother
                self.elect_dr_bdr().await;
                self.query_gen_router_lsa().await;
            }
        }
        // if a state change occurred, send out a RouterLSA
        if before != self.state {
            println!(
                "Interface {} state changed from {:?} to {:?}",
                self.name, before, self.state
            );
            // send out a RouterLSA
        }
    }

    fn elect_once_bdr<'a>(
        &'a self,
        candidates: &'a Vec<Neighbor>,
        need_declare: bool,
    ) -> Option<Ipv4Addr> {
        let mut bdr: Option<&Neighbor> = None;
        for nbr in candidates.iter() {
            // only those who not declare as DR can be BDR
            if nbr.nbr_ip == nbr.nbr_dr {
                continue;
            }
            // if someone declare to be bdr, choose the highest pri
            // or if no need for declare, then just choose one
            if nbr.nbr_ip == nbr.nbr_bdr || !need_declare {
                if bdr.is_none() {
                    bdr = Some(nbr);
                } else if nbr.nbr_pri > bdr.unwrap().nbr_pri
                    || (nbr.nbr_pri == bdr.unwrap().nbr_pri && nbr.nbr_id > bdr.unwrap().nbr_id)
                {
                    bdr = Some(nbr);
                }
            }
        }
        if bdr.is_some() {
            Some(bdr.unwrap().nbr_ip)
        } else {
            None
        }
    }

    fn elect_once_dr<'a>(&'a self, candidates: &'a Vec<Neighbor>) -> Option<Ipv4Addr> {
        let mut dr: Option<&Neighbor> = None;
        for nbr in candidates.iter() {
            // if someone declare to be dr, choose the highest pri
            // or if no need for declare, then just choose one
            if nbr.nbr_ip == nbr.nbr_dr {
                if dr.is_none() {
                    dr = Some(nbr);
                } else if nbr.nbr_pri > dr.unwrap().nbr_pri
                    || (nbr.nbr_pri == dr.unwrap().nbr_pri && nbr.nbr_id > dr.unwrap().nbr_id)
                {
                    dr = Some(nbr);
                }
            }
        }
        if dr.is_some() {
            Some(dr.unwrap().nbr_ip)
        } else {
            None
        }
    }

    async fn elect_dr_bdr(&mut self) {
        let mut candidates = vec![];
        for nbr in self.neighbor_list.iter() {
            // only those whose state not lower than 2-way can be candidates
            if nbr.state.higher_than_two_way() || nbr.state == NbrState::TwoWay {
                candidates.push(nbr.clone());
            }
        }
        let mut self_as_candidate = Neighbor::default();
        self_as_candidate.nbr_ip = self.addr;
        self_as_candidate.nbr_id = ipv4_to_bits(self.addr);
        self_as_candidate.nbr_pri = self.router_priority;
        self_as_candidate.nbr_dr = self.designated_router;
        self_as_candidate.nbr_bdr = self.backup_designated_router;
        candidates.push(self_as_candidate);
        let self_index = candidates.len() - 1;
        // 1. set the prev dr & bdr
        let prev_dr = self.designated_router;
        let prev_bdr = self.backup_designated_router;
        // 2. elect BDR
        let mut bdr;
        if let Some(tmp_bdr) = self.elect_once_bdr(&candidates, true) {
            bdr = Some(tmp_bdr);
        } else {
            // if no one declare to be bdr, choose one with no declaring dr
            bdr = self.elect_once_bdr(&candidates, false);
        }
        // 3. elect DR
        let mut dr;
        if let Some(tmp_dr) = self.elect_once_dr(&candidates) {
            dr = Some(tmp_dr);
        } else {
            // if no one declare to be dr, dr = bdr
            dr = bdr;
        }
        // 4. check some conditions
        let mut tmp_candidates = candidates.clone();
        // if no bdr, self try to be bdr
        if bdr.is_none() {
            if dr.unwrap() != self.addr {
                bdr = Some(self.addr);
            } else {
                tmp_candidates[self_index].nbr_dr = Ipv4Addr::from(0);
                let tmp_dr = self.elect_once_dr(&tmp_candidates);
                if tmp_dr.is_some() {
                    bdr = Some(self.addr);
                    dr = tmp_dr;
                }
            }
        }
        // avoid self be both dr and bdr
        if prev_dr != dr.unwrap() && self.addr == dr.unwrap() {
            // newly become dr, then recauculate bdr
            tmp_candidates[self_index].nbr_dr = self.addr;
            bdr = self.elect_once_bdr(&tmp_candidates, true);
            // else if newly become bdr, no need to recauculate dr
        }
        // 5. set interface state
        if dr.is_some() && self.addr == dr.unwrap() {
            self.state = InterfaceState::DR;
        } else if bdr.is_some() && self.addr == bdr.unwrap() {
            self.state = InterfaceState::BDR;
        } else {
            self.state = InterfaceState::DROther;
        }
        // 6. if dr or bdr changed, send AdjOK event to neighbors
        if dr.unwrap() != prev_dr || (bdr.is_some() && bdr.unwrap() != prev_bdr) {
            self.designated_router = dr.unwrap();
            // if i am dr, gen Network LSA
            if self.addr == dr.unwrap() {
                self.query_gen_network_lsa().await;
            }
            self.backup_designated_router = bdr.unwrap_or(Ipv4Addr::from(0));
            println!(
                "Interface {} re-elected DR: {:?}, BDR: {:?}",
                self.name, dr, bdr
            );
            for nbr in self.neighbor_list.iter_mut() {
                nbr.handle_event(crate::neighbor::NbrEvent::AdjOK);
            }
        }
    }
}
