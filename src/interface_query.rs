use crate::config::Config;
use crate::database::{DatabaseRequest, DatabaseRequestType, DatabaseResponse};
use crate::interface::{Interface, InterfaceState};
use crate::lsa::{LinkType, Lsa, LsaRouterLink};
use crate::neighbor::{NbrState, Neighbor};
use crate::packets::LinkStateRequest;
use crate::receiver::ipv4_to_bits;
use crate::sender::{SenderRequest, SenderRequestType, SenderResponse};

use tokio::sync::oneshot;

impl Interface {
    pub async fn query_database(&self, req: DatabaseRequestType) -> DatabaseResponse {
        let (rpl_tx, rpl_rx) = oneshot::channel();
        let req = DatabaseRequest {
            request_type: req,
            reply_channel: rpl_tx,
        };
        if self.db_request_sender.send(req).await.is_err() {
            panic!("Error sending request to database");
        }
        match rpl_rx.await {
            Ok(r) => r,
            _ => panic!("Error getting response from database"),
        }
    }

    pub async fn query_sender(&self, req: SenderRequestType) -> SenderResponse {
        let (rpl_tx, rpl_rx) = oneshot::channel();
        let req = SenderRequest {
            request_type: req,
            reply_channel: rpl_tx,
        };
        if self.sdr_request_sender.send(req).await.is_err() {
            panic!("Error sending request to sender");
        }
        match rpl_rx.await {
            Ok(r) => r,
            _ => panic!("Error getting response from sender"),
        }
    }

    pub async fn query_config(&self) -> Config {
        match self.query_database(DatabaseRequestType::QueryConfig).await {
            DatabaseResponse::Config(c) => c,
            _ => panic!("Error getting config from database"),
        }
    }

    pub async fn query_by_lsa_hdr(&self, hdr: crate::lsa::LsaHeader) -> Option<Lsa> {
        match self
            .query_database(DatabaseRequestType::QueryLsaByHdr(hdr))
            .await
        {
            DatabaseResponse::Lsa(l) => Some(l),
            DatabaseResponse::NotFound => None,
            _ => panic!("Error getting LSA from database"),
        }
    }

    pub async fn query_multi_lsa(&self, lsr: LinkStateRequest) -> Option<Vec<Lsa>> {
        let query_list: Vec<(u32, u32)> = lsr
            .requests
            .iter()
            .map(|x| (x.link_state_id, x.advertising_router))
            .collect();
        let res_list = match self
            .query_database(DatabaseRequestType::QueryMultiLsa(query_list))
            .await
        {
            DatabaseResponse::LsaList(l) => l,
            _ => panic!("Error getting LSA list from database"),
        };
        assert!(res_list.len() <= lsr.requests.len());
        if res_list.len() == lsr.requests.len() {
            Some(res_list)
        } else {
            None
        }
    }

    pub async fn query_gen_router_lsa(&self) {
        assert!(self.state != InterfaceState::Down);
        let mut link = LsaRouterLink::new(self.int_output_cost as u16);

        if (self.state != InterfaceState::Waiting)
            && (self.designated_router == self.addr
                || self.neighbor_list[self.get_neighbor_index(self.designated_router).unwrap()]
                    .state
                    == NbrState::Full)
        {
            link.link_type = LinkType::Transit as u8;
            link.link_id = ipv4_to_bits(self.designated_router);
            link.link_data = ipv4_to_bits(self.addr);
        } else {
            link.link_type = LinkType::Stub as u8;
            link.link_id = ipv4_to_bits(self.addr) & ipv4_to_bits(self.mask);
            link.link_data = ipv4_to_bits(self.mask);
        };
        match self
            .query_database(DatabaseRequestType::GenRouterLsa(link, self.addr))
            .await
        {
            DatabaseResponse::Lsa(lsa) => self.flood_lsu(vec![lsa]).await,
            DatabaseResponse::UpdateDone => {}
            _ => panic!("Error getting general router LSA from database"),
        };
    }

    pub async fn query_gen_network_lsa(&self) {
        let nbr_ip_list: Vec<std::net::Ipv4Addr> = self
            .neighbor_list
            .iter()
            .filter(|x| x.state == NbrState::Full)
            .map(|x| x.nbr_ip)
            .collect();
        if nbr_ip_list.len() == 0 {
            return;
        }
        match self
            .query_database(DatabaseRequestType::GenNetworkLsa(
                self.addr,
                self.mask,
                nbr_ip_list,
            ))
            .await
        {
            DatabaseResponse::Lsa(lsa) => self.flood_lsu(vec![lsa]).await,
            DatabaseResponse::UpdateDone => {}
            _ => panic!("Error getting general network LSA from database"),
        };
    }
}
