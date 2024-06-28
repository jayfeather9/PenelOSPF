use crate::lsa::LsaHeader;
use crate::packets::{DBDescription, Hello};
use crate::timer::Timer;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct Neighbor {
    pub state: NbrState,
    pub inactivity_timer: Timer,
    pub exstart_rxmt_timer: Timer,
    pub mst_exch_timer: Timer,
    pub lsr_rxmt_timer: Timer,
    pub nbr_is_master: bool,
    pub dd_seq_number: u32,
    pub last_rcv_dbd: Option<DBDescription>,
    pub last_sent_dbd: Option<(u8, u32, Vec<LsaHeader>)>,
    pub nbr_id: u32,
    pub nbr_pri: u8,
    pub nbr_ip: Ipv4Addr,
    pub nbr_options: u8,
    pub nbr_dr: Ipv4Addr,
    pub nbr_bdr: Ipv4Addr,
    pub lsa_retransmission_list: Vec<LsaHeader>,
    pub db_summary_list: Vec<LsaHeader>,
    pub link_state_req_list: Vec<LsaHeader>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NbrState {
    Down,
    Attempt,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
}

impl NbrState {
    pub fn lower_than_init(&self) -> bool {
        match self {
            NbrState::Down | NbrState::Attempt => true,
            _ => false,
        }
    }

    pub fn higher_than_two_way(&self) -> bool {
        match self {
            NbrState::ExStart | NbrState::Exchange | NbrState::Loading | NbrState::Full => true,
            _ => false,
        }
    }

    pub fn have_two_way_comm(&self) -> bool {
        match self {
            NbrState::Down | NbrState::Attempt | NbrState::Init => false,
            _ => true,
        }
    }

    pub fn two_way_comm_status_changed(&self, new_state: NbrState) -> bool {
        self.have_two_way_comm() != new_state.have_two_way_comm()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NbrEvent {
    Start,
    HelloReceived,
    OneWayReceived,
    TwoWayReceived,
    NegotiationDone,
    ExchangeDone,
    BadLSReq,
    LoadingDone,
    AdjOK,
    SeqNumberMismatch,
    SeqNumberMatch,
    KillNbr,
    InactivityTimer,
    LLDown,
}

impl Neighbor {
    pub fn new(hello_packet: Hello, inactivity_interval: u32, rxmt_interval: u32) -> Neighbor {
        Neighbor {
            state: NbrState::Down,
            inactivity_timer: Timer::new(inactivity_interval),
            exstart_rxmt_timer: Timer::new(rxmt_interval),
            mst_exch_timer: Timer::new(rxmt_interval),
            lsr_rxmt_timer: Timer::new(rxmt_interval),
            nbr_is_master: false,
            dd_seq_number: 0,
            last_rcv_dbd: None,
            last_sent_dbd: None,
            nbr_id: 0,
            nbr_pri: hello_packet.router_priority,
            nbr_ip: Ipv4Addr::from(hello_packet.header.router_id),
            nbr_options: hello_packet.options,
            nbr_dr: Ipv4Addr::from(hello_packet.designated_router),
            nbr_bdr: Ipv4Addr::from(hello_packet.backup_designated_router),
            lsa_retransmission_list: Vec::new(),
            db_summary_list: Vec::new(),
            link_state_req_list: Vec::new(),
        }
    }

    pub fn default() -> Self {
        Neighbor {
            state: NbrState::Down,
            inactivity_timer: Timer::new(0),
            exstart_rxmt_timer: Timer::new(0),
            mst_exch_timer: Timer::new(0),
            lsr_rxmt_timer: Timer::new(0),
            nbr_is_master: false,
            dd_seq_number: 0,
            last_rcv_dbd: None,
            last_sent_dbd: None,
            nbr_id: 0,
            nbr_pri: 0,
            nbr_ip: Ipv4Addr::new(0, 0, 0, 0),
            nbr_options: 0,
            nbr_dr: Ipv4Addr::new(0, 0, 0, 0),
            nbr_bdr: Ipv4Addr::new(0, 0, 0, 0),
            lsa_retransmission_list: Vec::new(),
            db_summary_list: Vec::new(),
            link_state_req_list: Vec::new(),
        }
    }

    pub async fn check_timers(&mut self) {
        if self.inactivity_timer.is_up() && self.inactivity_timer.is_expired() {
            self.handle_event(NbrEvent::InactivityTimer);
            self.inactivity_timer.stop();
        }
    }

    fn decide_adjacency(&mut self) -> bool {
        // TODO
        true
    }

    fn clear_lsa_lists(&mut self) {
        self.lsa_retransmission_list.clear();
        self.db_summary_list.clear();
        self.link_state_req_list.clear();
    }

    fn begin_exstart(&mut self) {
        self.state = NbrState::ExStart;
        // set dd_seq_number to some unique number like time elapsed
        self.dd_seq_number = self.inactivity_timer.elapsed() as u32;
        // if interface see this nbr in ExStart state, start send DBD packets
        self.exstart_rxmt_timer.start_imm();
    }

    pub fn handle_event(&mut self, event: NbrEvent) {
        let before = self.state;
        if event != NbrEvent::HelloReceived && event != NbrEvent::TwoWayReceived {
            println!("Neighbor {} received event {:?}", self.nbr_ip, event);
        }
        match event {
            NbrEvent::Start => {
                assert!(self.state == NbrState::Down);
                self.state = NbrState::Attempt;
                // start to send hello packets and activate inactivity timer
                self.inactivity_timer.start();
            }
            NbrEvent::HelloReceived => {
                if self.state.lower_than_init() {
                    self.state = NbrState::Init;
                }
                // reset or start inactivity timer
                self.inactivity_timer.start();
            }
            NbrEvent::OneWayReceived => {
                self.state = NbrState::Init;
                // clear 3 list of LSA
                self.clear_lsa_lists();
            }
            NbrEvent::TwoWayReceived => {
                // If state if higher than 2-Way, do nothing
                if self.state.higher_than_two_way() {
                    return;
                }
                // decide if we need to build adjacency relationship
                if self.decide_adjacency() {
                    // if yes, state is ExStart, and send DBD packets
                    self.begin_exstart();
                } else {
                    // if not, state is TwoWay
                    self.state = NbrState::TwoWay;
                }
            }
            NbrEvent::NegotiationDone => {
                assert!(self.state == NbrState::ExStart);
                self.state = NbrState::Exchange;
            }
            NbrEvent::ExchangeDone => {
                assert!(self.state == NbrState::Exchange);
                // if connect request list is empty, state is Full
                // else, state is Loading, and send LSR packets
                if self.link_state_req_list.is_empty() {
                    self.state = NbrState::Full;
                } else {
                    self.state = NbrState::Loading;
                    // if interface see this nbr in Loading state, start send LSR packets
                }
            }
            NbrEvent::LoadingDone => {
                assert!(self.state == NbrState::Loading);
                self.state = NbrState::Full;
            }
            NbrEvent::AdjOK => {
                // decide if we need to build adjacency relationship
                if self.state == NbrState::TwoWay {
                    if self.decide_adjacency() {
                        // if yes, state is ExStart, and send DBD packets
                        self.begin_exstart();
                    } else {
                        // if not, state is TwoWay
                        self.state = NbrState::TwoWay;
                    }
                } else {
                    if self.decide_adjacency() {
                        // maintain and do nothing
                    } else {
                        // if not, state is TwoWay and clear 3 list of LSA
                        self.state = NbrState::TwoWay;
                        self.clear_lsa_lists();
                    }
                }
            }
            NbrEvent::SeqNumberMismatch | NbrEvent::BadLSReq => {
                self.clear_lsa_lists();
                self.begin_exstart();
            }
            NbrEvent::SeqNumberMatch => {
                self.state = NbrState::Exchange;
            }
            NbrEvent::KillNbr => {
                self.state = NbrState::Down;
                // clear 3 list of LSA
                self.clear_lsa_lists();
            }
            NbrEvent::InactivityTimer => {
                self.state = NbrState::Down;
                // clear 3 list of LSA
                self.clear_lsa_lists();
            }
            NbrEvent::LLDown => {
                self.state = NbrState::Down;
                // clear 3 list of LSA
                self.clear_lsa_lists();
            }
        }
        if before != self.state {
            println!(
                "Neighbor {} state changed from {:?} to {:?}",
                self.nbr_ip, before, self.state
            );
        }
    }
}
