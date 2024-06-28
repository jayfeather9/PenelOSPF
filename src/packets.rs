use crate::lsa::*;
use encoding_derive::Encoding;
use nom::combinator::peek;
use nom::number::streaming::be_u16;
use nom_derive::*;

#[derive(Debug, Clone)]
pub enum OSPFPacket {
    Hello(Hello),
    DBDescription(DBDescription),
    LinkStateRequest(LinkStateRequest),
    LinkStateUpdate(LinkStateUpdate),
    LinkStateAcknowledgment(LinkStateAcknowledgment),
}

impl<'a> Parse<&'a [u8]> for OSPFPacket {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (_, wd) = peek(be_u16)(input)?;
        let b0 = (wd >> 8) as u8;
        let b1 = (wd & 0xff) as u8;
        if b0 != 2 {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        // println!("Parsing OSPF packet type: {:?} - {:?}", b1, input);
        match b1 {
            1 => {
                let (input, hello) = Hello::parse(input)?;
                Ok((input, OSPFPacket::Hello(hello)))
            }
            2 => {
                let (input, dbd) = DBDescription::parse(input)?;
                Ok((input, OSPFPacket::DBDescription(dbd)))
            }
            3 => {
                let (input, lsr) = LinkStateRequest::parse(input)?;
                Ok((input, OSPFPacket::LinkStateRequest(lsr)))
            }
            4 => {
                let (input, lsu) = LinkStateUpdate::parse(input)?;
                Ok((input, OSPFPacket::LinkStateUpdate(lsu)))
            }
            5 => {
                let (input, lsa) = LinkStateAcknowledgment::parse(input)?;
                Ok((input, OSPFPacket::LinkStateAcknowledgment(lsa)))
            }
            _ => Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Tag,
            ))),
        }
    }
}

impl OSPFPacket {
    pub fn get_hdr(&self) -> &Header {
        match self {
            OSPFPacket::Hello(hello) => &hello.header,
            OSPFPacket::DBDescription(dbd) => &dbd.header,
            OSPFPacket::LinkStateRequest(lsr) => &lsr.header,
            OSPFPacket::LinkStateUpdate(lsu) => &lsu.header,
            OSPFPacket::LinkStateAcknowledgment(lsa) => &lsa.header,
        }
    }

    pub fn get_mut_hdr(&mut self) -> &mut Header {
        match self {
            OSPFPacket::Hello(hello) => &mut hello.header,
            OSPFPacket::DBDescription(dbd) => &mut dbd.header,
            OSPFPacket::LinkStateRequest(lsr) => &mut lsr.header,
            OSPFPacket::LinkStateUpdate(lsu) => &mut lsu.header,
            OSPFPacket::LinkStateAcknowledgment(lsa) => &mut lsa.header,
        }
    }

    pub fn set_packet_length(&mut self) {
        // not right
        let len = self.encode_bincode().len() as u16;
        // println!("{:?}\n{:?}", self, self.encode_bincode());
        self.get_mut_hdr().packet_length = len;
    }

    pub fn set_checksum(&mut self) {
        self.get_mut_hdr().checksum = 0;
        let bytes = match self {
            OSPFPacket::Hello(hello) => hello.encode(),
            OSPFPacket::DBDescription(dbd) => dbd.encode(),
            OSPFPacket::LinkStateRequest(lsr) => lsr.encode(),
            OSPFPacket::LinkStateUpdate(lsu) => lsu.encode(),
            OSPFPacket::LinkStateAcknowledgment(lsa) => lsa.encode(),
        };
        let mut sum = 0u32;
        for i in 0..bytes.len() / 2 {
            sum += u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]) as u32;
            sum = (sum & 0xffff) + (sum >> 16);
        }
        if bytes.len() % 2 != 0 {
            sum += u16::from_be_bytes([bytes[bytes.len() - 1], 0]) as u32;
            sum = (sum & 0xffff) + (sum >> 16);
        }
        self.get_mut_hdr().checksum = !sum as u16;
    }

    pub fn encode_bincode(&self) -> Vec<u8> {
        match self {
            OSPFPacket::Hello(hello) => hello.encode(),
            OSPFPacket::DBDescription(dbd) => dbd.encode(),
            OSPFPacket::LinkStateRequest(lsr) => lsr.encode(),
            OSPFPacket::LinkStateUpdate(lsu) => lsu.encode(),
            OSPFPacket::LinkStateAcknowledgment(lsa) => lsa.encode(),
        }
    }
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct Header {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: u32,
    pub area_id: u32,
    pub checksum: u16,
    pub auth_type: u16,
    pub auth: u64,
}

#[repr(u8)]
pub enum PacketType {
    Hello = 1,
    DBD,
    LSR,
    LSU,
    LSAck,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct Hello {
    pub header: Header,
    pub network_mask: u32,
    pub hello_interval: u16,
    pub options: u8,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub designated_router: u32,
    pub backup_designated_router: u32,
    pub neighbors: Vec<u32>,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct DBDescription {
    pub header: Header,
    pub interface_mtu: u16,
    pub options: u8,
    pub flags: u8,
    pub dbd_seq_num: u32,
    pub lsa_hdrs: Vec<LsaHeader>,
}

#[derive(Debug, Clone)]
pub struct DBDFlag {
    pub init: bool,
    pub more: bool,
    pub masterslave: bool,
}

impl DBDescription {
    pub fn get_flag(&self) -> DBDFlag {
        DBDFlag::from_byte(self.flags)
    }
    pub fn all_flag_set(&self) -> bool {
        return (self.flags & 0b1110_0000) == 0b1110_0000;
    }
}

impl DBDFlag {
    pub fn new(init: bool, more: bool, masterslave: bool) -> Self {
        DBDFlag {
            init,
            more,
            masterslave,
        }
    }
    pub fn get_all_set() -> Self {
        DBDFlag::from_byte(0b0000_0111)
    }
    pub fn from_byte(byte: u8) -> Self {
        DBDFlag {
            init: (byte & 0b0000_0100) != 0,
            more: (byte & 0b0000_0010) != 0,
            masterslave: (byte & 0b0000_0001) != 0,
        }
    }
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.init {
            byte |= 0b0000_0100;
        }
        if self.more {
            byte |= 0b0000_0010;
        }
        if self.masterslave {
            byte |= 0b0000_0001;
        }
        byte
    }
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LinkStateRequest {
    pub header: Header,
    pub requests: Vec<LinkStateRequestItem>,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LinkStateRequestItem {
    pub link_state_type: u32,
    pub link_state_id: u32,
    pub advertising_router: u32,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LinkStateUpdate {
    pub header: Header,
    pub num_lsa: u32,
    #[nom(Count = "num_lsa")]
    pub lsas: Vec<Lsa>,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LinkStateAcknowledgment {
    pub header: Header,
    pub lsas: Vec<LsaHeader>,
}
