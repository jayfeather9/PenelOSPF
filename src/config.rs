use std::net::Ipv4Addr;

pub const LS_REFRESH_TIME: u32 = 1800;
pub const MIN_LS_INTERVAL: u32 = 5;
pub const MIN_LS_ARRIVAL: u32 = 1;
pub const MAX_AGE: u32 = 3600;
pub const MAX_AGE_DIFF: u32 = 900;
pub const LS_INFINITY: u32 = 0xFF_FFFF;
pub const INIT_SEQ_NUM_LSA: i32 = 0x8000_0001u32 as i32;
pub const MAX_SEQ_NUM_LSA: i32 = 0x7FFF_FFFF;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub hello_interval: u32,
    pub dead_interval: u32,
    pub inf_transit_delay: u32,
    pub rxmt_interval: u32,
    pub router_id: u32,
    pub area_id: u32,
    pub network_mask: u32,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub default_mtu: u16,
    pub options: u8,
}

impl Config {
    pub fn new() -> Self {
        Config {
            hello_interval: 10,
            dead_interval: 40,
            inf_transit_delay: 1,
            rxmt_interval: 5,
            router_id: 3232240450,
            area_id: 0,
            network_mask: 0xFFFFF000, // 255.255.255.0
            router_priority: 1,
            router_dead_interval: 40,
            default_mtu: 1500,
            options: 0x02,
        }
    }
}
