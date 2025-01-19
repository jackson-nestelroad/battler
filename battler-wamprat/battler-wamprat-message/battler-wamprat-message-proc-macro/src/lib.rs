extern crate proc_macro;

use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Error,
    Field,
    Ident,
    Index,
    ItemStruct,
    Meta,
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
};

enum ApplicationMessageFieldType {
    Arguments,
    ArgumentsKeyword,
}

struct InputFieldAttrs {
    field_type: ApplicationMessageFieldType,
}

fn parse_application_message_input_field_attrs(field: &Field) -> syn::Result<InputFieldAttrs> {
    let call_site = Span::call_site();
    let mut field_type = None;
    for attr in &field.attrs {
        if let Meta::Path(path) = &attr.meta {
            if path.is_ident("arguments") {
                field_type = Some(ApplicationMessageFieldType::Arguments);
            } else if path.is_ident("arguments_keyword") {
                field_type = Some(ApplicationMessageFieldType::ArgumentsKeyword);
            }
        }
    }
    let field_type = match field_type {
        Some(field_type) => field_type,
        None => {
            return Err(Error::new(
                call_site,
                "field must be marked `arguments` or `arguments_keyword`",
            ));
        }
    };
    Ok(InputFieldAttrs { field_type })
}

struct InputField {
    ident: Option<Ident>,
    attrs: InputFieldAttrs,
}

struct Input {
    ident: Ident,
    fields: Vec<InputField>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let call_site = Span::call_site();
        let input = match ItemStruct::parse(input) {
            Ok(item) => item,
            Err(_) => return Err(Error::new(call_site, "input must be a struct")),
        };
        let ident = input.ident;
        let fields = input
            .fields
            .into_iter()
            .map(|field| {
                let attrs = parse_application_message_input_field_attrs(&field)?;
                Ok(InputField {
                    ident: field.ident,
                    attrs,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self { ident, fields })
    }
}

/// Procedural macro for deriving `battler_wamp_values::WampApplicationMessage` for a struct.
#[proc_macro_derive(WampApplicationMessage, attributes(arguments, arguments_keyword))]
pub fn derive_wamp_application_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);
    let call_site = Span::call_site();

    let ident = input.ident;

    let (field_serializers, field_deserializers, field_identifiers): (Vec<_>, Vec<_>, Vec<_>) = input.fields.iter().enumerate().map(|(i, field)| {
        let accessor = match &field.ident {
            Some(ident) => quote!(self.#ident),
            None => { let i = Index::from(i); quote!(self.#i) },
        };
        let field_name = field.ident.clone().unwrap_or(Ident::new(&format!("field_{i}"), call_site));
        let input_output_ident = match field.attrs.field_type {
            ApplicationMessageFieldType::Arguments => quote!(arguments),
            ApplicationMessageFieldType::ArgumentsKeyword => quote!(arguments_keyword),
        };
        let validate_serialize_output = match field.attrs.field_type {
            ApplicationMessageFieldType::Arguments => quote! {
                match val {
                    battler_wamp_values::Value::List(val) => val,
                    _ => return Err(battler_wamp_values::WampSerializeError::new(std::fmt::format(format_args!("arguments of {} did not produce a list", std::stringify!(#ident))))),
                }
            },
            ApplicationMessageFieldType::ArgumentsKeyword => quote! {
                match val {
                    battler_wamp_values::Value::Dictionary(val) => val,
                    _ => return Err(battler_wamp_values::WampSerializeError::new(std::fmt::format(format_args!("arguments of {} did not produce a list", std::stringify!(#ident))))),
                }
            },
        };
        (
            quote! {
                let #input_output_ident = match battler_wamp_values::WampSerialize::wamp_serialize(#accessor) {
                    Ok(val) => #validate_serialize_output,
                    Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to serialize {} of {}", std::stringify!(#input_output_ident), std::stringify!(#ident))))),
                };
            },
            quote! {
                let #field_name = match battler_wamp_values::WampDeserialize::wamp_deserialize(#input_output_ident) {
                    Ok(val) => val,
                    Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to deserialize {} of {}", std::stringify!(#input_output_ident), std::stringify!(#ident)))))
                };
            },
            quote!(#field_name)
        )
    }).multiunzip();

    let named = input.fields.is_empty() || input.fields.iter().any(|field| field.ident.is_some());
    let struct_constructor = if named {
        quote!(#ident { #(#field_identifiers,)* })
    } else {
        quote!(#ident(#(#field_identifiers,)*))
    };

    quote!{
        impl battler_wamprat_message::WampApplicationMessage for #ident {
            fn wamp_serialize_application_message(self) -> core::result::Result<(battler_wamp_values::List, battler_wamp_values::Dictionary), battler_wamp_values::WampSerializeError> {
                let arguments = battler_wamp_values::List::default();
                let arguments_keyword = battler_wamp_values::Dictionary::default();
                #(#field_serializers)*
                Ok((arguments, arguments_keyword))
            }

            fn wamp_deserialize_application_message(
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
            ) -> core::result::Result<Self, battler_wamp_values::WampDeserializeError> {
                let arguments = battler_wamp_values::Value::List(arguments);
                let arguments_keyword = battler_wamp_values::Value::Dictionary(arguments_keyword);
                #(#field_deserializers)*
                Ok(#struct_constructor)
            }
        }
    }.into()
}
