use std::{
    marker::PhantomData,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::WampError,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    peer::{
        Invocation as NativeInvocation,
        RpcYield,
        SupportedAuthMethod,
    },
};
use battler_wamp_values::{
    Dictionary,
    List,
};
use tokio::task::JoinHandle;

use crate::{
    error::{
        WampratDeserializeError,
        WampratInvocationMissingProcedure,
        WampratSerializeError,
    },
    peer::{
        Peer,
        PeerConnectionConfig,
        PeerConnectionType,
        PeerHandle,
        peer::PreregisteredProcedure,
    },
    procedure::{
        Invocation,
        Procedure,
        ProgressReporter,
        TypedPatternMatchedProcedure,
        TypedPatternMatchedProgressiveProcedure,
        TypedProcedure,
        TypedProgressiveProcedure,
    },
};

/// An object for building a [`Peer`][`crate::peer::Peer`].
pub struct PeerBuilder {
    connection_config: PeerConnectionConfig,
    auth_methods: Vec<SupportedAuthMethod>,
    procedures: ahash::HashMap<WildcardUri, PreregisteredProcedure>,
}

impl PeerBuilder {
    /// Creates a new [`PeerBuilder`].
    pub fn new(connection_type: PeerConnectionType) -> Self {
        Self {
            connection_config: PeerConnectionConfig::new(connection_type),
            auth_methods: Vec::default(),
            procedures: ahash::HashMap::default(),
        }
    }

    /// The [`PeerConnectionConfig`] used to connect to the router.
    pub fn connection_config_mut(&mut self) -> &mut PeerConnectionConfig {
        &mut self.connection_config
    }

    /// Sets the supported authentication methods for connecting to the realm.
    pub fn set_auth_methods(
        &mut self,
        auth_methods: impl IntoIterator<Item = SupportedAuthMethod>,
    ) {
        self.auth_methods = auth_methods.into_iter().collect();
    }

    /// Adds a new strongly-typed procedure, which will be registered on every new connection to a
    /// router.
    pub fn add_procedure<T, Input, Output, Error>(&mut self, uri: Uri, procedure: T)
    where
        T: TypedProcedure<Input = Input, Output = Output, Error = Error> + 'static,
        Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Error: Into<WampError> + Send + Sync + 'static,
    {
        // Wrap the typed procedure with a generic wrapper that serializes and deserializes
        // application messages.
        struct ProcedureWrapper<T, Input, Output, Error> {
            procedure: T,
            _input: PhantomData<Input>,
            _output: PhantomData<Output>,
            _error: PhantomData<Error>,
        }

        impl<T, Input, Output, Error> ProcedureWrapper<T, Input, Output, Error>
        where
            T: TypedProcedure<Input = Input, Output = Output, Error = Error>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            fn new(procedure: T) -> Self {
                Self {
                    procedure,
                    _input: PhantomData,
                    _output: PhantomData,
                    _error: PhantomData,
                }
            }

            async fn invoke_internal(
                &self,
                invocation: Invocation,
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
            ) -> Result<RpcYield, WampError> {
                let input =
                    Input::wamp_deserialize_application_message(arguments, arguments_keyword)
                        .map_err(Into::<WampratDeserializeError>::into)
                        .map_err(Into::<WampError>::into)?;

                let output = self
                    .procedure
                    .invoke(invocation, input)
                    .await
                    .map_err(|err| Into::<WampError>::into(err))?;
                let (arguments, arguments_keyword) = output
                    .wamp_serialize_application_message()
                    .map_err(Into::<WampratSerializeError>::into)
                    .map_err(Into::<WampError>::into)?;
                Ok(RpcYield {
                    arguments,
                    arguments_keyword,
                })
            }
        }

        #[async_trait]
        impl<T, Input, Output, Error> Procedure for ProcedureWrapper<T, Input, Output, Error>
        where
            T: TypedProcedure<Input = Input, Output = Output, Error = Error>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            async fn invoke(&self, mut invocation: NativeInvocation) -> Result<()> {
                let mut arguments = List::default();
                let mut arguments_keyword = Dictionary::default();
                std::mem::swap(&mut invocation.arguments, &mut arguments);
                std::mem::swap(&mut invocation.arguments_keyword, &mut arguments_keyword);

                let mut inner_invocation = Invocation::default();
                std::mem::swap(&mut invocation.timeout, &mut inner_invocation.timeout);
                std::mem::swap(&mut invocation.procedure, &mut inner_invocation.procedure);
                std::mem::swap(&mut invocation.identity, &mut inner_invocation.identity);

                let result = self
                    .invoke_internal(inner_invocation, arguments, arguments_keyword)
                    .await;
                invocation.respond(result).await
            }
        }

        let options = T::options();
        self.procedures.insert(
            uri.into(),
            PreregisteredProcedure {
                procedure: Arc::new(Box::new(ProcedureWrapper::new(procedure))),
                ignore_registration_error: false,
                match_style: None,
                invocation_policy: options.invocation_policy,
            },
        );
    }

    /// Adds a new strongly-typed, pattern-matched procedure, which will be registered on every new
    /// connection to a router.
    pub fn add_procedure_pattern_matched<T, Pattern, Input, Output, Error>(&mut self, procedure: T)
    where
        T: TypedPatternMatchedProcedure<
                Pattern = Pattern,
                Input = Input,
                Output = Output,
                Error = Error,
            > + 'static,
        Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
        Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Error: Into<WampError> + Send + Sync + 'static,
    {
        // Wrap the typed procedure with a generic wrapper that serializes and deserializes
        // application messages.
        struct ProcedureWrapper<T, Pattern, Input, Output, Error> {
            procedure: T,
            _pattern: PhantomData<Pattern>,
            _input: PhantomData<Input>,
            _output: PhantomData<Output>,
            _error: PhantomData<Error>,
        }

        impl<T, Pattern, Input, Output, Error> ProcedureWrapper<T, Pattern, Input, Output, Error>
        where
            T: TypedPatternMatchedProcedure<
                    Pattern = Pattern,
                    Input = Input,
                    Output = Output,
                    Error = Error,
                >,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            fn new(procedure: T) -> Self {
                Self {
                    procedure,
                    _pattern: PhantomData,
                    _input: PhantomData,
                    _output: PhantomData,
                    _error: PhantomData,
                }
            }

            async fn invoke_internal(
                &self,
                invocation: Invocation,
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
            ) -> Result<RpcYield, WampError> {
                let input =
                    Input::wamp_deserialize_application_message(arguments, arguments_keyword)
                        .map_err(Into::<WampratDeserializeError>::into)
                        .map_err(Into::<WampError>::into)?;
                let procedure = Pattern::wamp_match_uri(
                    invocation
                        .procedure
                        .clone()
                        .ok_or_else(|| WampratInvocationMissingProcedure.into())?
                        .as_ref(),
                )
                .map_err(Into::<WampError>::into)?;
                let output = self
                    .procedure
                    .invoke(invocation, input, procedure)
                    .await
                    .map_err(|err| Into::<WampError>::into(err))?;
                let (arguments, arguments_keyword) = output
                    .wamp_serialize_application_message()
                    .map_err(Into::<WampratSerializeError>::into)
                    .map_err(Into::<WampError>::into)?;
                Ok(RpcYield {
                    arguments,
                    arguments_keyword,
                })
            }
        }

        #[async_trait]
        impl<T, Pattern, Input, Output, Error> Procedure
            for ProcedureWrapper<T, Pattern, Input, Output, Error>
        where
            T: TypedPatternMatchedProcedure<
                    Pattern = Pattern,
                    Input = Input,
                    Output = Output,
                    Error = Error,
                >,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            async fn invoke(&self, mut invocation: NativeInvocation) -> Result<()> {
                let mut arguments = List::default();
                let mut arguments_keyword = Dictionary::default();
                std::mem::swap(&mut invocation.arguments, &mut arguments);
                std::mem::swap(&mut invocation.arguments_keyword, &mut arguments_keyword);

                let mut inner_invocation = Invocation::default();
                std::mem::swap(&mut invocation.timeout, &mut inner_invocation.timeout);
                std::mem::swap(&mut invocation.procedure, &mut inner_invocation.procedure);
                std::mem::swap(&mut invocation.identity, &mut inner_invocation.identity);

                let result = self
                    .invoke_internal(inner_invocation, arguments, arguments_keyword)
                    .await;
                invocation.respond(result).await
            }
        }

        let options = T::options();
        self.procedures.insert(
            Pattern::uri_for_router(),
            PreregisteredProcedure {
                procedure: Arc::new(Box::new(ProcedureWrapper::new(procedure))),
                ignore_registration_error: false,
                match_style: Pattern::match_style(),
                invocation_policy: options.invocation_policy,
            },
        );
    }

    /// Adds a new strongly-typed, progressive procedure, which will be registered on every new
    /// connection to a router.
    pub fn add_procedure_progressive<T, Input, Output, Error>(&mut self, uri: Uri, procedure: T)
    where
        T: TypedProgressiveProcedure<Input = Input, Output = Output, Error = Error> + 'static,
        Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Error: Into<WampError> + Send + Sync + 'static,
    {
        // Wrap the typed procedure with a generic wrapper that serializes and deserializes
        // application messages.
        struct ProcedureWrapper<T, Input, Output, Error> {
            procedure: T,
            _input: PhantomData<Input>,
            _output: PhantomData<Output>,
            _error: PhantomData<Error>,
        }

        impl<T, Input, Output, Error> ProcedureWrapper<T, Input, Output, Error>
        where
            T: TypedProgressiveProcedure<Input = Input, Output = Output, Error = Error>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            fn new(procedure: T) -> Self {
                Self {
                    procedure,
                    _input: PhantomData,
                    _output: PhantomData,
                    _error: PhantomData,
                }
            }

            async fn invoke_internal(
                &self,
                invocation: Invocation,
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
                native_invocation: &NativeInvocation,
            ) -> Result<RpcYield, WampError> {
                let input =
                    Input::wamp_deserialize_application_message(arguments, arguments_keyword)
                        .map_err(Into::<WampratDeserializeError>::into)
                        .map_err(Into::<WampError>::into)?;
                let output = self
                    .procedure
                    .invoke(invocation, input, ProgressReporter::new(&native_invocation))
                    .await
                    .map_err(|err| Into::<WampError>::into(err))?;
                let (arguments, arguments_keyword) = output
                    .wamp_serialize_application_message()
                    .map_err(Into::<WampratSerializeError>::into)
                    .map_err(Into::<WampError>::into)?;
                Ok(RpcYield {
                    arguments,
                    arguments_keyword,
                })
            }
        }

        #[async_trait]
        impl<T, Input, Output, Error> Procedure for ProcedureWrapper<T, Input, Output, Error>
        where
            T: TypedProgressiveProcedure<Input = Input, Output = Output, Error = Error>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            async fn invoke(&self, mut invocation: NativeInvocation) -> Result<()> {
                let mut arguments = List::default();
                let mut arguments_keyword = Dictionary::default();
                std::mem::swap(&mut invocation.arguments, &mut arguments);
                std::mem::swap(&mut invocation.arguments_keyword, &mut arguments_keyword);

                let mut inner_invocation = Invocation::default();
                std::mem::swap(&mut invocation.timeout, &mut inner_invocation.timeout);
                std::mem::swap(&mut invocation.procedure, &mut inner_invocation.procedure);
                std::mem::swap(&mut invocation.identity, &mut inner_invocation.identity);

                let result = self
                    .invoke_internal(inner_invocation, arguments, arguments_keyword, &invocation)
                    .await;
                invocation.respond(result).await
            }
        }

        let options = T::options();
        self.procedures.insert(
            uri.into(),
            PreregisteredProcedure {
                procedure: Arc::new(Box::new(ProcedureWrapper::new(procedure))),
                ignore_registration_error: false,
                match_style: None,
                invocation_policy: options.invocation_policy,
            },
        );
    }

    /// Adds a new strongly-typed, pattern-matched, progressive procedure, which will be registered
    /// on every new connection to a router.
    pub fn add_procedure_pattern_matched_progressive<T, Pattern, Input, Output, Error>(
        &mut self,
        procedure: T,
    ) where
        T: TypedPatternMatchedProgressiveProcedure<
                Pattern = Pattern,
                Input = Input,
                Output = Output,
                Error = Error,
            > + 'static,
        Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
        Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Error: Into<WampError> + Send + Sync + 'static,
    {
        // Wrap the typed procedure with a generic wrapper that serializes and deserializes
        // application messages.
        struct ProcedureWrapper<T, Pattern, Input, Output, Error> {
            procedure: T,
            _pattern: PhantomData<Pattern>,
            _input: PhantomData<Input>,
            _output: PhantomData<Output>,
            _error: PhantomData<Error>,
        }

        impl<T, Pattern, Input, Output, Error> ProcedureWrapper<T, Pattern, Input, Output, Error>
        where
            T: TypedPatternMatchedProgressiveProcedure<
                    Pattern = Pattern,
                    Input = Input,
                    Output = Output,
                    Error = Error,
                >,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            fn new(procedure: T) -> Self {
                Self {
                    procedure,
                    _pattern: PhantomData,
                    _input: PhantomData,
                    _output: PhantomData,
                    _error: PhantomData,
                }
            }

            async fn invoke_internal(
                &self,
                invocation: Invocation,
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
                native_invocation: &NativeInvocation,
            ) -> Result<RpcYield, WampError> {
                let input =
                    Input::wamp_deserialize_application_message(arguments, arguments_keyword)
                        .map_err(Into::<WampratDeserializeError>::into)
                        .map_err(Into::<WampError>::into)?;
                let procedure = Pattern::wamp_match_uri(
                    invocation
                        .procedure
                        .clone()
                        .ok_or_else(|| WampratInvocationMissingProcedure.into())?
                        .as_ref(),
                )
                .map_err(Into::<WampError>::into)?;
                let output = self
                    .procedure
                    .invoke(
                        invocation,
                        input,
                        procedure,
                        ProgressReporter::new(&native_invocation),
                    )
                    .await
                    .map_err(|err| Into::<WampError>::into(err))?;
                let (arguments, arguments_keyword) = output
                    .wamp_serialize_application_message()
                    .map_err(Into::<WampratSerializeError>::into)
                    .map_err(Into::<WampError>::into)?;
                Ok(RpcYield {
                    arguments,
                    arguments_keyword,
                })
            }
        }

        #[async_trait]
        impl<T, Pattern, Input, Output, Error> Procedure
            for ProcedureWrapper<T, Pattern, Input, Output, Error>
        where
            T: TypedPatternMatchedProgressiveProcedure<
                    Pattern = Pattern,
                    Input = Input,
                    Output = Output,
                    Error = Error,
                >,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync,
            Error: Into<WampError> + Send + Sync,
        {
            async fn invoke(&self, mut invocation: NativeInvocation) -> Result<()> {
                let mut arguments = List::default();
                let mut arguments_keyword = Dictionary::default();
                std::mem::swap(&mut invocation.arguments, &mut arguments);
                std::mem::swap(&mut invocation.arguments_keyword, &mut arguments_keyword);

                let mut inner_invocation = Invocation::default();
                std::mem::swap(&mut invocation.timeout, &mut inner_invocation.timeout);
                std::mem::swap(&mut invocation.procedure, &mut inner_invocation.procedure);
                std::mem::swap(&mut invocation.identity, &mut inner_invocation.identity);

                let result = self
                    .invoke_internal(inner_invocation, arguments, arguments_keyword, &invocation)
                    .await;
                invocation.respond(result).await
            }
        }

        let options = T::options();
        self.procedures.insert(
            Pattern::uri_for_router(),
            PreregisteredProcedure {
                procedure: Arc::new(Box::new(ProcedureWrapper::new(procedure))),
                ignore_registration_error: false,
                match_style: Pattern::match_style(),
                invocation_policy: options.invocation_policy,
            },
        );
    }

    /// Builds and starts a new [`Peer`] object in an asynchronous task, which can be managed
    /// through the returned [`PeerHandle`].
    pub fn start<S>(
        self,
        peer: battler_wamp::peer::Peer<S>,
        realm: Uri,
    ) -> (PeerHandle<S>, JoinHandle<()>)
    where
        S: Send + 'static,
    {
        Peer::new(
            peer,
            self.connection_config,
            realm,
            self.auth_methods.into_iter(),
            self.procedures.into_iter(),
        )
        .start()
    }
}
