use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler_wamp::{
    auth::scram::{
        UserData,
        UserDatabase,
        UserDatabaseFactory,
        new_user,
    },
    core::{
        error::{
            InteractionError,
            WampError,
        },
        hash::HashMap,
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
        publish_options::PublishOptions,
    },
    peer::{
        PeerConfig,
        ProcedureMessage,
        ProcedureOptions,
        PublishedEvent,
        RpcCall,
        RpcYield,
        SubscriptionOptions,
        SupportedAuthMethod as PeerSupportedAuthMethod,
        new_web_socket_peer,
    },
    router::{
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        SupportedAuthMethod as RouterSupportedAuthMethod,
        new_web_socket_router,
    },
    serializer::serializer::SerializerType,
};
use battler_wamp_uri::{
    Uri,
    WildcardUri,
};
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
};
use clap::{
    Parser,
    Subcommand,
};
use futures_util::StreamExt;
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "battler-wamp-compat-test-bin")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Router {
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long)]
        realm: String,
        #[arg(long, default_value = "none")]
        auth: String, // "none", "undisputed", "scram"
    },
    Client {
        #[arg(long)]
        url: String,
        #[arg(long)]
        realm: String,
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value = "")]
        auth_id: String,
        #[arg(long, default_value = "")]
        auth_secret: String,
    },
}

#[derive(Default)]
struct SimpleUserDatabase {
    users: HashMap<String, UserData>,
}

impl SimpleUserDatabase {
    fn new(users: &[(&str, &str)]) -> Self {
        let mut db = HashMap::default();
        for &(username, password) in users {
            if let Ok(user) = new_user(username, password) {
                db.insert(username.to_owned(), user);
            }
        }
        Self { users: db }
    }
}

#[async_trait]
impl UserDatabase for SimpleUserDatabase {
    async fn user_data(&self, id: &str) -> Result<UserData> {
        self.users
            .get(id)
            .ok_or_else(|| InteractionError::NoSuchPrincipal.into())
            .cloned()
    }
}

#[derive(Debug, Clone)]
struct SimpleUserDatabaseFactory {
    users: Vec<(String, String)>,
}

impl SimpleUserDatabaseFactory {
    fn new(users: &[(&str, &str)]) -> Self {
        Self {
            users: users
                .iter()
                .map(|&(u, p)| (u.to_owned(), p.to_owned()))
                .collect(),
        }
    }
}

#[async_trait]
impl UserDatabaseFactory for SimpleUserDatabaseFactory {
    async fn create_user_database(&self) -> Result<Box<dyn UserDatabase>> {
        let users: Vec<(&str, &str)> = self
            .users
            .iter()
            .map(|(u, p)| (u.as_str(), p.as_str()))
            .collect();
        Ok(Box::new(SimpleUserDatabase::new(&users)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Optional tracing subscriber to aid debugging
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|val| val.parse::<tracing_core::LevelFilter>().ok())
        .unwrap_or(tracing_core::LevelFilter::INFO);
    let _ = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .try_init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Router { port, realm, auth } => {
            run_router(port, &realm, &auth).await?;
        }
        Commands::Client {
            url,
            realm,
            scenario,
            auth_id,
            auth_secret,
        } => {
            run_client_scenario(&url, &realm, &scenario, &auth_id, &auth_secret).await?;
        }
    }
    Ok(())
}

async fn run_router(port: u16, realm: &str, auth: &str) -> Result<()> {
    let auth_config = match auth {
        "scram" => {
            let factory = SimpleUserDatabaseFactory::new(&[("test-user", "test-password123!")]);
            RealmAuthenticationConfig {
                required: true,
                methods: vec![RouterSupportedAuthMethod::WampScram(Arc::new(
                    Box::new(factory) as Box<dyn UserDatabaseFactory>,
                ))],
            }
        }
        "undisputed" => RealmAuthenticationConfig {
            required: true,
            methods: vec![RouterSupportedAuthMethod::Undisputed],
        },
        _ => RealmAuthenticationConfig::default(),
    };

    let router = new_web_socket_router(
        RouterConfig {
            port,
            realms: vec![RealmConfig {
                name: "test".to_owned(),
                uri: Uri::try_from(realm)?,
                authentication: auth_config,
            }],
            ..Default::default()
        },
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;

    let (router_handle, join_handle) = router.start().await?;
    println!(
        "READY: ws://127.0.0.1:{}",
        router_handle.local_addr().port()
    );

    // Run until interrupted
    let _ = tokio::signal::ctrl_c().await;
    router_handle.cancel()?;
    join_handle.await?;
    Ok(())
}

async fn run_client_scenario(
    url: &str,
    realm: &str,
    scenario: &str,
    auth_id: &str,
    auth_secret: &str,
) -> Result<()> {
    let client_name = format!("rust-client-{}", scenario);
    let mut config = PeerConfig::default();
    config.name = client_name.clone();
    config.serializers = std::collections::HashSet::from_iter([SerializerType::Json]);
    config.callee.enforce_timeouts = true;
    let peer = new_web_socket_peer(config)?;
    peer.connect(url).await?;

    if scenario == "client-invalid-realm" {
        // Specifically join an invalid realm and exit
        match peer.join_realm("com.invalid.realm").await {
            Ok(_) => {
                println!("FAIL: joined invalid realm");
                return Err(Error::msg("Successfully joined invalid realm"));
            }
            Err(e) => {
                println!("SUCCESS: rejected with error: {:?}", e);
                return Ok(());
            }
        }
    }

    // Join the specified realm
    if auth_id != "" {
        let auth_method = if auth_secret.starts_with("role:") {
            PeerSupportedAuthMethod::Undisputed {
                id: auth_id.to_owned(),
                role: auth_secret.strip_prefix("role:").unwrap().to_owned(),
            }
        } else {
            PeerSupportedAuthMethod::WampScram {
                id: auth_id.to_owned(),
                password: auth_secret.to_owned(),
            }
        };
        peer.join_realm_with_authentication(realm, &[auth_method])
            .await?;
    } else {
        peer.join_realm(realm).await?;
    }

    match scenario {
        "pubsub" => {
            // Subscribe to com.compat.topic
            let mut sub = peer.subscribe(Uri::try_from("com.compat.topic")?).await?;
            println!("SUBSCRIBED");

            // Publish com.compat.topic
            peer.publish(
                Uri::try_from("com.compat.topic")?,
                PublishedEvent {
                    arguments: List::from_iter([
                        Value::Integer(123),
                        Value::String("test".to_owned()),
                    ]),
                    arguments_keyword: Dictionary::from_iter([(
                        "foo".to_owned(),
                        Value::String("bar".to_owned()),
                    )]),
                    options: PublishOptions {
                        acknowledge: Some(true),
                        exclude_me: Some(false),
                        ..Default::default()
                    },
                },
            )
            .await?;
            println!("PUBLISHED");

            // Receive the event we published
            match tokio::time::timeout(Duration::from_secs(5), sub.event_rx.recv()).await {
                Ok(Ok(event)) => {
                    println!("EVENT: {:?}", event);
                }
                _ => {
                    return Err(Error::msg("Timed out waiting for event"));
                }
            }
            peer.unsubscribe(sub.id).await?;
            println!("UNSUBSCRIBED");
        }
        "pubsub-prefix" => {
            let mut sub = peer
                .subscribe_with_options(
                    WildcardUri::try_from("com.compat")?,
                    SubscriptionOptions {
                        match_style: Some(MatchStyle::Prefix),
                    },
                )
                .await?;
            println!("SUBSCRIBED");
            match tokio::time::timeout(Duration::from_secs(5), sub.event_rx.recv()).await {
                Ok(Ok(event)) => {
                    println!("EVENT: {:?} on topic: {:?}", event.arguments, event.topic);
                }
                _ => return Err(Error::msg("Timed out waiting for prefix event")),
            }
            peer.unsubscribe(sub.id).await?;
        }
        "pubsub-wildcard" => {
            let mut sub = peer
                .subscribe_with_options(
                    WildcardUri::try_from("com.compat..status")?,
                    SubscriptionOptions {
                        match_style: Some(MatchStyle::Wildcard),
                    },
                )
                .await?;
            println!("SUBSCRIBED");
            match tokio::time::timeout(Duration::from_secs(5), sub.event_rx.recv()).await {
                Ok(Ok(event)) => {
                    println!("EVENT: {:?} on topic: {:?}", event.arguments, event.topic);
                }
                _ => return Err(Error::msg("Timed out waiting for wildcard event")),
            }
            peer.unsubscribe(sub.id).await?;
        }
        "rpc-callee" => {
            // Register com.compat.proc
            let mut procedure = peer.register(Uri::try_from("com.compat.proc")?).await?;
            println!("REGISTERED");

            // Process one invocation and respond
            if let Ok(ProcedureMessage::Invocation(invocation)) =
                procedure.procedure_message_rx.recv().await
            {
                println!("INVOCATION: {:?}", invocation.arguments);
                // Double the first argument if it is an integer
                let val = match invocation.arguments.first() {
                    Some(Value::Integer(x)) => Value::Integer(x * 2),
                    _ => Value::Null,
                };
                invocation
                    .respond_ok(RpcYield {
                        arguments: List::from_iter([val]),
                        ..Default::default()
                    })
                    .await?;
                println!("RESPONDED");
            }
            peer.unregister(procedure.id).await?;
            println!("UNREGISTERED");
        }
        "rpc-caller" => {
            println!("CALLING");
            let result = peer
                .call_and_wait(
                    Uri::try_from("com.compat.proc")?,
                    RpcCall {
                        arguments: List::from_iter([Value::Integer(10)]),
                        ..Default::default()
                    },
                )
                .await?;
            println!("RESULT: {:?}", result.arguments);
        }
        "rpc-unregister" => {
            let procedure = peer.register(Uri::try_from("com.compat.proc")?).await?;
            println!("REGISTERED");
            peer.unregister(procedure.id).await?;
            println!("UNREGISTERED");
        }
        "rpc-error" => {
            let mut procedure = peer
                .register(Uri::try_from("com.compat.error_proc")?)
                .await?;
            println!("REGISTERED");
            if let Ok(ProcedureMessage::Invocation(invocation)) =
                procedure.procedure_message_rx.recv().await
            {
                let err = WampError::new(
                    Uri::try_from("com.compat.error.custom")?,
                    "custom callee error payload".to_owned(),
                );
                invocation.respond(Err(err)).await?;
                println!("RESPONDED_ERROR");
            }
            peer.unregister(procedure.id).await?;
        }
        "rpc-shared" => {
            // Register with single policy by default or CLI could define it
            let policy = if auth_secret.starts_with("policy:") {
                auth_secret
                    .strip_prefix("policy:")
                    .unwrap()
                    .try_into()
                    .unwrap_or(InvocationPolicy::Single)
            } else {
                InvocationPolicy::Single
            };
            match peer
                .register_with_options(
                    WildcardUri::try_from("com.compat.shared")?,
                    ProcedureOptions {
                        invocation_policy: policy,
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(proc) => {
                    println!("REGISTERED_SHARED");
                    let mut rx = proc.procedure_message_rx.resubscribe();
                    tokio::select! {
                        msg = rx.recv() => {
                            if let Ok(ProcedureMessage::Invocation(invocation)) = msg {
                                invocation.respond_ok(RpcYield {
                                    arguments: List::from_iter([Value::String(client_name)]),
                                    ..Default::default()
                                }).await?;
                                println!("RESPONDED");
                            }
                        }
                        _ = sleep(Duration::from_millis(1500)) => {}
                    }
                    peer.unregister(proc.id).await?;
                }
                Err(e) => {
                    println!("REGISTER_FAILED: {:?}", e);
                }
            }
        }
        "rpc-disclose-caller" => {
            let mut procedure = peer
                .register_with_options(
                    WildcardUri::try_from("com.compat.disclose")?,
                    ProcedureOptions {
                        disclose_caller: true,
                        ..Default::default()
                    },
                )
                .await?;
            println!("REGISTERED");
            if let Ok(ProcedureMessage::Invocation(invocation)) =
                procedure.procedure_message_rx.recv().await
            {
                // Return the caller's authid
                let caller_authid = invocation.peer_info.identity.id.clone();
                invocation
                    .respond_ok(RpcYield {
                        arguments: List::from_iter([Value::String(caller_authid)]),
                        ..Default::default()
                    })
                    .await?;
            }
            peer.unregister(procedure.id).await?;
        }
        "rpc-timeout" => {
            // Caller scenario to call a slow proc and expect timeout
            println!("CALLING_TIMEOUT");
            let result = tokio::time::timeout(
                Duration::from_millis(500),
                peer.call_and_wait(
                    Uri::try_from("com.compat.slow_proc")?,
                    RpcCall {
                        timeout: Some(Duration::from_millis(500)),
                        ..Default::default()
                    },
                ),
            )
            .await;
            match result {
                Ok(Ok(_)) => println!("TIMEOUT_FAIL"),
                Ok(Err(e)) => println!("TIMEOUT_SUCCESS: {:?}", e),
                Err(_) => println!("TIMEOUT_SUCCESS: Local timeout"),
            }
        }
        "rpc-cancel" => {
            // Callee registers a slow procedure, waits for INTERRUPT, and explicitly responds
            // with an error.
            let mut procedure = peer
                .register(Uri::try_from("com.compat.slow_proc")?)
                .await?;
            println!("REGISTERED");
            if let Ok(ProcedureMessage::Invocation(invocation)) =
                procedure.procedure_message_rx.recv().await
            {
                println!("INVOCATION_RECEIVED");
                tokio::select! {
                    msg = procedure.procedure_message_rx.recv() => {
                        if let Ok(ProcedureMessage::Interrupt(_)) = msg {
                            println!("CANCEL_RECEIVED");
                            let err = WampError::new(
                                Uri::try_from("wamp.error.canceled")?,
                                "Call canceled by caller".to_owned(),
                            );
                            let _ = invocation.respond(Err(err)).await;
                            println!("CANCEL_RESPONDED");
                        }
                    }
                    _ = sleep(Duration::from_secs(3)) => {
                        let _ = invocation.respond_ok(RpcYield::default()).await;
                    }
                }
            }
            peer.unregister(procedure.id).await?;
        }
        "rpc-progressive" => {
            // Caller calls com.compat.progress and prints all progressive results
            println!("CALLING_PROGRESSIVE");
            let pending = peer
                .call_with_progress(Uri::try_from("com.compat.progress")?, RpcCall::default())
                .await?;

            let mut stream = pending.into_stream();
            while let Some(res) = stream.next().await {
                match res {
                    Ok(result) => {
                        println!(
                            "PROGRESS_RESULT: {:?} progress: {}",
                            result.arguments, result.progress
                        );
                    }
                    Err(e) => {
                        println!("PROGRESS_ERROR: {:?}", e);
                        break;
                    }
                }
            }
        }
        "rpc-caller-error" => {
            // Caller calls a procedure that the TS control client registered to throw an error
            let result = peer
                .call_and_wait(Uri::try_from("com.compat.error_proc")?, RpcCall::default())
                .await;
            match result {
                Err(e) => println!("CALL_ERROR: {:?}", e),
                Ok(_) => println!("CALL_ERROR_FAIL: expected error but got result"),
            }
        }
        "rpc-callee-progressive" => {
            // Callee registers a procedure and streams progressive results to the TS caller
            let mut procedure = peer
                .register(Uri::try_from("com.compat.callee_progress")?)
                .await?;
            println!("REGISTERED");
            if let Ok(ProcedureMessage::Invocation(invocation)) =
                procedure.procedure_message_rx.recv().await
            {
                invocation
                    .progress(RpcYield {
                        arguments: List::from_iter([Value::Integer(10)]),
                        ..Default::default()
                    })
                    .await?;
                println!("PROGRESS_SENT: 10");
                invocation
                    .progress(RpcYield {
                        arguments: List::from_iter([Value::Integer(20)]),
                        ..Default::default()
                    })
                    .await?;
                println!("PROGRESS_SENT: 20");
                invocation
                    .respond_ok(RpcYield {
                        arguments: List::from_iter([Value::Integer(30)]),
                        ..Default::default()
                    })
                    .await?;
                println!("RESPONDED");
            }
            peer.unregister(procedure.id).await?;
        }

        _ => {
            println!("Unknown scenario: {}", scenario);
        }
    }

    peer.leave_realm().await?;
    peer.disconnect().await?;
    Ok(())
}
