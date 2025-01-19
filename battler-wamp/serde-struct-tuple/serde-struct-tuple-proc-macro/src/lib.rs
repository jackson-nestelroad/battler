#![no_std]

extern crate alloc;
extern crate proc_macro;

use alloc::{
    fmt::format,
    vec::Vec,
};

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Error,
    Field,
    Ident,
    ItemStruct,
    Meta,
    Path,
    Type,
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
};

#[derive(Default)]
enum DefaultAttr {
    #[default]
    False,
    True,
    Path(Path),
}

impl DefaultAttr {
    pub fn can_be_default(&self) -> bool {
        match self {
            Self::False => false,
            _ => true,
        }
    }
}

#[derive(Default)]
struct InputFieldAttrs {
    default: DefaultAttr,
    skip_serializing_if: Option<Path>,
}

struct InputField {
    ident: Option<Ident>,
    ty: Type,
    attrs: InputFieldAttrs,
}

struct Input {
    ident: Ident,
    fields: Vec<InputField>,
}

fn parse_input_field_attrs(field: &Field) -> syn::Result<InputFieldAttrs> {
    let serde_attr = field.attrs.iter().find(|attr| {
        if let Meta::List(list) = &attr.meta {
            if list.path.is_ident("serde_struct_tuple") {
                return true;
            }
        }
        false
    });
    let serde_attr = match serde_attr {
        Some(attr) => attr,
        None => return Ok(InputFieldAttrs::default()),
    };

    let mut default = DefaultAttr::False;
    let mut skip_serializing_if = None;
    serde_attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("default") {
            default = match meta.value() {
                Ok(value) => DefaultAttr::Path(value.parse::<Path>()?),
                Err(_) => DefaultAttr::True,
            }
        }
        if meta.path.is_ident("skip_serializing_if") {
            let value = meta.value()?;
            skip_serializing_if = Some(value.parse::<Path>()?);
        }
        Ok(())
    })?;
    Ok(InputFieldAttrs {
        default,
        skip_serializing_if,
    })
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let call_site = Span::call_site();
        let input = match ItemStruct::parse(input) {
            Ok(item) => item,
            Err(_) => return Err(Error::new(call_site, "input must be a struct")),
        };
        let ident = input.ident;
        let mut fields = Vec::new();
        let mut defaulted = false;
        for field in input.fields {
            let attrs = parse_input_field_attrs(&field)?;
            if attrs.default.can_be_default() {
                defaulted = true
            } else if defaulted {
                return Err(Error::new(
                    call_site,
                    "fields after a default field must also be default",
                ));
            }
            fields.push(InputField {
                ident: field.ident,
                ty: field.ty,
                attrs,
            });
        }
        Ok(Input { ident, fields })
    }
}

/// Implements `serde_struct_tuple::DeserializeStructTuple` and `serde::Deserialize` for the
/// struct.
#[proc_macro_derive(DeserializeStructTuple, attributes(serde_struct_tuple))]
pub fn derive_deserialize_struct_tuple(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);
    let call_site = Span::call_site();

    let ident = input.ident;
    let visitor_ident = Ident::new(&format(format_args!("{ident}Visitor")), call_site);
    let field_deserializers = input
        .fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let field_ty = &field.ty;
            let if_empty = match &field.attrs.default {
                DefaultAttr::False => {
                    quote!(return Err(serde::de::Error::missing_field(stringify!(#ident))))
                }
                DefaultAttr::True => quote!(#field_ty::default()),
                DefaultAttr::Path(path) => quote!(#path()),
            };
            quote!(#ident: match value.next_element()? {
                Some(value) => value,
                None => #if_empty,
            })
        })
        .collect::<Vec<_>>();

    quote! {
        impl serde_struct_tuple::DeserializeStructTuple for #ident {
            type Value = #ident;
            fn visitor<'de>() -> impl serde::de::Visitor<'de, Value = Self::Value> {
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
                        Ok(#ident {
                            #(#field_deserializers,)*
                        })
                    }
                }

                #visitor_ident
            }
        }


        impl<'de> serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error> where D: serde::Deserializer<'de> {
                deserializer.deserialize_seq(Self::visitor())
            }
        }
    }
    .into()
}

/// Implements `serde_struct_tuple::SerializeStructTuple` and `serde::Serialize` for the struct.
#[proc_macro_derive(SerializeStructTuple, attributes(serde_struct_tuple))]
pub fn derive_serialize_struct_tuple(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);

    let ident = input.ident;

    let field_serializers = input
        .fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let field_serializer_check = match &field.attrs.skip_serializing_if {
                Some(skip_serializing_if) => Some(quote! {
                    if #skip_serializing_if(&self.#ident) {
                        return Ok(());
                    }
                }),
                None => None,
            };
            quote! {
                #field_serializer_check
                seq.serialize_element(&self.#ident)?;
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl serde_struct_tuple::SerializeStructTuple for #ident {
            fn serialize_fields_to_seq<S>(&self, seq: &mut S) -> core::result::Result<(), S::Error> where S: serde::ser::SerializeSeq {
                use serde::ser::SerializeSeq;
                #(#field_serializers)*
                Ok(())
            }
        }

        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> where S: serde::Serializer {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(None)?;
                self.serialize_fields_to_seq(&mut seq)?;
                seq.end()
            }
        }
    }
    .into()
}
