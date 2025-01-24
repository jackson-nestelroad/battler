use battler_wamp::core::uri::Uri;
use proc_macro2::{
    Ident,
    Span,
    TokenStream,
};
use quote::quote;
use syn::{
    Data,
    DeriveInput,
    Error,
    Fields,
    LitStr,
    Result,
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    spanned::Spanned,
};

struct StructInput {
    ident: Ident,
    uri: LitStr,
    fields: Fields,
}

struct EnumVariant {
    ident: Ident,
    uri: LitStr,
    fields: Fields,
    span: Span,
}

struct EnumInput {
    ident: Ident,
    variants: Vec<EnumVariant>,
}

enum Input {
    Struct(StructInput),
    Enum(EnumInput),
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let call_site = Span::call_site();
        let input = match DeriveInput::parse(input) {
            Ok(item) => item,
            Err(_) => return Err(Error::new(call_site, "input must be derive macro input")),
        };
        match input.data {
            Data::Struct(data) => {
                let ident = input.ident;
                let uri = input
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("uri"))
                    .and_then(|attr| {
                        Some(attr.parse_args_with(|input: ParseStream| input.parse::<LitStr>()))
                    })
                    .ok_or_else(|| Error::new(call_site, "missing uri attribute"))??;
                let fields = data.fields;
                Ok(Self::Struct(StructInput { ident, uri, fields }))
            }
            Data::Enum(data) => {
                let ident = input.ident;
                let variants =
                    data.variants
                        .into_iter()
                        .map(|variant| {
                            let span = variant.span();
                            let ident = variant.ident;
                            let uri = variant
                                .attrs
                                .iter()
                                .find(|attr| attr.path().is_ident("uri"))
                                .and_then(|attr| {
                                    Some(attr.parse_args_with(|input: ParseStream| {
                                        input.parse::<LitStr>()
                                    }))
                                })
                                .ok_or_else(|| Error::new(span, "missing uri attribute"))??;
                            let fields = variant.fields;
                            Ok(EnumVariant {
                                ident,
                                uri,
                                fields,
                                span,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                Ok(Self::Enum(EnumInput { ident, variants }))
            }
            Data::Union(_) => return Err(Error::new(call_site, "macro not allowed on a union")),
        }
    }
}

fn construct_from_fields(ty: TokenStream, fields: &Fields, span: Span) -> Result<TokenStream> {
    match fields {
        Fields::Named(fields) => {
            if fields.named.len() > 1 {
                return Err(Error::new(
                    span,
                    "struct must be constructible from a string",
                ));
            }
            match fields.named.get(0) {
                Some(field) => {
                    let ident = field.ident.as_ref().unwrap();
                    Ok(quote! {
                        Ok(#ty { #ident: value.message().into() })
                    })
                }
                None => Ok(quote! { Ok(#ty {})}),
            }
        }
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() > 1 {
                return Err(Error::new(
                    span,
                    "struct must be constructible from a string",
                ));
            }
            if fields.unnamed.len() == 1 {
                Ok(quote! {
                    Ok(#ty(value.message().into()))
                })
            } else {
                Ok(quote! {
                    Ok(#ty())
                })
            }
        }
        Fields::Unit => Ok(quote! { Ok(#ty) }),
    }
}

/// Procedural macro for generating conversions to and from
/// [`battler_wamp::core::error::WampError`].
#[proc_macro_derive(WampError, attributes(uri))]
pub fn derive_wamp_uri_matcher(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Input);
    let call_site = Span::call_site();
    let ident = match &input {
        Input::Struct(input) => input.ident.clone(),
        Input::Enum(input) => input.ident.clone(),
    };

    match &input {
        Input::Struct(input) => {
            if Uri::try_from(input.uri.value()).is_err() {
                return proc_macro::TokenStream::from(
                    Error::new(call_site, "invalid uri").into_compile_error(),
                );
            }
        }
        Input::Enum(input) => {
            for variant in &input.variants {
                if Uri::try_from(variant.uri.value()).is_err() {
                    return proc_macro::TokenStream::from(
                        Error::new(variant.span, "invalid uri").into_compile_error(),
                    );
                }
            }
        }
    }

    let into = match &input {
        Input::Struct(input) => {
            let uri = &input.uri;
            quote! {
                ::battler_wamp::core::error::WampError::new(::battler_wamp::core::uri::Uri::try_from(#uri).unwrap(), self.to_string())
            }
        }
        Input::Enum(input) => {
            let variants = input.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let uri = &variant.uri;
                quote! {
                    Self::#ident { .. } => ::battler_wamp::core::error::WampError::new(::battler_wamp::core::uri::Uri::try_from(#uri).unwrap(), self.to_string())
                }
            });
            quote! {
                match self {
                    #(#variants),*
                }
            }
        }
    };

    let try_from = match &input {
        Input::Struct(input) => {
            let constructor = match construct_from_fields(quote!(Self), &input.fields, call_site) {
                Ok(constructor) => constructor,
                Err(err) => return proc_macro::TokenStream::from(err.into_compile_error()),
            };
            let uri = &input.uri;
            quote! {
                if value.reason().as_ref() == #uri {
                    #constructor
                } else {
                    Err(value)
                }
            }
        }
        Input::Enum(input) => {
            let variant_matchers = match input
                .variants
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let constructor =
                        construct_from_fields(quote!(Self::#ident), &variant.fields, call_site)?;
                    let uri = &variant.uri;
                    Ok(quote! {
                        if value.reason().as_ref() == #uri {
                            return #constructor;
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()
            {
                Ok(variant_matchers) => variant_matchers,
                Err(err) => return proc_macro::TokenStream::from(err.into_compile_error()),
            };
            quote! {
                #(#variant_matchers)*
                Err(value)
            }
        }
    };

    quote! {
        impl ::core::convert::Into<::battler_wamp::core::error::WampError> for #ident where #ident: ::std::string::ToString {
            fn into(self) -> ::battler_wamp::core::error::WampError {
                #into
            }
        }

        impl ::core::convert::TryFrom<::battler_wamp::core::error::WampError> for #ident {
            type Error = ::battler_wamp::core::error::WampError;
            fn try_from(value: ::battler_wamp::core::error::WampError) -> ::core::result::Result<Self, Self::Error> {
                #try_from
            }
        }
    }
    .into()
}
