use anyhow::Result;
use battler_wamp::{
    core::uri::Uri,
    peer::{
        WebSocketPeer,
        new_web_socket_peer,
    },
    router::{
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        new_web_socket_router,
    },
};
use battler_wamp_values::{
    WampDictionary,
    WampList,
};
use battler_wamprat::{
    peer::{
        CallOptions,
        PeerConnectionConfig,
        PeerConnectionType,
    },
    procedure::{
        Invocation,
        ProgressReporter,
        TypedPatternMatchedProgressiveProcedure,
        TypedProcedure,
    },
    subscription::{
        TypedPatternMatchedSubscription,
        TypedSubscription,
    },
};
use battler_wamprat_error::WampError;
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_schema::{
    PeerConfig,
    WampSchemaError,
};
use battler_wamprat_schema_proc_macro::WampSchema as WampSchemaUnderTest;
use battler_wamprat_uri::WampUriMatcher;
use thiserror::Error;
use tokio::{
    sync::broadcast::{
        self,
        error::{
            RecvError,
            TryRecvError,
        },
    },
    task::JoinHandle,
};

async fn start_router(port: u16, realm: &str) -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig::default();
    config.port = port;
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(realm)?,
        authentication: RealmAuthenticationConfig::default(),
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let mut config = battler_wamp::peer::PeerConfig::default();
    config.name = name.to_owned();
    new_web_socket_peer(config)
}

#[derive(Debug, WampList)]
struct OneNumber(u64);

#[derive(Debug, WampList)]
struct TwoNumbers(u64, u64);

#[derive(Debug, WampApplicationMessage)]
struct Input(#[arguments] TwoNumbers);

#[derive(Debug, WampApplicationMessage)]
struct Output(#[arguments] OneNumber);

#[derive(Debug, Error, WampError)]
enum DivideError {
    #[error("cannot divide by 0")]
    #[uri("com.battler.error.divide_by_zero")]
    DivideByZero,
}

#[derive(Debug, Clone, WampApplicationMessage)]
struct Ping;

#[derive(WampSchemaUnderTest)]
#[realm("com.battler.realm")]
#[allow(unused)]
enum Calculator {
    #[rpc(uri = "com.battler.add", input = Input, output = Output)]
    Add,
    #[rpc(uri = "com.battler.divide", input = Input, output = Output, error = DivideError)]
    Divide,
    #[pubsub(uri = "com.battler.ping", event = Ping)]
    Ping,
}

struct Adder;
impl AddProcedure for Adder {}

impl TypedProcedure for Adder {
    type Input = Input;
    type Output = Output;
    type Error = anyhow::Error;

    async fn invoke(&self, _: Invocation, input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(Output(OneNumber(input.0.0 + input.0.1)))
    }
}

struct Divider;
impl DivideProcedure for Divider {}

impl TypedProcedure for Divider {
    type Input = Input;
    type Output = Output;
    type Error = DivideError;

    async fn invoke(&self, _: Invocation, input: Self::Input) -> Result<Self::Output, Self::Error> {
        if input.0.1 == 0 {
            Err(DivideError::DivideByZero)
        } else {
            Ok(Output(OneNumber(input.0.0 / input.0.1)))
        }
    }
}

struct PingHandler {
    events_tx: broadcast::Sender<Ping>,
}
impl PingSubscription for PingHandler {}

impl TypedSubscription for PingHandler {
    type Event = Ping;

    async fn handle_event(&self, event: Self::Event) {
        self.events_tx.send(event).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn generates_producer_and_consumers_for_calculator_procedures() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(0, "com.battler.realm").await.unwrap();

    let mut producer_builder = Calculator::producer_builder(PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
            "ws://{}",
            router_handle.local_addr()
        ))),
        auth_methods: Vec::default(),
    });

    producer_builder.register_add(Adder).unwrap();
    producer_builder.register_divide(Divider).unwrap();
    let producer = producer_builder
        .start(create_peer("producer").unwrap())
        .unwrap();
    producer.wait_until_ready().await.unwrap();

    let consumer = Calculator::consumer(
        PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
                "ws://{}",
                router_handle.local_addr()
            ))),
            auth_methods: Vec::default(),
        },
        create_peer("consumer").unwrap(),
    )
    .unwrap();
    consumer.wait_until_ready().await.unwrap();

    assert_matches::assert_matches!(
        consumer
            .add(Input(TwoNumbers(36345, 88818)), CallOptions::default())
            .await,
        Ok(rpc) => {
            assert_matches::assert_matches!(rpc.result().await, Ok(Output(OneNumber(125163))));
        }
    );

    assert_matches::assert_matches!(consumer.divide(Input(TwoNumbers(25, 2)), CallOptions::default()).await, Ok(rpc) => {
        assert_matches::assert_matches!(rpc.result_observing_error().await, Ok(Output(OneNumber(12))));
    });

    assert_matches::assert_matches!(consumer.divide(Input(TwoNumbers(1, 0)), CallOptions::default()).await, Ok(rpc) => {
        assert_matches::assert_matches!(rpc.result_observing_error().await, Err(WampSchemaError::Known(DivideError::DivideByZero)));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn generates_pub_sub_for_calculator_topics() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(0, "com.battler.realm").await.unwrap();

    let producer_builder = Calculator::producer_builder(PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
            "ws://{}",
            router_handle.local_addr()
        ))),
        auth_methods: Vec::default(),
    });

    let producer = producer_builder
        .start(create_peer("producer").unwrap())
        .unwrap();
    producer.wait_until_ready().await.unwrap();

    let consumer = Calculator::consumer(
        PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
                "ws://{}",
                router_handle.local_addr()
            ))),
            auth_methods: Vec::default(),
        },
        create_peer("consumer").unwrap(),
    )
    .unwrap();
    consumer.wait_until_ready().await.unwrap();

    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        consumer.subscribe_ping(PingHandler { events_tx }).await,
        Ok(())
    );

    assert_matches::assert_matches!(producer.publish_ping(Ping).await, Ok(()));
    assert_matches::assert_matches!(producer.publish_ping(Ping).await, Ok(()));
    assert_matches::assert_matches!(producer.publish_ping(Ping).await, Ok(()));

    assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
    assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
    assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
    assert_matches::assert_matches!(events_rx.try_recv(), Err(TryRecvError::Empty));

    assert_matches::assert_matches!(consumer.unsubscribe_ping().await, Ok(()));

    assert_matches::assert_matches!(producer.publish_ping(Ping).await, Ok(()));
    assert_matches::assert_matches!(events_rx.recv().await, Err(RecvError::Closed));
}

#[derive(Debug, Clone, WampDictionary)]
struct Message {
    author: String,
    content: String,
}

#[derive(Debug, WampApplicationMessage)]
struct MessageEvent(#[arguments_keyword] Message);

#[derive(Debug, WampUriMatcher)]
#[uri("com.battler.message.{version}.{channel}")]
struct MessagePattern {
    version: u64,
    channel: String,
}

#[derive(WampApplicationMessage)]
struct UploadInput;

#[derive(Debug, WampDictionary)]
struct UploadOutputArgs {
    percentage: u64,
}

#[derive(Debug, WampApplicationMessage)]
struct UploadOutput(#[arguments_keyword] UploadOutputArgs);

#[derive(Debug, WampUriMatcher)]
#[uri("com.battler.upload.{file_type}.public")]
struct UploadPattern {
    file_type: String,
}

#[derive(Debug, Error, WampError)]
enum UploadError {
    #[error("unsupported file type")]
    #[uri("com.battler_wamprat.test.error.unsupported_file_type")]
    UnsupportedFileType,
}

#[derive(WampSchemaUnderTest)]
#[realm("com.battler.realm")]
#[allow(unused)]
enum Chat {
    #[pubsub(pattern = MessagePattern, event = MessageEvent)]
    Message,
    #[rpc(pattern = UploadPattern, input = UploadInput, output = UploadOutput, error = UploadError, progressive)]
    Upload,
}

struct MessageHandler {
    events_tx: broadcast::Sender<(Message, u64, String)>,
}
impl MessageSubscription for MessageHandler {}

impl TypedPatternMatchedSubscription for MessageHandler {
    type Event = MessageEvent;
    type Pattern = MessagePattern;

    async fn handle_event(&self, event: Self::Event, topic: Self::Pattern) {
        self.events_tx
            .send((event.0, topic.version, topic.channel))
            .unwrap();
    }
}

struct UploadHandler;
impl UploadProcedure for UploadHandler {}

impl TypedPatternMatchedProgressiveProcedure for UploadHandler {
    type Input = UploadInput;
    type Output = UploadOutput;
    type Error = UploadError;
    type Pattern = UploadPattern;

    async fn invoke<'rpc>(
        &self,
        _: Invocation,
        _: Self::Input,
        procedure: Self::Pattern,
        progress: ProgressReporter<'rpc, Self::Output>,
    ) -> Result<Self::Output, Self::Error> {
        if procedure.file_type != "png" {
            return Err(UploadError::UnsupportedFileType);
        }
        progress
            .send(UploadOutput(UploadOutputArgs { percentage: 25 }))
            .await
            .ok();
        progress
            .send(UploadOutput(UploadOutputArgs { percentage: 50 }))
            .await
            .ok();
        progress
            .send(UploadOutput(UploadOutputArgs { percentage: 75 }))
            .await
            .ok();
        Ok(UploadOutput(UploadOutputArgs { percentage: 100 }))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn generates_pub_sub_for_chat_topics() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(0, "com.battler.realm").await.unwrap();

    let producer_builder = Chat::producer_builder(PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
            "ws://{}",
            router_handle.local_addr()
        ))),
        auth_methods: Vec::default(),
    });

    let producer = producer_builder
        .start(create_peer("producer").unwrap())
        .unwrap();
    producer.wait_until_ready().await.unwrap();

    let consumer = Chat::consumer(
        PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
                "ws://{}",
                router_handle.local_addr()
            ))),
            auth_methods: Vec::default(),
        },
        create_peer("consumer").unwrap(),
    )
    .unwrap();
    consumer.wait_until_ready().await.unwrap();

    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        consumer
            .subscribe_message(MessageHandler { events_tx })
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        producer
            .publish_message(
                MessagePattern {
                    version: 1,
                    channel: "main".to_owned()
                },
                MessageEvent(Message {
                    author: "user1".to_owned(),
                    content: "foo".to_owned()
                })
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        producer
            .publish_message(
                MessagePattern {
                    version: 2,
                    channel: "home".to_owned()
                },
                MessageEvent(Message {
                    author: "user2".to_owned(),
                    content: "bar".to_owned()
                })
            )
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(events_rx.recv().await, Ok((message, version, channel)) => {
        assert_eq!(message.author, "user1");
        assert_eq!(message.content, "foo");
        assert_eq!(version, 1);
        assert_eq!(channel, "main");
    });
    assert_matches::assert_matches!(events_rx.recv().await, Ok((message, version, channel)) => {
        assert_eq!(message.author, "user2");
        assert_eq!(message.content, "bar");
        assert_eq!(version, 2);
        assert_eq!(channel, "home");
    });
    assert_matches::assert_matches!(events_rx.try_recv(), Err(TryRecvError::Empty));

    assert_matches::assert_matches!(consumer.unsubscribe_message().await, Ok(()));

    assert_matches::assert_matches!(
        producer
            .publish_message(
                MessagePattern {
                    version: 1,
                    channel: "main".to_owned()
                },
                MessageEvent(Message {
                    author: "user1".to_owned(),
                    content: "baz".to_owned()
                })
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(events_rx.recv().await, Err(RecvError::Closed));
}

#[tokio::test(flavor = "multi_thread")]
async fn generates_producer_and_consumers_for_chat_procedures() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(0, "com.battler.realm").await.unwrap();

    let mut producer_builder = Chat::producer_builder(PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
            "ws://{}",
            router_handle.local_addr()
        ))),
        auth_methods: Vec::default(),
    });

    producer_builder.register_upload(UploadHandler).unwrap();
    let producer = producer_builder
        .start(create_peer("producer").unwrap())
        .unwrap();
    producer.wait_until_ready().await.unwrap();

    let consumer = Chat::consumer(
        PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
                "ws://{}",
                router_handle.local_addr()
            ))),
            auth_methods: Vec::default(),
        },
        create_peer("consumer").unwrap(),
    )
    .unwrap();
    consumer.wait_until_ready().await.unwrap();

    assert_matches::assert_matches!(
        consumer
            .upload(UploadPattern { file_type: "gif".to_owned() }, UploadInput, CallOptions::default())
            .await,
        Ok(mut rpc) => {
            assert_matches::assert_matches!(rpc.next_result_observing_error().await, Err(WampSchemaError::Known(UploadError::UnsupportedFileType)));
        }
    );

    assert_matches::assert_matches!(
        consumer
            .upload(UploadPattern { file_type: "png".to_owned() }, UploadInput, CallOptions::default())
            .await,
        Ok(mut rpc) => {
            assert_matches::assert_matches!(rpc.next_result().await, Ok(Some(output)) => {
                assert_eq!(output.0.percentage, 25);
            });
            assert_matches::assert_matches!(rpc.next_result().await, Ok(Some(output)) => {
                assert_eq!(output.0.percentage, 50);
            });
            assert_matches::assert_matches!(rpc.next_result().await, Ok(Some(output)) => {
                assert_eq!(output.0.percentage, 75);
            });
            assert_matches::assert_matches!(rpc.next_result().await, Ok(Some(output)) => {
                assert_eq!(output.0.percentage, 100);
            });
            assert_matches::assert_matches!(rpc.next_result().await, Ok(None));
        }
    );
}
