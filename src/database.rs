use std::collections::HashMap;
use std::net::Ipv4Addr;

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::config::Config;
use crate::interface::Interface;
use crate::lsa::LsaNetwork;
use crate::lsa::LsaRouter;
use crate::lsa::LsaRouterLink;
use crate::lsa::LsaType;
use crate::lsa::{Lsa, LsaHeader};
use crate::receiver::ipv4_to_bits;
use crate::route::RouteTable;

// We use tokio channels for communication, see https://rust-book.junmajinlong.com/ch100/05_task_communication.html
#[derive(Debug)]
pub struct LinkStateDatabase {
    pub each_int_link: HashMap<Ipv4Addr, LsaRouterLink>,
    pub lsa_list: Vec<Lsa>,
    pub global_config: Config,
    pub request_channel: mpsc::Receiver<DatabaseRequest>,
    pub last_iter_instant: std::time::Instant,
    pub cur_lsa_seq_num: i32,
    pub route_table: RouteTable,
    pub int_list: Vec<Interface>,
}

impl LinkStateDatabase {
    pub fn from(
        config: Config,
        request_channel: mpsc::Receiver<DatabaseRequest>,
        int_list: Vec<Interface>,
    ) -> Self {
        LinkStateDatabase {
            each_int_link: HashMap::new(),
            lsa_list: vec![],
            global_config: config,
            request_channel,
            last_iter_instant: std::time::Instant::now(),
            cur_lsa_seq_num: crate::config::INIT_SEQ_NUM_LSA as i32,
            route_table: RouteTable::new(),
            int_list,
        }
    }
}

#[derive(Debug)]
pub struct DatabaseRequest {
    pub request_type: DatabaseRequestType,
    pub reply_channel: oneshot::Sender<DatabaseResponse>,
}

#[derive(Debug)]
pub enum DatabaseRequestType {
    QueryConfig,
    ChangeConfig(Config),
    QueryLsaByHdr(LsaHeader),
    QueryLsaByLSID(u32),
    QueryLsaByLSIDAdvRouter(u32, u32),
    QueryMultiLsa(Vec<(u32, u32)>),
    RemoveLsa(LsaHeader),
    AddOrUpdateLsa(Lsa),
    QueryAllLsa,
    QueryAllLsaByType(u8),
    GenRouterLsa(LsaRouterLink, Ipv4Addr),
    GenNetworkLsa(Ipv4Addr, Ipv4Addr, Vec<Ipv4Addr>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    UpdateDone,
    NotFound,
    Config(Config),
    Lsa(Lsa),
    LsaList(Vec<Lsa>),
}

impl LinkStateDatabase {
    pub async fn database_thread(&mut self) {
        while let Some(req) = self.request_channel.recv().await {
            let iter_duration = self.last_iter_instant.elapsed();
            self.last_iter_instant = std::time::Instant::now();
            self.lsa_aging(iter_duration).await;
            let reply = self.handle_request(req.request_type).await;
            if let Err(e) = req.reply_channel.send(reply) {
                eprintln!("Error sending reply: {:?}", e);
            }
        }
    }

    async fn lsa_aging(&mut self, iter_duration: std::time::Duration) {
        for lsa in self.lsa_list.iter_mut() {
            if (lsa.get_mut_hdr().age as u32 + iter_duration.as_secs() as u32)
                < crate::config::MAX_AGE
            {
                // TODO: if is self originated && > refresh age, refresh a new one
                lsa.get_mut_hdr().age += iter_duration.as_secs() as u16;
            } else {
                // TODO: remove LSA from routing table
            }
        }
    }

    async fn handle_request(&mut self, req: DatabaseRequestType) -> DatabaseResponse {
        let mut recalculate_needed = false;
        let response = match req {
            DatabaseRequestType::QueryConfig => {
                DatabaseResponse::Config(self.global_config.clone())
            }
            DatabaseRequestType::ChangeConfig(new_config) => {
                self.global_config = new_config;
                DatabaseResponse::UpdateDone
            }
            DatabaseRequestType::AddOrUpdateLsa(lsa) => {
                let lsa_index = self
                    .lsa_list
                    .iter()
                    .position(|x| x.same_ids(&lsa.get_hdr()));
                match lsa_index {
                    Some(i) => {
                        self.lsa_list[i] = lsa;
                    }
                    None => {
                        self.lsa_list.push(lsa);
                    }
                }
                recalculate_needed = true;
                DatabaseResponse::UpdateDone
            }
            DatabaseRequestType::QueryAllLsa => DatabaseResponse::LsaList(self.lsa_list.clone()),
            DatabaseRequestType::QueryAllLsaByType(ls_type) => {
                let lsa_list: Vec<Lsa> = self
                    .lsa_list
                    .iter()
                    .filter(|x| x.get_hdr().ls_type == ls_type)
                    .cloned()
                    .collect();
                DatabaseResponse::LsaList(lsa_list)
            }
            DatabaseRequestType::QueryLsaByHdr(hdr) => {
                let lsa = self.lsa_list.iter().find(|x| x.same_ids(&hdr)).cloned();
                match lsa {
                    Some(l) => DatabaseResponse::Lsa(l),
                    None => DatabaseResponse::NotFound,
                }
            }
            DatabaseRequestType::QueryLsaByLSID(lsid) => {
                let lsa = self
                    .lsa_list
                    .iter()
                    .find(|x| x.get_hdr().link_state_id == lsid)
                    .cloned();
                match lsa {
                    Some(l) => DatabaseResponse::Lsa(l),
                    None => DatabaseResponse::NotFound,
                }
            }
            DatabaseRequestType::QueryLsaByLSIDAdvRouter(lsid, adv_router) => {
                let lsa = self
                    .lsa_list
                    .iter()
                    .find(|x| {
                        x.get_hdr().link_state_id == lsid
                            && x.get_hdr().advertising_router == adv_router
                    })
                    .cloned();
                match lsa {
                    Some(l) => DatabaseResponse::Lsa(l),
                    None => DatabaseResponse::NotFound,
                }
            }
            DatabaseRequestType::QueryMultiLsa(queries) => {
                let mut lsa_list = vec![];
                for lsa in &self.lsa_list {
                    for (lsid, adv_router) in &queries {
                        if lsa.get_hdr().link_state_id == *lsid
                            && lsa.get_hdr().advertising_router == *adv_router
                        {
                            lsa_list.push(lsa.clone());
                        }
                    }
                }
                DatabaseResponse::LsaList(lsa_list)
            }
            DatabaseRequestType::RemoveLsa(hdr) => {
                let lsa_index = self.lsa_list.iter().position(|x| x.get_hdr() == &hdr);
                recalculate_needed = true;
                match lsa_index {
                    Some(i) => {
                        self.lsa_list.remove(i);
                        DatabaseResponse::UpdateDone
                    }
                    None => DatabaseResponse::NotFound,
                }
            }
            DatabaseRequestType::GenRouterLsa(link, int_addr) => {
                // change or add the link to the hashmap
                self.each_int_link.insert(int_addr, link);
                let lsa = self.make_router_lsa(self.each_int_link.values().cloned().collect());
                let old_index = self
                    .lsa_list
                    .iter()
                    .position(|x| x.same_ids(&lsa.get_hdr()));
                // clear the old one
                self.lsa_list.retain(|x| !x.same_ids(&lsa.get_hdr()));
                self.cur_lsa_seq_num += 1;
                self.lsa_list.push(lsa.clone());
                recalculate_needed = true;
                println!("Router LSA generated: {:?}", lsa);
                match old_index {
                    Some(_) => DatabaseResponse::Lsa(lsa),
                    None => DatabaseResponse::UpdateDone,
                }
            }
            DatabaseRequestType::GenNetworkLsa(int_addr, int_mask, neighbors) => {
                let lsa = self.make_network_lsa(int_addr, int_mask, neighbors);
                let old_index = self
                    .lsa_list
                    .iter()
                    .position(|x| x.same_ids(&lsa.get_hdr()));
                self.cur_lsa_seq_num += 1;
                self.lsa_list.push(lsa.clone());
                recalculate_needed = true;
                match old_index {
                    Some(_) => DatabaseResponse::Lsa(lsa),
                    None => DatabaseResponse::UpdateDone,
                }
            }
        };
        // TODO: check if any LSA change occurs, if so, recalculate routing
        if recalculate_needed {
            self.update_route_table();
        }
        response
    }

    fn make_router_lsa(&self, links: Vec<LsaRouterLink>) -> Lsa {
        let mut lsa = Lsa::LsaRouter(LsaRouter {
            header: LsaHeader {
                age: 0,
                options: 0x02,
                ls_type: LsaType::LsaRouter as u8,
                link_state_id: self.global_config.router_id,
                advertising_router: self.global_config.router_id,
                sequence_number: self.cur_lsa_seq_num as u32,
                checksum: 0,
                length: 0,
            },
            flags: 0,
            num_links: links.len() as u16,
            links,
        });
        lsa.set_checksum_length();
        lsa
    }

    fn make_network_lsa(
        &self,
        int_addr: Ipv4Addr,
        int_mask: Ipv4Addr,
        neighbors: Vec<Ipv4Addr>,
    ) -> Lsa {
        let mut lsa = Lsa::LsaNetwork(LsaNetwork {
            header: LsaHeader {
                age: 0,
                options: 0x02,
                ls_type: LsaType::LsaNetwork as u8,
                link_state_id: ipv4_to_bits(int_addr),
                advertising_router: self.global_config.router_id,
                sequence_number: self.cur_lsa_seq_num as u32,
                checksum: 0,
                length: 0,
            },
            network_mask: ipv4_to_bits(int_mask),
            attached_routers: neighbors.iter().map(|x| ipv4_to_bits(*x)).collect(),
        });
        lsa.set_checksum_length();
        lsa
    }

    pub fn get_network_lsa(&self, int_addr: u32) -> Option<LsaNetwork> {
        for lsa in self.lsa_list.iter() {
            if let Lsa::LsaNetwork(lsa) = lsa {
                // println!(
                //     "[get_network_lsa] lsa link_state_id: {}, int_addr: {}",
                //     lsa.header.link_state_id, int_addr
                // );
                if lsa.header.link_state_id == int_addr {
                    return Some(lsa.clone());
                }
            }
        }
        None
    }
}
