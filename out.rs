mod lsa {
    use encoding_derive::Encoding;
    use nom::combinator::peek;
    use nom::number::streaming::be_u16;
    use nom_derive::*;
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
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaHeader {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "age",
                "options",
                "ls_type",
                "link_state_id",
                "advertising_router",
                "sequence_number",
                "checksum",
                "length",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.age,
                &self.options,
                &self.ls_type,
                &self.link_state_id,
                &self.advertising_router,
                &self.sequence_number,
                &self.checksum,
                &&self.length,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "LsaHeader",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaHeader {
        #[inline]
        fn clone(&self) -> LsaHeader {
            LsaHeader {
                age: ::core::clone::Clone::clone(&self.age),
                options: ::core::clone::Clone::clone(&self.options),
                ls_type: ::core::clone::Clone::clone(&self.ls_type),
                link_state_id: ::core::clone::Clone::clone(&self.link_state_id),
                advertising_router: ::core::clone::Clone::clone(
                    &self.advertising_router,
                ),
                sequence_number: ::core::clone::Clone::clone(&self.sequence_number),
                checksum: ::core::clone::Clone::clone(&self.checksum),
                length: ::core::clone::Clone::clone(&self.length),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaHeader {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, age) = <u16>::parse_be(i)?;
            let (i, options) = <u8>::parse_be(i)?;
            let (i, ls_type) = <u8>::parse_be(i)?;
            let (i, link_state_id) = <u32>::parse_be(i)?;
            let (i, advertising_router) = <u32>::parse_be(i)?;
            let (i, sequence_number) = <u32>::parse_be(i)?;
            let (i, checksum) = <u16>::parse_be(i)?;
            let (i, length) = <u16>::parse_be(i)?;
            let struct_def = LsaHeader {
                age,
                options,
                ls_type,
                link_state_id,
                advertising_router,
                sequence_number,
                checksum,
                length,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, age) = <u16>::parse_be(i)?;
            let (i, options) = <u8>::parse_be(i)?;
            let (i, ls_type) = <u8>::parse_be(i)?;
            let (i, link_state_id) = <u32>::parse_be(i)?;
            let (i, advertising_router) = <u32>::parse_be(i)?;
            let (i, sequence_number) = <u32>::parse_be(i)?;
            let (i, checksum) = <u16>::parse_be(i)?;
            let (i, length) = <u16>::parse_be(i)?;
            let struct_def = LsaHeader {
                age,
                options,
                ls_type,
                link_state_id,
                advertising_router,
                sequence_number,
                checksum,
                length,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for LsaHeader {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for LsaHeader {
        #[inline]
        fn eq(&self, other: &LsaHeader) -> bool {
            self.age == other.age && self.options == other.options
                && self.ls_type == other.ls_type
                && self.link_state_id == other.link_state_id
                && self.advertising_router == other.advertising_router
                && self.sequence_number == other.sequence_number
                && self.checksum == other.checksum && self.length == other.length
        }
    }
    impl LsaHeader {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.age.clone().to_be_bytes());
            vec.extend(&self.options.clone().to_be_bytes());
            vec.extend(&self.ls_type.clone().to_be_bytes());
            vec.extend(&self.link_state_id.clone().to_be_bytes());
            vec.extend(&self.advertising_router.clone().to_be_bytes());
            vec.extend(&self.sequence_number.clone().to_be_bytes());
            vec.extend(&self.checksum.clone().to_be_bytes());
            vec.extend(&self.length.clone().to_be_bytes());
            vec
        }
    }
    #[repr(u8)]
    enum LsaType {
        LsaRouter = 1,
        LsaNetwork,
        LsaSumnet,
        LsaSumasb,
        LsaAsexternal,
    }
    #[repr(u8)]
    enum LinkType {
        P2P = 1,
        Transit,
        Stub,
        Virtual,
    }
    pub enum Lsa {
        LsaRouter(LsaRouter),
        LsaNetwork(LsaNetwork),
        LsaSumnet(LsaSum),
        LsaSumasb(LsaSum),
        LsaAsexternal(LsaAsexternal),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Lsa {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Lsa::LsaRouter(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LsaRouter",
                        &__self_0,
                    )
                }
                Lsa::LsaNetwork(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LsaNetwork",
                        &__self_0,
                    )
                }
                Lsa::LsaSumnet(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LsaSumnet",
                        &__self_0,
                    )
                }
                Lsa::LsaSumasb(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LsaSumasb",
                        &__self_0,
                    )
                }
                Lsa::LsaAsexternal(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LsaAsexternal",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Lsa {
        #[inline]
        fn clone(&self) -> Lsa {
            match self {
                Lsa::LsaRouter(__self_0) => {
                    Lsa::LsaRouter(::core::clone::Clone::clone(__self_0))
                }
                Lsa::LsaNetwork(__self_0) => {
                    Lsa::LsaNetwork(::core::clone::Clone::clone(__self_0))
                }
                Lsa::LsaSumnet(__self_0) => {
                    Lsa::LsaSumnet(::core::clone::Clone::clone(__self_0))
                }
                Lsa::LsaSumasb(__self_0) => {
                    Lsa::LsaSumasb(::core::clone::Clone::clone(__self_0))
                }
                Lsa::LsaAsexternal(__self_0) => {
                    Lsa::LsaAsexternal(::core::clone::Clone::clone(__self_0))
                }
            }
        }
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
    }
    impl<'a> Parse<&'a [u8]> for Lsa {
        fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
            let (_, word) = peek(be_u16)(input)?;
            let ls_type = (word & 0xff) as u8;
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
                _ => {
                    Err(
                        nom::Err::Error(
                            nom::error::make_error(input, nom::error::ErrorKind::Tag),
                        ),
                    )
                }
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
    }
    pub struct LsaRouter {
        #[nom(Verify = "header.ls_type == LsaType::LsaRouter as u8")]
        pub header: LsaHeader,
        pub flags: u16,
        pub num_links: u16,
        #[nom(Count = "num_links")]
        pub links: Vec<LsaRouterLink>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaRouter {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "LsaRouter",
                "header",
                &self.header,
                "flags",
                &self.flags,
                "num_links",
                &self.num_links,
                "links",
                &&self.links,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaRouter {
        #[inline]
        fn clone(&self) -> LsaRouter {
            LsaRouter {
                header: ::core::clone::Clone::clone(&self.header),
                flags: ::core::clone::Clone::clone(&self.flags),
                num_links: ::core::clone::Clone::clone(&self.num_links),
                links: ::core::clone::Clone::clone(&self.links),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaRouter {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaRouter as u8 },
            )(i)?;
            let (i, flags) = <u16>::parse_be(i)?;
            let (i, num_links) = <u16>::parse_be(i)?;
            let (i, links) = nom::multi::count(
                <LsaRouterLink>::parse_be,
                num_links as usize,
            )(i)?;
            let struct_def = LsaRouter {
                header,
                flags,
                num_links,
                links,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaRouter as u8 },
            )(i)?;
            let (i, flags) = <u16>::parse_be(i)?;
            let (i, num_links) = <u16>::parse_be(i)?;
            let (i, links) = nom::multi::count(
                <LsaRouterLink>::parse_be,
                num_links as usize,
            )(i)?;
            let struct_def = LsaRouter {
                header,
                flags,
                num_links,
                links,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaRouter {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.header.clone().encode());
            vec.extend(&self.flags.clone().to_be_bytes());
            vec.extend(&self.num_links.clone().to_be_bytes());
            for &item in &self.links {
                vec.extend(&item.clone().encode());
            }
            vec
        }
    }
    pub struct LsaRouterLink {
        pub link_id: u32,
        pub link_data: u32,
        pub link_type: u8,
        pub tos_num: u8,
        pub metric: u16,
        #[nom(Count = "tos_num")]
        pub tos_list: Vec<LsaRouterLinkTos>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaRouterLink {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "link_id",
                "link_data",
                "link_type",
                "tos_num",
                "metric",
                "tos_list",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.link_id,
                &self.link_data,
                &self.link_type,
                &self.tos_num,
                &self.metric,
                &&self.tos_list,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "LsaRouterLink",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaRouterLink {
        #[inline]
        fn clone(&self) -> LsaRouterLink {
            LsaRouterLink {
                link_id: ::core::clone::Clone::clone(&self.link_id),
                link_data: ::core::clone::Clone::clone(&self.link_data),
                link_type: ::core::clone::Clone::clone(&self.link_type),
                tos_num: ::core::clone::Clone::clone(&self.tos_num),
                metric: ::core::clone::Clone::clone(&self.metric),
                tos_list: ::core::clone::Clone::clone(&self.tos_list),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaRouterLink {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, link_id) = <u32>::parse_be(i)?;
            let (i, link_data) = <u32>::parse_be(i)?;
            let (i, link_type) = <u8>::parse_be(i)?;
            let (i, tos_num) = <u8>::parse_be(i)?;
            let (i, metric) = <u16>::parse_be(i)?;
            let (i, tos_list) = nom::multi::count(
                <LsaRouterLinkTos>::parse_be,
                tos_num as usize,
            )(i)?;
            let struct_def = LsaRouterLink {
                link_id,
                link_data,
                link_type,
                tos_num,
                metric,
                tos_list,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, link_id) = <u32>::parse_be(i)?;
            let (i, link_data) = <u32>::parse_be(i)?;
            let (i, link_type) = <u8>::parse_be(i)?;
            let (i, tos_num) = <u8>::parse_be(i)?;
            let (i, metric) = <u16>::parse_be(i)?;
            let (i, tos_list) = nom::multi::count(
                <LsaRouterLinkTos>::parse_be,
                tos_num as usize,
            )(i)?;
            let struct_def = LsaRouterLink {
                link_id,
                link_data,
                link_type,
                tos_num,
                metric,
                tos_list,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaRouterLink {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.link_id.clone().to_be_bytes());
            vec.extend(&self.link_data.clone().to_be_bytes());
            vec.extend(&self.link_type.clone().to_be_bytes());
            vec.extend(&self.tos_num.clone().to_be_bytes());
            vec.extend(&self.metric.clone().to_be_bytes());
            for &item in &self.tos_list {
                vec.extend(&item.clone().encode());
            }
            vec
        }
    }
    pub struct LsaRouterLinkTos {
        pub tos: u8,
        pub reserved: u8,
        pub metric: u16,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaRouterLinkTos {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "LsaRouterLinkTos",
                "tos",
                &self.tos,
                "reserved",
                &self.reserved,
                "metric",
                &&self.metric,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaRouterLinkTos {
        #[inline]
        fn clone(&self) -> LsaRouterLinkTos {
            LsaRouterLinkTos {
                tos: ::core::clone::Clone::clone(&self.tos),
                reserved: ::core::clone::Clone::clone(&self.reserved),
                metric: ::core::clone::Clone::clone(&self.metric),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaRouterLinkTos {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, tos) = <u8>::parse_be(i)?;
            let (i, reserved) = <u8>::parse_be(i)?;
            let (i, metric) = <u16>::parse_be(i)?;
            let struct_def = LsaRouterLinkTos {
                tos,
                reserved,
                metric,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, tos) = <u8>::parse_be(i)?;
            let (i, reserved) = <u8>::parse_be(i)?;
            let (i, metric) = <u16>::parse_be(i)?;
            let struct_def = LsaRouterLinkTos {
                tos,
                reserved,
                metric,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaRouterLinkTos {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.tos.clone().to_be_bytes());
            vec.extend(&self.reserved.clone().to_be_bytes());
            vec.extend(&self.metric.clone().to_be_bytes());
            vec
        }
    }
    pub struct LsaNetwork {
        #[nom(Verify = "header.ls_type == LsaType::LsaNetwork as u8")]
        pub header: LsaHeader,
        pub network_mask: u32,
        pub attached_routers: Vec<u32>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaNetwork {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "LsaNetwork",
                "header",
                &self.header,
                "network_mask",
                &self.network_mask,
                "attached_routers",
                &&self.attached_routers,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaNetwork {
        #[inline]
        fn clone(&self) -> LsaNetwork {
            LsaNetwork {
                header: ::core::clone::Clone::clone(&self.header),
                network_mask: ::core::clone::Clone::clone(&self.network_mask),
                attached_routers: ::core::clone::Clone::clone(&self.attached_routers),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaNetwork {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaNetwork as u8 },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, attached_routers) = <Vec<u32>>::parse_be(i)?;
            let struct_def = LsaNetwork {
                header,
                network_mask,
                attached_routers,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaNetwork as u8 },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, attached_routers) = <Vec<u32>>::parse_be(i)?;
            let struct_def = LsaNetwork {
                header,
                network_mask,
                attached_routers,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaNetwork {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.header.clone().encode());
            vec.extend(&self.network_mask.clone().to_be_bytes());
            for &item in &self.attached_routers {
                vec.extend(&item.clone().to_be_bytes());
            }
            vec
        }
    }
    pub struct LsaSum {
        #[nom(
            Verify = "header.ls_type == LsaType::LsaSumnet as u8 || header.ls_type == LsaType::LsaSumasb as u8"
        )]
        pub header: LsaHeader,
        pub network_mask: u32,
        pub metric: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaSum {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "LsaSum",
                "header",
                &self.header,
                "network_mask",
                &self.network_mask,
                "metric",
                &&self.metric,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaSum {
        #[inline]
        fn clone(&self) -> LsaSum {
            LsaSum {
                header: ::core::clone::Clone::clone(&self.header),
                network_mask: ::core::clone::Clone::clone(&self.network_mask),
                metric: ::core::clone::Clone::clone(&self.metric),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>> for LsaSum {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| {
                    header.ls_type == LsaType::LsaSumnet as u8
                        || header.ls_type == LsaType::LsaSumasb as u8
                },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, metric) = <u32>::parse_be(i)?;
            let struct_def = LsaSum {
                header,
                network_mask,
                metric,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| {
                    header.ls_type == LsaType::LsaSumnet as u8
                        || header.ls_type == LsaType::LsaSumasb as u8
                },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, metric) = <u32>::parse_be(i)?;
            let struct_def = LsaSum {
                header,
                network_mask,
                metric,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaSum {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.header.clone().encode());
            vec.extend(&self.network_mask.clone().to_be_bytes());
            vec.extend(&self.metric.clone().to_be_bytes());
            vec
        }
    }
    pub struct LsaAsexternal {
        #[nom(Verify = "header.ls_type == LsaType::LsaAsexternal as u8")]
        pub header: LsaHeader,
        pub network_mask: u32,
        pub metric: u32,
        pub forwarding_address: u32,
        pub external_route_tag: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LsaAsexternal {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "LsaAsexternal",
                "header",
                &self.header,
                "network_mask",
                &self.network_mask,
                "metric",
                &self.metric,
                "forwarding_address",
                &self.forwarding_address,
                "external_route_tag",
                &&self.external_route_tag,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for LsaAsexternal {
        #[inline]
        fn clone(&self) -> LsaAsexternal {
            LsaAsexternal {
                header: ::core::clone::Clone::clone(&self.header),
                network_mask: ::core::clone::Clone::clone(&self.network_mask),
                metric: ::core::clone::Clone::clone(&self.metric),
                forwarding_address: ::core::clone::Clone::clone(
                    &self.forwarding_address,
                ),
                external_route_tag: ::core::clone::Clone::clone(&self.external_route_tag),
            }
        }
    }
    impl<'nom> nom_derive::Parse<&'nom [u8], nom::error::Error<&'nom [u8]>>
    for LsaAsexternal {
        fn parse_be(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaAsexternal as u8 },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, metric) = <u32>::parse_be(i)?;
            let (i, forwarding_address) = <u32>::parse_be(i)?;
            let (i, external_route_tag) = <u32>::parse_be(i)?;
            let struct_def = LsaAsexternal {
                header,
                network_mask,
                metric,
                forwarding_address,
                external_route_tag,
            };
            Ok((i, struct_def))
        }
        fn parse_le(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            let i = orig_i;
            let (i, header) = nom::combinator::verify(
                <LsaHeader>::parse_be,
                |header| { header.ls_type == LsaType::LsaAsexternal as u8 },
            )(i)?;
            let (i, network_mask) = <u32>::parse_be(i)?;
            let (i, metric) = <u32>::parse_be(i)?;
            let (i, forwarding_address) = <u32>::parse_be(i)?;
            let (i, external_route_tag) = <u32>::parse_be(i)?;
            let struct_def = LsaAsexternal {
                header,
                network_mask,
                metric,
                forwarding_address,
                external_route_tag,
            };
            Ok((i, struct_def))
        }
        fn parse(orig_i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self> {
            Self::parse_be(orig_i)
        }
    }
    impl LsaAsexternal {
        pub fn encode(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            use byteorder::{BigEndian, WriteBytesExt};
            vec.extend(&self.header.clone().encode());
            vec.extend(&self.network_mask.clone().to_be_bytes());
            vec.extend(&self.metric.clone().to_be_bytes());
            vec.extend(&self.forwarding_address.clone().to_be_bytes());
            vec.extend(&self.external_route_tag.clone().to_be_bytes());
            vec
        }
    }
}
