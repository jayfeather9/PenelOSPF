extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;
use syn::{Data, DeriveInput, Fields, Type};

#[proc_macro_derive(Encoding)]
pub fn encoding_derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let encode_impl = match input.data {
        Data::Struct(data_struct) => {
            let encode_fields = match data_struct.fields {
                Fields::Named(ref fields) => {
                    let field_encodes = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        if is_vec_of(ty, "u8")
                            || is_vec_of(ty, "u16")
                            || is_vec_of(ty, "u32")
                            || is_vec_of(ty, "u64")
                        {
                            quote! {
                                for &item in &self.#name {
                                    vec.extend(&item.to_be_bytes());
                                }
                            }
                        } else if is_vec(ty) {
                            quote! {
                                for item in &self.#name {
                                    vec.extend(&item.clone().encode());
                                }
                            }
                        } else if is_type(ty, "u8")
                            || is_type(ty, "u16")
                            || is_type(ty, "u32")
                            || is_type(ty, "u64")
                        {
                            quote! {vec.extend(&self.#name.to_be_bytes());}
                        } else {
                            quote! {vec.extend(&self.#name.clone().encode());}
                        }
                    });
                    quote! {
                        #(#field_encodes)*
                    }
                }
                _ => unimplemented!(),
            };
            quote! {
                impl #name {
                    pub fn encode(&self) -> Vec<u8> {
                        let mut vec = Vec::new();
                        use byteorder::{BigEndian, WriteBytesExt};
                        #encode_fields
                        vec
                    }
                }
            }
        }
        _ => unimplemented!(),
    };

    encode_impl.into()
}

fn is_type(ty: &Type, type_name: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == type_name;
        }
    }
    false
}

fn is_vec_of(ty: &Type, inner_type_name: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if let Some(syn::GenericArgument::Type(ref inner_ty)) = args.args.first() {
                        return is_type(inner_ty, inner_type_name);
                    }
                }
            }
        }
    }
    false
}

fn is_vec(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}
