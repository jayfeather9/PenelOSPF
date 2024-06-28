use pnet::datalink::NetworkInterface;
use pnet::packet::ipv4::MutableIpv4Packet;
use std::net::Ipv4Addr;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::packets::OSPFPacket;

#[derive(Debug)]
pub struct OSPFPacketSender {
    pub request_channel: mpsc::Receiver<SenderRequest>,
}

#[derive(Debug)]
pub struct SenderRequest {
    pub request_type: SenderRequestType,
    pub reply_channel: oneshot::Sender<SenderResponse>,
}

#[derive(Debug)]
pub enum SenderRequestType {
    SendOSPFPacket(OSPFPacket, (Ipv4Addr, Ipv4Addr), NetworkInterface),
}

#[derive(Debug)]
pub enum SenderResponse {
    PacketSent,
    Failed,
}

const SENDER_BUFFER_SIZE: usize = 200;

impl OSPFPacketSender {
    pub async fn sender_thread(&mut self) {
        while let Some(req) = self.request_channel.recv().await {
            let reply = self.handle_request(req.request_type).await;
            if let Err(e) = req.reply_channel.send(reply) {
                eprintln!("Error sending reply: {:?}", e);
            }
        }
    }

    async fn handle_request(&mut self, req: SenderRequestType) -> SenderResponse {
        match req {
            SenderRequestType::SendOSPFPacket(mut packet, (src_ip, dst_ip), interface) => {
                packet.set_packet_length();
                packet.set_checksum();
                let encoded = packet.encode_bincode();
                assert!(encoded.len() < SENDER_BUFFER_SIZE);
                let mut buffer = [0u8; SENDER_BUFFER_SIZE];
                let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer).unwrap();
                ipv4_packet.set_version(4);
                ipv4_packet.set_header_length(5);
                ipv4_packet.set_total_length(encoded.len() as u16 + 20);
                ipv4_packet.set_ttl(1);
                ipv4_packet
                    .set_next_level_protocol(pnet::packet::ip::IpNextHeaderProtocols::OspfigP);
                ipv4_packet.set_source(src_ip);
                ipv4_packet.set_destination(dst_ip);
                ipv4_packet.set_checksum(pnet::packet::ipv4::checksum(&ipv4_packet.to_immutable()));
                ipv4_packet.set_payload(&encoded);

                let (mut tx, _) = pnet::transport::transport_channel(
                    1024,
                    pnet::transport::TransportChannelType::Layer3(
                        pnet::packet::ip::IpNextHeaderProtocols::Ipv4,
                    ),
                )
                .unwrap();
                println!(
                    "Sending ospf type {} packet to {:?}",
                    packet.get_hdr().packet_type,
                    dst_ip
                );

                match tx.send_to(ipv4_packet, std::net::IpAddr::V4(dst_ip)) {
                    Ok(_) => SenderResponse::PacketSent,
                    Err(_) => SenderResponse::Failed,
                }
            }
        }
    }
}
