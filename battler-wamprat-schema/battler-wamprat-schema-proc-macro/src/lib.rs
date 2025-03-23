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

fn variant_to_function_name(s: &str) -> String {
    let name = s
        .chars()
        .map(|c| {
            if c.is_uppercase() {
                format!("_{}", c.to_lowercase())
            } else {
                format!("{c}")
            }
        })
        .collect::<Vec<_>>()
        .join("");
    if let Some('_') = name.chars().nth(0) {
        name[1..].to_owned()
    } else {
        name
    }
}

/// Procedural macro for generating strongly-typed producer and consumer services around a
/// [`battler_wamprat::peer::Peer`].
#[proc_macro_derive(WampSchema, attributes(realm, rpc, pubsub))]
pub fn derive_wamp_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[allow(unused)]
    let input = parse_macro_input!(input as Input);

    let ident = &input.ident;
    let realm = &input.realm;

    let peer = quote!(self.__peer_handle);
    let peer_builder = quote!(self.__peer_builder);

    let subscriptions = input.variants.iter().map(|variant| match &variant.attribute {
        Attribute::Rpc(_) => quote!(),
        Attribute::PubSub(pubsub) => {
            let variant_ident = &variant.ident;
            let name = Ident::new(&format!("{variant_ident}Subscription"), variant.span);
            let event = &pubsub.event;
            let underlying = match &pubsub.uri {
                UriAttribute::Uri(_) => quote!(::battler_wamprat::subscription::TypedSubscription<Event = #event>),
                UriAttribute::Pattern(pattern) => quote!(::battler_wamprat::subscription::TypedPatternMatchedSubscription<Pattern = #pattern, Event = #event>),
            };
            quote! {
                #[doc = "Subscription for handling events of the"]
                #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                #[doc = "topic."]
                pub trait #name: #underlying {}
            }
        }
    }).collect::<Vec<_>>();

    let consumer_methods = input
        .variants
        .iter()
        .map(|variant| match &variant.attribute {
            Attribute::Rpc(rpc) => {
                let variant_ident = &variant.ident;
                let name = variant_to_function_name(&variant_ident.to_string());
                let name = Ident::new(&name, variant.span);
                let input = &rpc.input;
                let output = &rpc.output;
                let error = &rpc.error;
                let error = match error {
                    Some(error) => quote!(#error),
                    None => quote!(::anyhow::Error),
                };
                let (uri_input, uri_arg) = match &rpc.uri {
                    UriAttribute::Uri(uri) => (quote!(), quote!(::battler_wamp::core::uri::Uri::try_from(#uri)?)),
                    UriAttribute::Pattern(pattern) => (quote!(uri: #pattern,), quote!(uri.wamp_generate_uri()?)),
                };
                let (output_rpc, method) = if rpc.progressive {
                    (quote!(::battler_wamprat_schema::ProgressivePendingRpc), quote!(call_with_progress))
                } else {
                    (quote!(::battler_wamprat_schema::SimplePendingRpc), quote!(call))
                };
                quote! {
                    #[doc = "Calls the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "procedure."]
                    pub async fn #name(&self, #uri_input input: #input, call_options: ::battler_wamprat::peer::CallOptions) -> ::anyhow::Result<#output_rpc<#output, #error>> {
                        Ok(#peer.#method::<#input, #output>(#uri_arg, input, call_options).await?.into())
                    }
                }
            }
            Attribute::PubSub(pubsub) => {
                let variant_ident = &variant.ident;
                let name = variant_to_function_name(&variant_ident.to_string());
                let subscribe_name = Ident::new(&format!("subscribe_{name}"), variant.span);
                let unsubscribe_name = Ident::new(&format!("unsubscribe_{name}"), variant.span);
                let (subscribe_method_call, unsubscribe_method_call) = match &pubsub.uri {
                    UriAttribute::Uri(uri) => (quote!(subscribe(::battler_wamp::core::uri::Uri::try_from(#uri)?, subscription)), quote!(unsubscribe(&::battler_wamp::core::uri::WildcardUri::try_from(#uri)?))),
                    UriAttribute::Pattern(pattern) => (quote!(subscribe_pattern_matched(subscription)), quote!(unsubscribe(&#pattern::uri_for_router()))),
                };
                let subscription_type = Ident::new(&format!("{variant_ident}Subscription"), variant.span);
                quote! {
                    #[doc = "Subscribes to the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "topic."]
                    pub async fn #subscribe_name<T>(&self, subscription: T) -> ::anyhow::Result<()> where T: #subscription_type + 'static {
                        #peer.#subscribe_method_call.await
                    }

                    #[doc = "Unsubscribes from the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "topic."]
                    pub async fn #unsubscribe_name(&self) -> ::anyhow::Result<()> {
                        #peer.#unsubscribe_method_call.await
                    }
                }
            },
        })
        .collect::<Vec<_>>();

    let procedures = input
        .variants
        .iter()
        .map(|variant| match &variant.attribute {
            Attribute::Rpc(rpc) => {
                let variant_ident = &variant.ident;
                let name = Ident::new(&format!("{variant_ident}Procedure"), variant.span);
                let input = &rpc.input;
                let output = &rpc.output;
                let error = &rpc.error;
                let error = match error {
                    Some(error) => quote!(#error),
                    None => quote!(::anyhow::Error),
                };
                let underlying = match &rpc.uri {
                    UriAttribute::Uri(_) => if rpc.progressive {
                        quote!(::battler_wamprat::procedure::TypedProgressiveProcedure<Input = #input, Output = #output, Error = #error>)

                    } else {
                        quote!(::battler_wamprat::procedure::TypedProcedure<Input = #input, Output = #output, Error = #error>)
                    }
                    UriAttribute::Pattern(pattern) => if rpc.progressive {
                        quote!(::battler_wamprat::procedure::TypedPatternMatchedProgressiveProcedure<Pattern = #pattern, Input = #input, Output = #output, Error = #error>)

                    } else {
                        quote!(::battler_wamprat::procedure::TypedPatternMatchedProcedure<Pattern = #pattern, Input = #input, Output = #output, Error = #error>)
                    }
                };
                quote! {
                    #[doc = "Procedure for handling invocations of the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "procedure."]
                    pub trait #name: #underlying {}
                }
            }
            Attribute::PubSub(_) => quote!(),
        })
        .collect::<Vec<_>>();

    let producer_builder_methods = input
        .variants
        .iter()
        .map(|variant| match &variant.attribute {
            Attribute::Rpc(rpc) => {
                let variant_ident = &variant.ident;
                let name = variant_to_function_name(&variant_ident.to_string());
                let name = Ident::new(&format!("register_{name}"), variant.span);
                let method_call = match &rpc.uri {
                    UriAttribute::Uri(uri) => if rpc.progressive {
                        quote!(add_procedure_progressive(::battler_wamp::core::uri::Uri::try_from(#uri)?, procedure))

                    } else {
                        quote!(add_procedure(::battler_wamp::core::uri::Uri::try_from(#uri)?, procedure))
                    }
                    UriAttribute::Pattern(_) => if rpc.progressive {
                        quote!(add_procedure_pattern_matched_progressive(procedure))
                    } else {
                        quote!(add_procedure_pattern_matched(procedure))
                    }
                };
                let procedure_type = Ident::new(&format!("{variant_ident}Procedure"), variant.span);
                quote! {
                    #[doc = "Registers a procedure for invocations to the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "procedure."]
                    pub fn #name<T>(&mut self, procedure: T) -> ::anyhow::Result<()> where T: #procedure_type + 'static {
                        #peer_builder.#method_call;
                        Ok(())
                    }
                }
            }
            Attribute::PubSub(_) => quote!(),
        })
        .collect::<Vec<_>>();

    let producer_methods = input
        .variants
        .iter()
        .map(|variant| match &variant.attribute {
            Attribute::Rpc(_) => quote!(),
            Attribute::PubSub(pubsub) => {
                let variant_ident = &variant.ident;
                let name = variant_to_function_name(&variant_ident.to_string());
                let name = Ident::new(&format!("publish_{name}"), variant.span);
                let event = &pubsub.event;
                let (uri_input, uri_arg) = match &pubsub.uri {
                    UriAttribute::Uri(uri) => (
                        quote!(),
                        quote!(::battler_wamp::core::uri::Uri::try_from(#uri)?),
                    ),
                    UriAttribute::Pattern(pattern) => {
                        (quote!(uri: #pattern,), quote!(uri.wamp_generate_uri()?))
                    }
                };
                quote! {
                    #[doc = "Publishes an event to the"]
                    #[doc = concat!("[`", stringify!(#ident), "::", stringify!(#variant_ident), "`]")]
                    #[doc = "topic."]
                    pub async fn #name(&self, #uri_input event: #event) -> ::anyhow::Result<()> {
                        #peer.publish(#uri_arg, event).await
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let consumer = Ident::new(&format!("{ident}Consumer"), Span::call_site());
    let producer = Ident::new(&format!("{ident}Producer"), Span::call_site());
    let producer_builder = Ident::new(&format!("{producer}Builder"), producer.span());

    quote! {
        #(#subscriptions)*

        #(#procedures)*

        #[doc = "A consumer (client) of the"]
        #[doc = concat!("[`", stringify!(#ident), "`]")]
        #[doc = "service."]
        pub struct #consumer<S> {
            __peer_handle: ::battler_wamprat::peer::PeerHandle<S>,
            __join_handle: ::tokio::task::JoinHandle<()>,
        }

        impl<S> #consumer<S> where S: Send + 'static {
            fn new(config: ::battler_wamprat_schema::PeerConfig, peer: ::battler_wamp::peer::Peer<S>) -> ::anyhow::Result<Self> {
                let mut peer_builder = ::battler_wamprat::peer::PeerBuilder::new(config.connection.connection_type.clone());
                *peer_builder.connection_config_mut() = config.connection;
                peer_builder.set_auth_methods(config.auth_methods);
                let (peer_handle, join_handle) = peer_builder.start(
                    peer,
                    ::battler_wamp::core::uri::Uri::try_from(#realm)?,
                );
                Ok(Self { __peer_handle: peer_handle, __join_handle: join_handle })
            }

            #[doc = "Cancels and waits for the peer to be fully cleaned up by joining the asynchronous task."]
            pub async fn stop(self) -> ::core::result::Result<(), ::anyhow::Error> {
                #peer.cancel()?;
                self.__join_handle.await.map_err(|err| err.into())
            }

            #[doc = "Waits until the consumer is known to be in a ready state."]
            pub async fn wait_until_ready(&self) -> ::core::result::Result<(), ::anyhow::Error> {
                #peer.wait_until_ready().await
            }

            #(#consumer_methods)*
        }

        #[doc = "A producer (server) of the"]
        #[doc = concat!("[`", stringify!(#ident), "`]")]
        #[doc = "service."]
        pub struct #producer<S> {
            __peer_handle: ::battler_wamprat::peer::PeerHandle<S>,
            __join_handle: ::tokio::task::JoinHandle<()>,
        }

        impl<S> #producer<S> where S: Send + 'static {
            #[doc = "Cancels and waits for the peer to be fully cleaned up by joining the asynchronous task."]
            pub async fn stop(self) -> ::core::result::Result<(), ::anyhow::Error> {
                #peer.cancel()?;
                self.__join_handle.await.map_err(|err| err.into())
            }

            #[doc = "Waits until the producer is known to be in a ready state."]
            pub async fn wait_until_ready(&self) -> ::core::result::Result<(), ::anyhow::Error> {
                #peer.wait_until_ready().await
            }

            #(#producer_methods)*
        }

        #[doc = "A builder for a"]
        #[doc = concat!("[`", stringify!(#producer), "`]")]
        #[doc = "service producer."]
        pub struct #producer_builder {
            __peer_builder: ::battler_wamprat::peer::PeerBuilder,
        }

        impl #producer_builder {
            fn new(config: ::battler_wamprat_schema::PeerConfig) -> Self {
                let mut peer_builder = ::battler_wamprat::peer::PeerBuilder::new(config.connection.connection_type.clone());
                *peer_builder.connection_config_mut() = config.connection;
                peer_builder.set_auth_methods(config.auth_methods);
                Self { __peer_builder: peer_builder }
            }

            #[doc = "Starts the producer on the given peer."]
            pub fn start<S>(self, peer: ::battler_wamp::peer::Peer<S>) -> ::anyhow::Result<#producer<S>> where S: Send + 'static {
                let (peer_handle, join_handle) = #peer_builder.start(peer, ::battler_wamp::core::uri::Uri::try_from(#realm)?);
                Ok(#producer { __peer_handle: peer_handle, __join_handle: join_handle })
            }

            #(#producer_builder_methods)*
        }

        impl #ident {
            #[doc = "Creates a peer that consumes the"]
            #[doc = concat!("[`", stringify!(#ident), "`]")]
            #[doc = "service."]
            pub fn consumer<S>(config: ::battler_wamprat_schema::PeerConfig, peer: ::battler_wamp::peer::Peer<S>) -> ::anyhow::Result<#consumer<S>> where S: Send + 'static {
                #consumer::<S>::new(config, peer)
            }

            #[doc = "Creates a peer builder for a producer of the"]
            #[doc = concat!("[`", stringify!(#ident), "`]")]
            #[doc = "service."]
            pub fn producer_builder(config: ::battler_wamprat_schema::PeerConfig) -> #producer_builder {
                #producer_builder::new(config)
            }
        }
    }
    .into()
}
