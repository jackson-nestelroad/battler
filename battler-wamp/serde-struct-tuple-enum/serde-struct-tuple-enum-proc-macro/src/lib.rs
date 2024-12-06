extern crate alloc;
extern crate proc_macro;

use alloc::fmt::format;

use proc_macro::TokenStream;
use proc_macro2::{
    Span,
    TokenTree,
};
use quote::quote;
use syn::{
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    Data,
    DeriveInput,
    Error,
    Expr,
    Field,
    Ident,
    Lit,
    Meta,
};

struct VariantAttrs {
    tag: Lit,
}

struct Variant {
    ident: Ident,
    attrs: VariantAttrs,
    field: Field,
}

struct Input {
    ident: Ident,
    tag: Ident,
    variants: Vec<Variant>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let call_site = Span::call_site();
        let input = DeriveInput::parse(input)?;
        let ident = input.ident;
        let data = match input.data {
            Data::Enum(data) => data,
            _ => return Err(Error::new(call_site, "input must be a struct")),
        };
        let mut tag = None;
        for attr in input.attrs {
            if let Meta::List(list) = attr.meta {
                if list.path.is_ident("tag") {
                    let mut tokens = list.tokens.into_iter();
                    match tokens.next() {
                        Some(TokenTree::Ident(ident)) => {
                            tag = Some(ident);
                        }
                        Some(_) | None => {
                            return Err(Error::new(call_site, "tag attribute must have a type"))
                        }
                    }
                }
            }
        }
        let tag = match tag {
            Some(tag) => tag,
            None => return Err(Error::new(call_site, "missing tag attribute")),
        };
        let variants = data
            .variants
            .into_iter()
            .map(|variant| {
                let mut tag = None;
                for attr in variant.attrs {
                    if let Meta::NameValue(name_value) = attr.meta {
                        if name_value.path.is_ident("tag") {
                            tag = match name_value.value {
                                Expr::Lit(lit) => Some(lit.lit),
                                _ => return Err(Error::new(call_site, "tag must be a literal")),
                            }
                        }
                    }
                }
                let tag = match tag {
                    Some(tag) => tag,
                    None => {
                        return Err(Error::new(
                            call_site,
                            "enum variants must have a tag attribute",
                        ))
                    }
                };
                let attrs = VariantAttrs { tag };
                if variant.fields.len() != 1 {
                    return Err(Error::new(call_site, "enum variants must have one field"));
                }
                let field = variant.fields.into_iter().next().unwrap();
                Ok(Variant {
                    ident: variant.ident,
                    attrs,
                    field,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            ident,
            tag,
            variants,
        })
    }
}

/// Implements [`serde::Deserialize`] for the enum, assuming each enum variant is a simple wrapper
/// around implementations of [`serde_struct_tuple::DeserializeStructTuple`].
#[proc_macro_derive(DeserializeStructTupleEnum, attributes(tag))]
pub fn derive_deserialize_struct_tuple_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);
    let call_site = Span::call_site();

    let ident = input.ident;
    let visitor_ident = Ident::new(&format(format_args!("{ident}Visitor")), call_site);

    let tag = input.tag;

    let match_codes = input
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let code = &variant.attrs.tag;
            let field_ty = &variant.field.ty;
            quote! {
                #code => Ok(#ident::#variant_ident(#field_ty::visitor().visit_seq(value)?))
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl<'de> serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error> where D: serde::Deserializer<'de> {
                struct #visitor_ident;

                impl<'de> serde::de::Visitor<'de> for #visitor_ident {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                        formatter.write_fmt(format_args!("{} tuple", stringify!(#ident)))
                    }

                    fn visit_seq<A>(self, mut value: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let tag: #tag = value.next_element()?.ok_or_else(|| serde::de::Error::missing_field(stringify!(#ident)))?;
                        match tag {
                            #(#match_codes,)*
                            _ => Err(serde::de::Error::invalid_value(serde::de::Unexpected::TupleVariant, &self)),
                        }
                    }
                }

                deserializer.deserialize_seq(#visitor_ident)
            }
        }
    }.into()
}

/// Implements [`serde::Serialize`] for the enum, assuming each enum variant is a simple wrapper
/// around implementations of [`serde_struct_tuple::SerializeStructTuple`].
#[proc_macro_derive(SerializeStructTupleEnum, attributes(tag))]
pub fn derive_serialize_struct_tuple_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);

    let ident = input.ident;

    let match_variant = input
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let tag = &variant.attrs.tag;
            quote! {
                #ident::#variant_ident(inner) => {
                    let mut seq = serializer.serialize_seq(None)?;
                    seq.serialize_element(&#tag)?;
                    inner.serialize_fields_to_seq(&mut seq)?;
                    seq.end()
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer {
                    use serde::ser::SerializeSeq;
                    match self {
                        #(#match_variant)*
                    }
                }
        }
    }
    .into()
}
