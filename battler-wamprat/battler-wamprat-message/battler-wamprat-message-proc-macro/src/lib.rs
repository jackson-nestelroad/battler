extern crate proc_macro;

use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    Error,
    Field,
    Ident,
    ItemStruct,
    Meta,
    Path,
    Type,
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

fn parse_input_field_attrs(field: &Field) -> syn::Result<InputFieldAttrs> {
    let attr = field.attrs.iter().find(|attr| {
        if let Meta::List(list) = &attr.meta {
            if list.path.is_ident("battler_wamprat_message") {
                return true;
            }
        }
        false
    });
    let attr = match attr {
        Some(attr) => attr,
        None => return Ok(InputFieldAttrs::default()),
    };

    let mut default = DefaultAttr::False;
    let mut skip_serializing_if = None;
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("default") {
            default = match meta.value() {
                Ok(value) => DefaultAttr::Path(value.parse::<Path>()?),
                Err(_) => DefaultAttr::True,
            };
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

struct InputField {
    ident: Option<Ident>,
    ty: Type,
    attrs: InputFieldAttrs,
}

struct ListInput {
    ident: Ident,
    fields: Vec<InputField>,
}

impl Parse for ListInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let call_site = Span::call_site();
        let input = match ItemStruct::parse(input) {
            Ok(item) => item,
            Err(_) => return Err(Error::new(call_site, "input must be a struct")),
        };
        let ident = input.ident;
        let mut defaulted = false;
        let fields = input
            .fields
            .into_iter()
            .map(|field| {
                let attrs = parse_input_field_attrs(&field)?;
                if attrs.default.can_be_default() {
                    defaulted = true;
                } else if defaulted {
                    return Err(Error::new(
                        call_site,
                        "fields after a default field must also have a default",
                    ));
                }
                Ok(InputField {
                    ident: field.ident,
                    ty: field.ty,
                    attrs,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self { ident, fields })
    }
}

/// Procedural macro for deriving [`battler_wamprat_message::WampSerialize`] and
/// [`battler_wamprat_message::WampDeserialize`] for a struct that converts to a
/// [`battler_wamprat_message::List`].
#[proc_macro_derive(WampList, attributes(battler_wamprat_message))]
pub fn derive_wamp_list(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ListInput);
    let call_site = Span::call_site();

    let ident = input.ident;

    let (field_serializers, field_deserializers, field_identifiers): (Vec<_>, Vec<_>, Vec<_>) = input.fields.iter().enumerate().map(|(i, field)| {
        let accessor = match &field.ident {
            Some(ident) => quote!(self.#ident),
            None => quote!(self.#i),
        };
        let ty = &field.ty;
        let field_name = field.ident.clone().unwrap_or(Ident::new(&format!("field_{i}"), call_site));
        let serialize_check = match &field.attrs.skip_serializing_if {
            Some(skip_serializing_if) => Some(quote! {
                if #skip_serializing_if(&#accessor) {
                    return Ok(battler_wamprat_message::Value::List(list));
                }
            }),
            None => None,
        };
        let if_empty = match &field.attrs.default {
            DefaultAttr::False => quote!(return Err(battler_wamprat_message::WampDeserializeError::new(std::fmt::format(format_args!("list member {} of {} is missing", std::stringify!(#field_name), std::stringify!(#ident)))))),
            DefaultAttr::True => quote!(<#ty as Default>::default()),
            DefaultAttr::Path(path) => quote!(#path()),
        };
        (
            quote! {
                #serialize_check
                match battler_wamprat_message::WampSerialize::wamp_serialize(#accessor) {
                    Ok(val) => list.push(val),
                    Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to serialize list member {} of {}", std::stringify!(#field_name), std::stringify!(#ident))))),
                }
            },
            quote! {
                let #field_name = match list.get_mut(#i) {
                    Some(val) => {
                        let mut out = battler_wamprat_message::Value::Bool(false);
                        std::mem::swap(val, &mut out);
                        Some(out)
                    }
                    None => None,
                };
                let #field_name = match #field_name {
                    Some(val) => match battler_wamprat_message::WampDeserialize::wamp_deserialize(val) {
                        Ok(val) => val,
                        Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to deserialize list member {} of {}", std::stringify!(#field_name), std::stringify!(#ident)))))
                    },
                    None => #if_empty,
                };
            },
            quote!(#field_name)
        )
    }).multiunzip();

    let serialize = quote! {
        impl battler_wamprat_message::WampSerialize for #ident {
            fn wamp_serialize(self) -> core::result::Result<battler_wamprat_message::Value, battler_wamprat_message::WampSerializeError> {
                let mut list = battler_wamprat_message::List::default();
                #(#field_serializers)*
                Ok(battler_wamprat_message::Value::List(list))
            }
        }
    };

    let named = input.fields.is_empty() || input.fields.iter().any(|field| field.ident.is_some());
    let struct_constructor = if named {
        quote!(#ident { #(#field_identifiers,)* })
    } else {
        quote!(#ident(#(#field_identifiers,)*))
    };
    let deserialize = quote! {
        impl battler_wamprat_message::WampDeserialize for #ident {
            fn wamp_deserialize(value: battler_wamprat_message::Value) -> core::result::Result<Self, battler_wamprat_message::WampDeserializeError> {
                let mut list = match value {
                    battler_wamprat_message::Value::List(list) => list,
                    _ => return Err(battler_wamprat_message::WampDeserializeError::new("value must be a list")),
                };
                #(#field_deserializers)*
                Ok(#struct_constructor)
            }
        }
    };

    quote! {
        #serialize
        #deserialize
    }
    .into()
}

struct DictionaryInput {
    ident: Ident,
    fields: Vec<InputField>,
}

impl Parse for DictionaryInput {
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
                let attrs = parse_input_field_attrs(&field)?;
                Ok(InputField {
                    ident: field.ident,
                    ty: field.ty,
                    attrs,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self { ident, fields })
    }
}

/// Procedural macro for deriving [`battler_wamprat_message::WampSerialize`] and
/// [`battler_wamprat_message::WampDeserialize`] for a struct that converts to a
/// [`battler_wamprat_message::Dictionary`].
#[proc_macro_derive(WampDictionary, attributes(battler_wamprat_message))]
pub fn derive_wamp_dictionary(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DictionaryInput);
    let call_site = Span::call_site();

    let ident = input.ident;

    let (field_serializers, field_deserializers, field_identifiers): (Vec<_>, Vec<_>, Vec<_>) = input.fields.iter().enumerate().map(|(i, field)| {
        let accessor = match &field.ident {
            Some(ident) => quote!(self.#ident),
            None => quote!(self.#i),
        };
        let ty = &field.ty;
        let field_name = field.ident.clone().unwrap_or(Ident::new(&format!("field_{i}"), call_site));
        let serialize_check = match &field.attrs.skip_serializing_if {
            Some(skip_serializing_if) => quote!(!#skip_serializing_if(&#accessor)),
            None => quote!(true),
        };
        let if_empty = match &field.attrs.default {
            DefaultAttr::False => quote!(return Err(battler_wamprat_message::WampDeserializeError::new(std::fmt::format(format_args!("dictionary member {} of {} is missing", std::stringify!(#field_name), std::stringify!(#ident)))))),
            DefaultAttr::True => quote!(<#ty as Default>::default()),
            DefaultAttr::Path(path) => quote!(#path()),
        };
        (
            quote! {
                if #serialize_check {
                    match battler_wamprat_message::WampSerialize::wamp_serialize(#accessor) {
                        Ok(val) => dict.insert(stringify!(#field_name).to_owned(), val),
                        Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to serialize dictionary member {} of {}", std::stringify!(#field_name), std::stringify!(#ident))))),
                    };
                }
            },
            quote! {
                let #field_name = match dict.get_mut(stringify!(#field_name)) {
                    Some(val) => {
                        let mut out = battler_wamprat_message::Value::Bool(false);
                        std::mem::swap(val, &mut out);
                        Some(out)
                    }
                    None => None,
                };
                let #field_name = match #field_name {
                    Some(val) => match battler_wamprat_message::WampDeserialize::wamp_deserialize(val) {
                        Ok(val) => val,
                        Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to deserialize dictionary member {} of {}", std::stringify!(#field_name), std::stringify!(#ident)))))
                    },
                    None => #if_empty,
                };
            },
            quote!(#field_name)
        )
    }).multiunzip();

    let serialize = quote! {
        impl battler_wamprat_message::WampSerialize for #ident {
            fn wamp_serialize(self) -> core::result::Result<battler_wamprat_message::Value, battler_wamprat_message::WampSerializeError> {
                let mut dict = battler_wamprat_message::Dictionary::default();
                #(#field_serializers)*
                Ok(battler_wamprat_message::Value::Dictionary(dict))
            }
        }
    };

    let named = input.fields.is_empty() || input.fields.iter().any(|field| field.ident.is_some());
    let struct_constructor = if named {
        quote!(#ident { #(#field_identifiers,)* })
    } else {
        quote!(#ident(#(#field_identifiers,)*))
    };
    let deserialize = quote! {
        impl battler_wamprat_message::WampDeserialize for #ident {
            fn wamp_deserialize(value: battler_wamprat_message::Value) -> core::result::Result<Self, battler_wamprat_message::WampDeserializeError> {
                let mut dict = match value {
                    battler_wamprat_message::Value::Dictionary(dict) => dict,
                    _ => return Err(battler_wamprat_message::WampDeserializeError::new("value must be a list")),
                };
                #(#field_deserializers)*
                Ok(#struct_constructor)
            }
        }
    };

    quote! {
        #serialize
        #deserialize
    }
    .into()
}

enum ApplicationMessageFieldType {
    Arguments,
    ArgumentsKeyword,
}

struct ApplicationMessageInputFieldAttrs {
    field_type: ApplicationMessageFieldType,
}

fn parse_application_message_input_field_attrs(
    field: &Field,
) -> syn::Result<ApplicationMessageInputFieldAttrs> {
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
            ))
        }
    };
    Ok(ApplicationMessageInputFieldAttrs { field_type })
}

struct ApplicationMessageInputField {
    ident: Option<Ident>,
    attrs: ApplicationMessageInputFieldAttrs,
}

struct ApplicationMessageInput {
    ident: Ident,
    fields: Vec<ApplicationMessageInputField>,
}

impl Parse for ApplicationMessageInput {
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
                Ok(ApplicationMessageInputField {
                    ident: field.ident,
                    attrs,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self { ident, fields })
    }
}

/// Procedural macro for deriving [`battler_wamprat_message::WampApplicationMessage`] for a struct.
#[proc_macro_derive(WampApplicationMessage, attributes(arguments, arguments_keyword))]
pub fn derive_wamp_application_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ApplicationMessageInput);
    let call_site = Span::call_site();

    let ident = input.ident;

    let (field_serializers, field_deserializers, field_identifiers): (Vec<_>, Vec<_>, Vec<_>) = input.fields.iter().enumerate().map(|(i, field)| {
        let accessor = match &field.ident {
            Some(ident) => quote!(self.#ident),
            None => quote!(self.#i),
        };
        let field_name = field.ident.clone().unwrap_or(Ident::new(&format!("field_{i}"), call_site));
        let input_output_ident = match field.attrs.field_type {
            ApplicationMessageFieldType::Arguments => quote!(arguments),
            ApplicationMessageFieldType::ArgumentsKeyword => quote!(arguments_keyword),
        };
        let validate_serialize_output = match field.attrs.field_type {
            ApplicationMessageFieldType::Arguments => quote! {
                match val {
                    battler_wamprat_message::Value::List(val) => val,
                    _ => return Err(battler_wamprat_message::WampSerializeError::new(std::fmt::format(format_args!("arguments of {} did not produce a list", std::stringify!(#ident))))),
                }
            },
            ApplicationMessageFieldType::ArgumentsKeyword => quote! {
                match val {
                    battler_wamprat_message::Value::Dictionary(val) => val,
                    _ => return Err(battler_wamprat_message::WampSerializeError::new(std::fmt::format(format_args!("arguments of {} did not produce a list", std::stringify!(#ident))))),
                }
            },
        };
        (
            quote! {
                let #input_output_ident = match battler_wamprat_message::WampSerialize::wamp_serialize(#accessor) {
                    Ok(val) => #validate_serialize_output,
                    Err(err) => return Err(err.annotate(std::fmt::format(format_args!("failed to serialize {} of {}", std::stringify!(#input_output_ident), std::stringify!(#ident))))),
                };
            },
            quote! {
                let #field_name = match battler_wamprat_message::WampDeserialize::wamp_deserialize(#input_output_ident) {
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
            fn wamp_serialize_application_message(self) -> core::result::Result<(battler_wamprat_message::List, battler_wamprat_message::Dictionary), battler_wamprat_message::WampSerializeError> {
                let arguments = battler_wamprat_message::List::default();
                let arguments_keyword = battler_wamprat_message::Dictionary::default();
                #(#field_serializers)*
                Ok((arguments, arguments_keyword))
            }

            fn wamp_deserialize_application_message(
                arguments: battler_wamprat_message::List,
                arguments_keyword: battler_wamprat_message::Dictionary,
            ) -> core::result::Result<Self, battler_wamprat_message::WampDeserializeError> {
                let arguments = battler_wamprat_message::Value::List(arguments);
                let arguments_keyword = battler_wamprat_message::Value::Dictionary(arguments_keyword);
                #(#field_deserializers)*
                Ok(#struct_constructor)
            }
        }
    }.into()
}
