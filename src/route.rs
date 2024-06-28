use crate::database::LinkStateDatabase;
use crate::lsa::{LinkType, Lsa, LsaNetwork, LsaRouter};
use crate::route;
use std::collections::HashMap;
use std::env::current_exe;
use std::net::Ipv4Addr;

#[derive(Debug)]
pub struct RouteEntry {
    pub dest_id: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub next_hop: Ipv4Addr,
    pub metric: u32,
    pub int_addr: Ipv4Addr,
}

#[derive(Debug)]
pub struct RouteTable {
    pub entries: Vec<RouteEntry>,
    nodes: HashMap<Ipv4Addr, Node>,
    prevs: HashMap<Ipv4Addr, Ipv4Addr>,
    edges: HashMap<Ipv4Addr, Vec<Edge>>,
}

#[derive(Debug, PartialEq, Clone)]
struct Node {
    pub id: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub dis: u32,
}

impl std::cmp::PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.dis.cmp(&other.dis))
    }
}

#[derive(Debug)]
pub struct Edge {
    pub dst: Ipv4Addr,
    pub dis: u32,
}

impl RouteTable {
    pub fn new() -> Self {
        RouteTable {
            entries: vec![],
            nodes: HashMap::new(),
            prevs: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn clear_graph(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.prevs.clear();
    }

    pub fn put_or_update_edge(&mut self, src: Ipv4Addr, dst: Ipv4Addr, dis: u32) {
        // println!("put_or_update_edge: {} => {} ({})", src, dst, dis);
        self.edges
            .entry(src)
            .or_insert(vec![])
            .push(Edge { dst, dis });
    }
}

impl LinkStateDatabase {
    pub fn update_route_table(&mut self) {
        // println!("Updating route table, current lsa list:");
        // for lsa in self.lsa_list.iter() {
        //     println!("{:?}", lsa);
        // }
        self.route_table.clear_graph();
        // kinda ugly, but for passing the borrow checker
        for lsa in self.lsa_list.clone().iter().cloned() {
            match lsa {
                Lsa::LsaRouter(l) => {
                    self.update_route_table_manage_router_lsa(l);
                }
                Lsa::LsaNetwork(l) => {
                    self.update_route_table_manage_network_lsa(l);
                }
                _ => {}
            }
        }

        self.run_dijkstra();

        for lsa in self.lsa_list.iter() {
            match lsa {
                Lsa::LsaSumnet(l) => {
                    // self-originated
                    if l.header.advertising_router == self.global_config.router_id {
                        continue;
                    }
                    let adv_rtr = self
                        .route_table
                        .nodes
                        .get(&Ipv4Addr::from(l.header.advertising_router));
                    // if the advertising router is not in the route table, skip
                    if adv_rtr.is_none() {
                        continue;
                    }
                    let adv_rtr = adv_rtr.unwrap().clone();
                    // if the advertising router is not reachable, skip
                    if adv_rtr.dis == u32::MAX {
                        continue;
                    }
                    self.route_table.nodes.insert(
                        Ipv4Addr::from(l.header.link_state_id),
                        Node {
                            id: Ipv4Addr::from(l.header.link_state_id),
                            mask: Ipv4Addr::from(l.network_mask),
                            dis: adv_rtr.dis,
                        },
                    );
                    self.route_table.put_or_update_edge(
                        adv_rtr.id,
                        Ipv4Addr::from(l.header.link_state_id),
                        l.metric as u32,
                    );
                    self.route_table
                        .prevs
                        .insert(Ipv4Addr::from(l.header.link_state_id), adv_rtr.id);
                }
                _ => {}
            }
        }
        // TODO: construct external routes

        // construct route table
        self.route_table.entries.clear();
        for node in self.route_table.nodes.values() {
            if node.dis == u32::MAX || node.id == Ipv4Addr::from(self.global_config.router_id) {
                // println!(
                //     "[PASSED] node: {:?}, mask: {:?}, dis: {}",
                //     node.id, node.mask, node.dis
                // );
                continue;
            }
            let mut cur_node = node.id;
            let mut next_hop = Ipv4Addr::from(0);
            while cur_node != Ipv4Addr::from(self.global_config.router_id) {
                let prev_node = self.route_table.prevs.get(&cur_node).unwrap();
                next_hop = cur_node;
                // println!("cur_node: {:?}, prev_node: {:?}", cur_node, prev_node);
                cur_node = *prev_node;
            }
            // println!("node: {:?}, next_hop: {:?}", node, next_hop);
            // get interface address
            let int_addr = self
                .int_list
                .iter()
                .find(|x| x.addr == cur_node || x.addr & node.mask == node.id)
                .unwrap()
                .addr;
            self.route_table.entries.push(RouteEntry {
                dest_id: node.id,
                mask: node.mask,
                next_hop,
                metric: node.dis,
                int_addr,
            });
        }

        println!("route table: {:?}", self.route_table.entries);

        println!("Route table updated");
    }

    fn update_route_table_manage_router_lsa(&mut self, lsa: LsaRouter) {
        self.route_table.nodes.insert(
            Ipv4Addr::from(lsa.header.link_state_id),
            Node {
                id: Ipv4Addr::from(lsa.header.link_state_id),
                mask: Ipv4Addr::from(0),
                dis: u32::MAX,
            },
        );
        // println!("Router LSA: {:?}", lsa);
        for link in lsa.links.iter() {
            if link.link_type == LinkType::P2P as u8 {
                self.route_table.put_or_update_edge(
                    Ipv4Addr::from(lsa.header.link_state_id),
                    Ipv4Addr::from(link.link_id),
                    link.metric as u32,
                );
            } else if link.link_type == LinkType::Transit as u8 {
                // println!("Transit link: {:?}", link);
                if self.get_network_lsa(link.link_id).is_none() {
                    println!("!!!Not Found");
                    for nlsa in self.lsa_list.iter() {
                        match nlsa {
                            Lsa::LsaNetwork(n) => {
                                println!(
                                    "Itering etwork lsa, ls_id: {}, adv_rtr: {}",
                                    Ipv4Addr::from(n.header.link_state_id),
                                    Ipv4Addr::from(n.header.advertising_router)
                                );
                            }
                            _ => {}
                        }
                    }
                    continue;
                }

                let nlsa = self.get_network_lsa(link.link_id).unwrap();
                for rtr_id in nlsa.attached_routers {
                    if rtr_id == lsa.header.link_state_id {
                        continue;
                    }
                    self.route_table.put_or_update_edge(
                        Ipv4Addr::from(lsa.header.link_state_id),
                        Ipv4Addr::from(rtr_id),
                        link.metric as u32,
                    );
                }
            } else if link.link_type == LinkType::Stub as u8 {
                self.route_table.nodes.insert(
                    Ipv4Addr::from(link.link_id),
                    Node {
                        id: Ipv4Addr::from(link.link_id),
                        mask: Ipv4Addr::from(link.link_data),
                        dis: u32::MAX,
                    },
                );
                self.route_table.put_or_update_edge(
                    Ipv4Addr::from(lsa.header.link_state_id),
                    Ipv4Addr::from(link.link_id),
                    link.metric as u32,
                );
            } else {
                unimplemented!();
            }
        }
    }

    fn update_route_table_manage_network_lsa(&mut self, lsa: LsaNetwork) {
        let net_node_id = Ipv4Addr::from(lsa.header.link_state_id & lsa.network_mask);
        self.route_table.nodes.insert(
            net_node_id,
            Node {
                id: net_node_id,
                mask: Ipv4Addr::from(lsa.network_mask),
                dis: u32::MAX,
            },
        );
        for rtr_id in lsa.attached_routers {
            self.route_table
                .put_or_update_edge(Ipv4Addr::from(rtr_id), net_node_id, 0);
        }
    }

    fn run_dijkstra(&mut self) {
        // for ed in self.route_table.edges.iter() {
        //     println!("{} => {:?}", ed.0, ed.1);
        // }
        self.route_table
            .nodes
            .get_mut(&Ipv4Addr::from(self.global_config.router_id))
            .unwrap()
            .dis = 0;
        let mut q: Vec<Node> = self.route_table.nodes.values().cloned().collect();
        for node in q.iter() {
            self.route_table.prevs.insert(node.id, Ipv4Addr::from(0));
        }
        // println!("q: {:?}", q);

        while !q.is_empty() {
            q.sort_by(|a, b| a.dis.cmp(&b.dis));
            // println!("u: {:?}", q[0]);
            if q[0].dis == u32::MAX {
                break;
            }
            let u = q.remove(0).id;
            for edge in self.route_table.edges.get(&u).unwrap_or(&vec![]) {
                // println!("edge: {:?}", edge);
                // println!("v: {:?}", self.route_table.nodes.get(&edge.dst).unwrap());
                let v = self.route_table.nodes.get(&edge.dst).unwrap().id;

                let alt = self.route_table.nodes.get(&u).unwrap().dis + edge.dis;
                // println!(
                //     "alt: {}, v.dis: {}",
                //     alt,
                //     self.route_table.nodes.get(&v).unwrap().dis
                // );
                if alt < self.route_table.nodes.get(&v).unwrap().dis {
                    self.route_table.nodes.get_mut(&v).unwrap().dis = alt;
                    // change the dis in "q"
                    for node in q.iter_mut() {
                        if node.id == v {
                            node.dis = alt;
                            break;
                        }
                    }
                    self.route_table.prevs.insert(v, u);
                    println!("{} => {}", u, v);
                }
            }
        }
        println!("Dijkstra finished");
    }
}

// #[derive(Debug)]
// pub struct Route {
//     pub dest_type: DestType,
//     pub dest_id: Ipv4Addr,
//     pub addr_mask: Ipv4Addr,
//     pub area_id: u32,
//     pub path_type: PathType,
//     pub cost: u32,
//     pub type2_metric: u32,
//     pub link_state_origin: Lsa,
//     pub next_hop: Ipv4Addr,
//     pub advertising_router: Ipv4Addr,
// }

// #[derive(Debug)]
// pub enum DestType {
//     Network,
//     Router,
// }

// #[derive(Debug)]
// pub enum PathType {
//     IntraArea,
//     InterArea,
//     Type1Ext,
//     Type2Ext,
// }

// #[derive(Debug)]
// pub struct RouteTable {
//     pub routes: Vec<Route>,
// }
