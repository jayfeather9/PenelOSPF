// hello.rs, deprecated now
use crate::config::Config;
use crate::packets::{Header, Hello, PacketType};
use pnet::datalink::{self, Channel::Ethernet};
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::{MutablePacket, Packet};
use std::net::Ipv4Addr;

pub async fn send_hello(interface_name: String, config: Config, src_ip: Ipv4Addr) {
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .expect("Interface not found");

    let (mut tx, _) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!(
            "An error occurred when creating the datalink channel: {}",
            e
        ),
    };

    let mac = interface.mac.unwrap();

    let mut buffer = [0u8; 200];
    let mut ethernet_packet = MutableEthernetPacket::new(&mut buffer).unwrap();
    ethernet_packet.set_destination(mac);
    ethernet_packet.set_source(mac);
    ethernet_packet.set_ethertype(EtherTypes::Ipv4);

    let mut ipv4_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut()).unwrap();
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(20);
    ipv4_packet.set_ttl(1);
    ipv4_packet.set_next_level_protocol(pnet::packet::ip::IpNextHeaderProtocols::OspfigP);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(Ipv4Addr::new(224, 0, 0, 5)); // OSPF multicast address

    let mut hello_packet = Hello {
        header: Header {
            version: 2,
            packet_type: PacketType::Hello as u8,
            packet_length: 24,
            router_id: config.router_id,
            area_id: config.area_id,
            checksum: 0, // Calculate checksum if needed
            auth_type: 0,
            auth: 0,
        },
        network_mask: config.network_mask,
        hello_interval: config.hello_interval as u16,
        options: 0,
        router_priority: config.router_priority,
        router_dead_interval: config.router_dead_interval,
        designated_router: ipv4_to_bits(config.designated_router),
        backup_designated_router: ipv4_to_bits(config.backup_designated_router),
        neighbors: vec![],
    };

    crate::packets::calc_packet_checksum(&mut hello_packet);
    let encoded = bincode::encode_to_vec(hello_packet, crate::config::BINCODE_CONF).unwrap();
    ipv4_packet.set_total_length(20 + encoded.len() as u16);
    ipv4_packet.set_payload(&encoded);

    // Send the packet
    let _ = tx.send_to(ethernet_packet.packet(), None).unwrap();
}
