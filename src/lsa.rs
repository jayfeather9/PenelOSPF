use encoding_derive::Encoding;
use nom_derive::*;

#[derive(Debug, Clone, NomBE, PartialEq, Encoding)]
pub struct LsaHeader {
    pub age: u16,
    pub options: u8,
    pub ls_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
    pub sequence_number: u32,
    pub checksum: u16,
    pub length: u16,
}

impl LsaHeader {
    pub fn same_ids(&self, oth_hdr: &LsaHeader) -> bool {
        self.ls_type == oth_hdr.ls_type
            && self.link_state_id == oth_hdr.link_state_id
            && self.advertising_router == oth_hdr.advertising_router
    }
}

#[repr(u8)]
pub enum LsaType {
    LsaRouter = 1,
    LsaNetwork,
    LsaSumnet,
    LsaSumasb,
    LsaAsexternal,
}

#[repr(u8)]
pub enum LinkType {
    P2P = 1,
    Transit,
    Stub,
    Virtual,
}

#[derive(Debug, Clone)]
pub enum Lsa {
    LsaRouter(LsaRouter),
    LsaNetwork(LsaNetwork),
    LsaSumnet(LsaSum),
    LsaSumasb(LsaSum),
    LsaAsexternal(LsaAsexternal),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LsaCmpResult {
    Same,
    Newer,
    Older,
}

impl Lsa {
    pub fn get_hdr(&self) -> &LsaHeader {
        match self {
            Lsa::LsaRouter(lsa) => &lsa.header,
            Lsa::LsaNetwork(lsa) => &lsa.header,
            Lsa::LsaSumnet(lsa) => &lsa.header,
            Lsa::LsaSumasb(lsa) => &lsa.header,
            Lsa::LsaAsexternal(lsa) => &lsa.header,
        }
    }

    pub fn get_mut_hdr(&mut self) -> &mut LsaHeader {
        match self {
            Lsa::LsaRouter(lsa) => &mut lsa.header,
            Lsa::LsaNetwork(lsa) => &mut lsa.header,
            Lsa::LsaSumnet(lsa) => &mut lsa.header,
            Lsa::LsaSumasb(lsa) => &mut lsa.header,
            Lsa::LsaAsexternal(lsa) => &mut lsa.header,
        }
    }

    pub fn get_rtr(&mut self) -> Option<&mut LsaRouter> {
        match self {
            Lsa::LsaRouter(lsa) => Some(lsa),
            _ => None,
        }
    }

    pub fn cmp_with(&self, other: &Lsa) -> LsaCmpResult {
        let my_hdr = self.get_hdr();
        let oth_hdr = other.get_hdr();
        assert!(my_hdr.ls_type == oth_hdr.ls_type);
        assert!(my_hdr.link_state_id == oth_hdr.link_state_id);
        assert!(my_hdr.advertising_router == oth_hdr.advertising_router);
        if my_hdr.sequence_number != oth_hdr.sequence_number {
            if my_hdr.sequence_number > oth_hdr.sequence_number {
                LsaCmpResult::Newer
            } else {
                LsaCmpResult::Older
            }
        } else if my_hdr.checksum != oth_hdr.checksum {
            if my_hdr.checksum > oth_hdr.checksum {
                LsaCmpResult::Newer
            } else {
                LsaCmpResult::Older
            }
        } else if my_hdr.age == oth_hdr.age {
            LsaCmpResult::Same
        } else if my_hdr.age == crate::config::MAX_AGE as u16 {
            LsaCmpResult::Newer
        } else if oth_hdr.age == crate::config::MAX_AGE as u16 {
            LsaCmpResult::Older
        } else if (my_hdr.age as i32 - oth_hdr.age as i32).abs() as u32
            > crate::config::MAX_AGE_DIFF
        {
            if my_hdr.age < oth_hdr.age {
                LsaCmpResult::Newer
            } else {
                LsaCmpResult::Older
            }
        } else {
            LsaCmpResult::Same
        }
    }

    pub fn same_ids(&self, oth_hdr: &LsaHeader) -> bool {
        let my_hdr = self.get_hdr();
        my_hdr.ls_type == oth_hdr.ls_type
            && my_hdr.link_state_id == oth_hdr.link_state_id
            && my_hdr.advertising_router == oth_hdr.advertising_router
    }
}

impl<'a> Parse<&'a [u8]> for Lsa {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        assert!(input.len() >= 5, "input too short {:?}", input);
        let ls_type = input[3];
        // println!(
        //     "parsing lsa type {} len {} = {:?}",
        //     ls_type,
        //     input.len(),
        //     input
        // );
        match ls_type {
            1 => {
                let (input, lsa) = LsaRouter::parse(input)?;
                Ok((input, Lsa::LsaRouter(lsa)))
            }
            2 => {
                let (input, lsa) = LsaNetwork::parse(input)?;
                Ok((input, Lsa::LsaNetwork(lsa)))
            }
            3 => {
                let (input, lsa) = LsaSum::parse(input)?;
                Ok((input, Lsa::LsaSumnet(lsa)))
            }
            4 => {
                let (input, lsa) = LsaSum::parse(input)?;
                Ok((input, Lsa::LsaSumasb(lsa)))
            }
            5 => {
                let (input, lsa) = LsaAsexternal::parse(input)?;
                Ok((input, Lsa::LsaAsexternal(lsa)))
            }
            _ => Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Tag,
            ))),
        }
    }
}

impl Lsa {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Lsa::LsaRouter(lsa) => lsa.encode(),
            Lsa::LsaNetwork(lsa) => lsa.encode(),
            Lsa::LsaSumnet(lsa) => lsa.encode(),
            Lsa::LsaSumasb(lsa) => lsa.encode(),
            Lsa::LsaAsexternal(lsa) => lsa.encode(),
        }
    }

    pub fn set_checksum_length(&mut self) {
        let buf = self.encode();
        self.get_mut_hdr().length = buf.len() as u16;
        let mut buf = self.encode();
        // remove first 2 elements
        buf.remove(0);
        buf.remove(0);
        let len = buf.len();
        let sum = fletcher16_checksum(buf, len as u32, 14);
        self.get_mut_hdr().checksum = sum as u16;
    }
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaRouter {
    #[nom(Verify = "header.ls_type == LsaType::LsaRouter as u8")]
    pub header: LsaHeader,
    pub flags: u16,
    pub num_links: u16,
    #[nom(Count = "num_links")]
    pub links: Vec<LsaRouterLink>,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaRouterLink {
    pub link_id: u32,
    pub link_data: u32,
    pub link_type: u8,
    pub tos_num: u8,
    pub metric: u16,
    #[nom(Count = "tos_num")]
    pub tos_list: Vec<LsaRouterLinkTos>,
}

impl LsaRouterLink {
    pub fn new(metric: u16) -> Self {
        LsaRouterLink {
            link_id: 0,
            link_data: 0,
            link_type: 0,
            tos_num: 0,
            metric,
            tos_list: vec![],
        }
    }
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaRouterLinkTos {
    pub tos: u8,
    pub reserved: u8,
    pub metric: u16,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaNetwork {
    #[nom(Verify = "header.ls_type == LsaType::LsaNetwork as u8")]
    pub header: LsaHeader,
    pub network_mask: u32,
    #[nom(Count = "(header.length as usize - 24) / 4")]
    pub attached_routers: Vec<u32>,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaSum {
    #[nom(
        Verify = "header.ls_type == LsaType::LsaSumnet as u8 || header.ls_type == LsaType::LsaSumasb as u8"
    )]
    pub header: LsaHeader,
    pub network_mask: u32,
    pub metric: u32,
}

#[derive(Debug, Clone, NomBE, Encoding)]
pub struct LsaAsexternal {
    #[nom(Verify = "header.ls_type == LsaType::LsaAsexternal as u8")]
    pub header: LsaHeader,
    pub network_mask: u32,
    pub metric: u32,
    pub forwarding_address: u32,
    pub external_route_tag: u32,
}

fn fletcher16_checksum(data: Vec<u8>, len: u32, offset: u32) -> u16 {
    let mut c0 = 0i32;
    let mut c1 = 0i32;
    for idx in 0..len {
        if idx == offset || idx == offset + 1 {
            c1 += c0;
            c0 %= 255;
            c1 %= 255;
        } else {
            c0 += data[idx as usize] as i32;
            c1 += c0;
            c0 %= 255;
            c1 %= 255;
        }
    }

    c0 %= 255;
    c1 %= 255;
    let mul = (len - offset) as i32 * c0;

    let mut x = mul - c0 - c1;
    let mut y = c1 - mul - 1;

    if y >= 0 {
        y += 1
    }
    if x < 0 {
        x -= 1;
    }

    x %= 255;
    y %= 255;

    if x == 0 {
        x = 255;
    }
    if y == 0 {
        y = 255;
    }

    y &= 0xFF;

    ((x << 8) | y) as u16
}
