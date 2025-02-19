use battler_wamp::core::uri::Uri;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Error,
    Ident,
    ItemEnum,
    LitStr,
    Path,
    Result,
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    spanned::Spanned,
};

#[allow(dead_code)]
enum UriAttribute {
    Uri(LitStr),
    Pattern(Path),
}

#[allow(dead_code)]
struct RpcAttribute {
    input: Path,
    output: Path,
    error: Option<Path>,
    uri: UriAttribute,
    progressive: bool,
}

#[allow(dead_code)]
struct PubSubAttribute {
    event: Path,
    uri: UriAttribute,
}

#[allow(dead_code)]
enum Attribute {
    Rpc(RpcAttribute),
    PubSub(PubSubAttribute),
}

#[allow(dead_code)]
struct Variant {
    span: Span,
    ident: Ident,
    attribute: Attribute,
}

#[allow(dead_code)]
struct Input {
    ident: Ident,
    realm: LitStr,
    variants: Vec<Variant>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let call_site = Span::call_site();
        let input =
            ItemEnum::parse(input).map_err(|_| Error::new(call_site, "input must be an enum"))?;
        let ident = input.ident;
        let realm = input
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("realm"))
            .map(|attr| attr.parse_args_with(|input: ParseStream| input.parse::<LitStr>()))
            .ok_or_else(|| Error::new(call_site, "missing realm attribute"))??;
        Uri::try_from(realm.value()).map_err(|_| Error::new(call_site, "invalid realm uri"))?;
        let variants = input
            .variants
            .into_iter()
            .map(|variant| {
                let span = variant.span();
                let ident = variant.ident;
                let rpc: Option<Result<RpcAttribute>> = variant
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("rpc"))
                    .map(|attr| {
                        let mut input = None;
                        let mut output = None;
                        let mut error = None;
                        let mut uri = None;
                        let mut progressive = false;
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("input") {
                                input = Some(meta.value()?.parse::<Path>()?);
                                Ok(())
                            } else if meta.path.is_ident("output") {
                                output = Some(meta.value()?.parse::<Path>()?);
                                Ok(())
                            } else if meta.path.is_ident("error") {
                                error = Some(meta.value()?.parse::<Path>()?);
                                Ok(())
                            } else if meta.path.is_ident("uri") {
                                uri = Some(UriAttribute::Uri(meta.value()?.parse::<LitStr>()?));
                                Ok(())
                            } else if meta.path.is_ident("pattern") {
                                uri = Some(UriAttribute::Pattern(meta.value()?.parse::<Path>()?));
                                Ok(())
                            } else if meta.path.is_ident("progressive") {
                                progressive = true;
                                Ok(())
                            } else {
                                Ok(())
                            }
                        })?;
                        Ok(RpcAttribute {
                            input: input
                                .ok_or_else(|| Error::new(span, "missing input attribute"))?,
                            output: output
                                .ok_or_else(|| Error::new(span, "missing output attribute"))?,
                            error,
                            uri: uri
                                .ok_or_else(|| Error::new(span, "missing uri/pattern attribute"))?,
                            progressive,
                        })
                    });
                let pub_sub: Option<Result<PubSubAttribute>> = variant
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("pubsub"))
                    .map(|attr| {
                        let mut event = None;
                        let mut uri = None;
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("event") {
                                event = Some(meta.value()?.parse::<Path>()?);
                                Ok(())
                            } else if meta.path.is_ident("uri") {
                                uri = Some(UriAttribute::Uri(meta.value()?.parse::<LitStr>()?));
                                Ok(())
                            } else if meta.path.is_ident("pattern") {
                                uri = Some(UriAttribute::Pattern(meta.value()?.parse::<Path>()?));
                                Ok(())
                            } else {
                                Ok(())
                            }
                        })?;
                        Ok(PubSubAttribute {
                            event: event
                                .ok_or_else(|| Error::new(span, "missing event attribute"))?,
                            uri: uri
                                .ok_or_else(|| Error::new(span, "missing uri/pattern attribute"))?,
                        })
                    });
                let attribute = match (rpc, pub_sub) {
                    (Some(_), Some(_)) | (None, None) => {
                        return Err(Error::new(span, "variant must be oneof rpc, pubsub"));
                    }
                    (Some(rpc), None) => Attribute::Rpc(rpc?),
                    (None, Some(pub_sub)) => Attribute::PubSub(pub_sub?),
                };
                Ok(Variant {
                    span,
                    ident,
                    attribute,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Input {
            ident,
            realm,
            variants,
        })
    }
}

/// Procedural macro for generating strongly-typed producer and consumer services around a
/// `battler_wamprat::Peer`.
#[proc_macro_derive(WampSchema, attributes(realm, rpc, pubsub))]
pub fn derive_wamp_uri_matcher(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[allow(unused)]
    let input = parse_macro_input!(input as Input);
    quote! {}.into()
}
