use std::{
    marker::PhantomData,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::uri::Uri,
    peer::{
        Invocation,
        RpcYield,
    },
};
use battler_wamp_values::{
    Dictionary,
    List,
};
use tokio::task::JoinHandle;

use crate::{
    peer::{
        peer::PreregisteredProcedure,
        Peer,
        PeerConnectionConfig,
        PeerConnectionType,
        PeerHandle,
    },
    procedure::{
        Procedure,
        TypedProcedure,
    },
};

/// An object for building a [`Peer`][`crate::peer::Peer`].
pub struct PeerBuilder {
    connection_config: PeerConnectionConfig,
    procedures: ahash::HashMap<Uri, PreregisteredProcedure>,
}

impl PeerBuilder {
    /// Creates a new [`PeerBuilder`].
    pub fn new(connection_type: PeerConnectionType) -> Self {
        Self {
            connection_config: PeerConnectionConfig::new(connection_type),
            procedures: ahash::HashMap::default(),
        }
    }

    /// The [`PeerConnectionConfig`] used to connect to the router.
    pub fn connection_config_mut(&mut self) -> &mut PeerConnectionConfig {
        &mut self.connection_config
    }

    /// Adds a new strongly-typed procedure, which will be registered on every new connection to a
    /// router.
    pub fn add_procedure_typed<T, Input, Output>(&mut self, uri: Uri, procedure: T)
    where
        T: TypedProcedure<Input = Input, Output = Output> + 'static,
        Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        // Wrap the typed procedure with a generic wrapper that serializes and deserializes
        // application messages.
        struct ProcedureWrapper<T, Input, Output> {
            procedure: T,
            _input: PhantomData<Input>,
            _output: PhantomData<Output>,
        }

        impl<T, Input, Output> ProcedureWrapper<T, Input, Output>
        where
            T: TypedProcedure<Input = Input, Output = Output>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            fn new(procedure: T) -> Self {
                Self {
                    procedure,
                    _input: PhantomData,
                    _output: PhantomData,
                }
            }

            async fn invoke_internal(
                &self,
                arguments: battler_wamp_values::List,
                arguments_keyword: battler_wamp_values::Dictionary,
            ) -> Result<RpcYield> {
                let input =
                    Input::wamp_deserialize_application_message(arguments, arguments_keyword)?;
                let output = self.procedure.invoke(input).await?;
                let (arguments, arguments_keyword) = output.wamp_serialize_application_message()?;
                Ok(RpcYield {
                    arguments,
                    arguments_keyword,
                })
            }
        }

        #[async_trait]
        impl<T, Input, Output> Procedure for ProcedureWrapper<T, Input, Output>
        where
            T: TypedProcedure<Input = Input, Output = Output>,
            Input: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
            Output: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            async fn invoke(&self, mut invocation: Invocation) -> Result<()> {
                let mut arguments = List::default();
                let mut arguments_keyword = Dictionary::default();
                std::mem::swap(&mut invocation.arguments, &mut arguments);
                std::mem::swap(&mut invocation.arguments_keyword, &mut arguments_keyword);
                let result = self.invoke_internal(arguments, arguments_keyword).await;
                invocation.respond(result)
            }
        }

        self.procedures.insert(
            uri,
            PreregisteredProcedure {
                procedure: Arc::new(Box::new(ProcedureWrapper::new(procedure))),
                ignore_registration_error: false,
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
            self.procedures.into_iter(),
        )
        .start()
    }
}
