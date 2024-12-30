use serde_struct_tuple::{
    DeserializeStructTuple,
    SerializeStructTuple,
};
use serde_struct_tuple_enum::{
    DeserializeStructTupleEnum,
    SerializeStructTupleEnum,
};

use crate::core::{
    id::Id,
    types::{
        Dictionary,
        Integer,
        List,
    },
    uri::Uri,
};

/// A HELLO message for a peer to initiate a WAMP session in a realm.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct HelloMessage {
    pub realm: Uri,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub details: Dictionary,
}

/// A WELCOME message for a router to confirm a peer's WAMP session in a realm.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct WelcomeMessage {
    pub session: Id,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub details: Dictionary,
}

/// An ABORT message for quickly terminating a WAMP session.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct AbortMessage {
    pub details: Dictionary,
    pub reason: Uri,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub arguments_keyword: Dictionary,
}

/// A GOODBYE message for ending a WAMP session with a two-way handshake.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct GoodbyeMessage {
    pub details: Dictionary,
    pub reason: Uri,
}

/// An ERROR message for communicating an error in response to a single request.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct ErrorMessage {
    pub request_type: Integer,
    pub request: Id,
    pub details: Dictionary,
    pub error: Uri,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub arguments_keyword: Dictionary,
}

/// A PUBLISH message for publishing an event to a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct PublishMessage {
    pub request: Id,
    pub options: Dictionary,
    pub topic: Uri,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub arguments_keyword: Dictionary,
}

/// A PUBLISHED message for confirming an event was published.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct PublishedMessage {
    pub publish_request: Id,
    pub publication: Id,
}

/// A SUBSCRIBE message for subscribing to a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct SubscribeMessage {
    pub request: Id,
    pub options: Dictionary,
    pub topic: Uri,
}

/// A SUBSCRIBED message for confirming a peer has subscribed to a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct SubscribedMessage {
    pub subscribe_request: Id,
    pub subscription: Id,
}

/// An UNSUBSCRIBE message for unsubscribing from a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct UnsubscribeMessage {
    pub request: Id,
    pub subscribed_subscription: Id,
}

/// An UNSUBSCRIBED message for confirming a peer has unsubscribed from a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct UnsubscribedMessage {
    pub unsubscribe_request: Id,
}

/// An EVENT message for relaying a published event to subscribers.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct EventMessage {
    pub subscribed_subscription: Id,
    pub published_publication: Id,
    pub details: Dictionary,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub publish_arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub publish_arguments_keyword: Dictionary,
}

/// A CALL message for invoking a procedure.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct CallMessage {
    pub request: Id,
    pub options: Dictionary,
    pub procedure: Uri,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub arguments_keyword: Dictionary,
}

/// A RESULT message for sending the result of a procedure invocation.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct ResultMessage {
    pub call_request: Id,
    pub details: Dictionary,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub yield_arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub yield_arguments_keyword: Dictionary,
}

/// A REGISTER message for registering a procedure in the realm.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct RegisterMessage {
    pub request: Id,
    pub options: Dictionary,
    pub procedure: Uri,
}

/// A REGISTERED message for confirming a procedure has been registered.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct RegisteredMessage {
    pub register_request: Id,
    pub registration: Id,
}

/// An UNREGISTERED message for unregistering a procedure in the realm.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct UnregisterMessage {
    pub request: Id,
    pub registered_registration: Id,
}

/// An UNREGISTERED message for confirming a procedure has been unregistered.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct UnregisteredMessage {
    pub unregister_request: Id,
}

/// An INVOCATION message for invoking a procedure on its callee.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct InvocationMessage {
    pub request: Id,
    pub registered_registration: Id,
    pub details: Dictionary,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub call_arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub call_arguments_keyword: Dictionary,
}

/// A YIELD message for yielding the result of an invocation from the callee.
#[derive(Debug, Default, Clone, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
pub struct YieldMessage {
    pub invocation_request: Id,
    pub options: Dictionary,
    #[serde_struct_tuple(default, skip_serializing_if = List::is_empty)]
    pub arguments: List,
    #[serde_struct_tuple(default, skip_serializing_if = Dictionary::is_empty)]
    pub arguments_keyword: Dictionary,
}

/// A WAMP message.
#[derive(Debug, Clone, PartialEq, Eq, SerializeStructTupleEnum, DeserializeStructTupleEnum)]
#[tag(Integer)]
pub enum Message {
    #[tag = 1]
    Hello(HelloMessage),
    #[tag = 2]
    Welcome(WelcomeMessage),
    #[tag = 3]
    Abort(AbortMessage),
    #[tag = 6]
    Goodbye(GoodbyeMessage),
    #[tag = 8]
    Error(ErrorMessage),
    #[tag = 16]
    Publish(PublishMessage),
    #[tag = 17]
    Published(PublishedMessage),
    #[tag = 32]
    Subscribe(SubscribeMessage),
    #[tag = 33]
    Subscribed(SubscribedMessage),
    #[tag = 34]
    Unsubscribe(UnsubscribeMessage),
    #[tag = 35]
    Unsubscribed(UnsubscribedMessage),
    #[tag = 36]
    Event(EventMessage),
    #[tag = 48]
    Call(CallMessage),
    #[tag = 50]
    Result(ResultMessage),
    #[tag = 64]
    Register(RegisterMessage),
    #[tag = 65]
    Registered(RegisteredMessage),
    #[tag = 66]
    Unregister(UnregisterMessage),
    #[tag = 67]
    Unregistered(UnregisteredMessage),
    #[tag = 68]
    Invocation(InvocationMessage),
    #[tag = 70]
    Yield(YieldMessage),
}

impl Message {
    /// The message name, mostly for logging.
    pub fn message_name(&self) -> &'static str {
        match self {
            Self::Hello(_) => "HELLO",
            Self::Welcome(_) => "WELCOME",
            Self::Abort(_) => "ABORT",
            Self::Goodbye(_) => "GOODBYE",
            Self::Error(_) => "ERROR",
            Self::Publish(_) => "PUBLISH",
            Self::Published(_) => "PUBLISHED",
            Self::Subscribe(_) => "SUBSCRIBE",
            Self::Subscribed(_) => "SUBSCRIBED",
            Self::Unsubscribe(_) => "UNSUBSCRIBE",
            Self::Unsubscribed(_) => "UNSUBSCRIBED",
            Self::Event(_) => "EVENT",
            Self::Call(_) => "CALL",
            Self::Result(_) => "RESULT",
            Self::Register(_) => "REGISTER",
            Self::Registered(_) => "REGISTERED",
            Self::Unregister(_) => "UNREGISTER",
            Self::Unregistered(_) => "UNREGISTERED",
            Self::Invocation(_) => "INVOCATION",
            Self::Yield(_) => "YIELD",
        }
    }

    /// The request ID on the message.
    pub fn request_id(&self) -> Option<Id> {
        match self {
            Self::Error(message) => Some(message.request),
            Self::Publish(message) => Some(message.request),
            Self::Published(message) => Some(message.publish_request),
            Self::Subscribe(message) => Some(message.request),
            Self::Subscribed(message) => Some(message.subscribe_request),
            Self::Unsubscribe(message) => Some(message.request),
            Self::Unsubscribed(message) => Some(message.unsubscribe_request),
            Self::Call(message) => Some(message.request),
            Self::Result(message) => Some(message.call_request),
            Self::Register(message) => Some(message.request),
            Self::Registered(message) => Some(message.register_request),
            Self::Unregister(message) => Some(message.request),
            Self::Unregistered(message) => Some(message.unregister_request),
            Self::Invocation(message) => Some(message.request),
            Self::Yield(message) => Some(message.invocation_request),
            _ => None,
        }
    }

    /// The details dictionary on the message.
    pub fn details(&self) -> Option<&Dictionary> {
        match self {
            Self::Hello(message) => Some(&message.details),
            Self::Welcome(message) => Some(&message.details),
            Self::Abort(message) => Some(&message.details),
            Self::Goodbye(message) => Some(&message.details),
            Self::Error(message) => Some(&message.details),
            Self::Event(message) => Some(&message.details),
            Self::Result(message) => Some(&message.details),
            Self::Invocation(message) => Some(&message.details),
            _ => None,
        }
    }

    /// The error reason on the message.
    pub fn reason(&self) -> Option<&Uri> {
        match self {
            Self::Abort(message) => Some(&message.reason),
            Self::Goodbye(message) => Some(&message.reason),
            Self::Error(message) => Some(&message.error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod message_test {
    use std::fmt::Debug;

    use crate::{
        core::{
            id::Id,
            types::{
                Dictionary,
                List,
                Value,
            },
            uri::Uri,
        },
        message::message::{
            CallMessage,
            HelloMessage,
            Message,
        },
    };

    #[track_caller]
    fn assert_serialize_to_deserialize_equal<'de, T>(value: &T)
    where
        T: Debug + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
    {
        let serialized = serde_json::to_string(value).unwrap();
        let deserialized = serde_json::from_str::<T>(&serialized).unwrap();
        let serialized = serde_json::to_string(&deserialized).unwrap();
        let deserialized = serde_json::from_str::<T>(&serialized).unwrap();
        assert_eq!(value, &deserialized);
    }

    #[test]
    fn deserializes_message_from_tuple() {
        assert_matches::assert_matches!(serde_json::from_str(r#"
            [1, "com.battler"]
        "#), Ok(Message::Hello(message)) => {
            assert_eq!(message, HelloMessage {
                realm: Uri::try_from("com.battler").unwrap(),
                details: Dictionary::default(),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [1, "com.battler", { "key": true }]
        "#), Ok(Message::Hello(message)) => {
            assert_eq!(message, HelloMessage {
                realm: Uri::try_from("com.battler").unwrap(),
                details: Dictionary::from_iter([("key".to_owned(), Value::Bool(true))]),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [1, "com.battler", { "a": 1, "b": "s", "c": false, "d": { "e": "f" }, "g": [0, 1, 2, [], {}] }]
        "#), Ok(Message::Hello(message)) => {
            assert_eq!(message, HelloMessage {
                realm: Uri::try_from("com.battler").unwrap(),
                details: Dictionary::from_iter([
                    ("a".to_owned(), Value::Integer(1)),
                    ("b".to_owned(), Value::String("s".to_owned())),
                    ("c".to_owned(), Value::Bool(false)),
                    ("d".to_owned(), Value::Dictionary(Dictionary::from_iter([
                        ("e".to_owned(), Value::String("f".to_owned())),
                    ]))),
                    ("g".to_owned(), Value::List(List::from_iter([
                        Value::Integer(0),
                        Value::Integer(1),
                        Value::Integer(2),
                        Value::List(List::default()),
                        Value::Dictionary(Dictionary::default()),
                    ]))),
                ]),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [48, 7814135, {}, "com.myapp.ping"]
        "#), Ok(Message::Call(message)) => {
            assert_eq!(message, CallMessage {
                request: Id::try_from(7814135).unwrap(),
                options: Dictionary::default(),
                procedure: Uri::try_from("com.myapp.ping").unwrap(),
                arguments: List::default(),
                arguments_keyword: Dictionary::default(),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [48, 7814135, {}, "com.myapp.echo", ["Hello, world!"]]
        "#), Ok(Message::Call(message)) => {
            assert_eq!(message, CallMessage {
                request: Id::try_from(7814135).unwrap(),
                options: Dictionary::default(),
                procedure: Uri::try_from("com.myapp.echo").unwrap(),
                arguments: List::from_iter([
                    Value::String("Hello, world!".to_owned()),
                ]),
                arguments_keyword: Dictionary::default(),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [48, 7814135, {}, "com.myapp.add2", [23, 7]]
        "#), Ok(Message::Call(message)) => {
            assert_eq!(message, CallMessage {
                request: Id::try_from(7814135).unwrap(),
                options: Dictionary::default(),
                procedure: Uri::try_from("com.myapp.add2").unwrap(),
                arguments: List::from_iter([
                    Value::Integer(23),
                    Value::Integer(7),
                ]),
                arguments_keyword: Dictionary::default(),
            })
        });

        assert_matches::assert_matches!(serde_json::from_str(r#"
            [48, 7814135, {}, "com.myapp.user.new", ["Johnny"], {
                "firstname": "John",
                "surname": "Doe"
            }]
        "#), Ok(Message::Call(message)) => {
            assert_eq!(message, CallMessage {
                request: Id::try_from(7814135).unwrap(),
                options: Dictionary::default(),
                procedure: Uri::try_from("com.myapp.user.new").unwrap(),
                arguments: List::from_iter([
                    Value::String("Johnny".to_owned()),
                ]),
                arguments_keyword: Dictionary::from_iter([
                    ("firstname".to_owned(), Value::String("John".to_owned())),
                    ("surname".to_owned(), Value::String("Doe".to_owned())),
                ]),
            })
        });
    }

    #[test]
    fn serializes_message_to_tuple() {
        assert_matches::assert_matches!(
            serde_json::to_string(&Message::Hello(HelloMessage {
                realm: Uri::try_from("com.battler").unwrap(),
                details: Dictionary::default(),
            })),
            Ok(serialized) => {
                assert_eq!(serialized, r#"[1,"com.battler"]"#);
            }
        );

        assert_matches::assert_matches!(
            serde_json::to_string(&Message::Hello(HelloMessage {
                realm: Uri::try_from("com.battler").unwrap(),
               details: Dictionary::from_iter([("key".to_owned(), Value::Bool(true))]),
            })),
            Ok(serialized) => {
                assert_eq!(serialized, r#"[1,"com.battler",{"key":true}]"#);
            }
        );
    }

    #[test]
    fn serializes_and_deserializes_equivalently() {
        assert_serialize_to_deserialize_equal(&Message::Hello(HelloMessage {
            realm: Uri::try_from("com.battler").unwrap(),
            details: Dictionary::from_iter([
                ("a".to_owned(), Value::Integer(1)),
                ("b".to_owned(), Value::String("s".to_owned())),
                ("c".to_owned(), Value::Bool(false)),
                (
                    "d".to_owned(),
                    Value::Dictionary(Dictionary::from_iter([(
                        "e".to_owned(),
                        Value::String("f".to_owned()),
                    )])),
                ),
                (
                    "g".to_owned(),
                    Value::List(List::from_iter([
                        Value::Integer(0),
                        Value::Integer(1),
                        Value::Integer(2),
                        Value::List(List::default()),
                        Value::Dictionary(Dictionary::default()),
                    ])),
                ),
            ]),
        }));

        assert_serialize_to_deserialize_equal(&Message::Call(CallMessage {
            request: Id::try_from(7814135).unwrap(),
            options: Dictionary::default(),
            procedure: Uri::try_from("com.myapp.ping").unwrap(),
            arguments: List::default(),
            arguments_keyword: Dictionary::default(),
        }));

        assert_serialize_to_deserialize_equal(&Message::Call(CallMessage {
            request: Id::try_from(7814135).unwrap(),
            options: Dictionary::default(),
            procedure: Uri::try_from("com.myapp.echo").unwrap(),
            arguments: List::from_iter([Value::String("Hello, world!".to_owned())]),
            arguments_keyword: Dictionary::default(),
        }));

        assert_serialize_to_deserialize_equal(&Message::Call(CallMessage {
            request: Id::try_from(7814135).unwrap(),
            options: Dictionary::default(),
            procedure: Uri::try_from("com.myapp.add2").unwrap(),
            arguments: List::from_iter([Value::Integer(23), Value::Integer(7)]),
            arguments_keyword: Dictionary::default(),
        }));

        assert_serialize_to_deserialize_equal(&Message::Call(CallMessage {
            request: Id::try_from(7814135).unwrap(),
            options: Dictionary::default(),
            procedure: Uri::try_from("com.myapp.user.new").unwrap(),
            arguments: List::from_iter([Value::String("Johnny".to_owned())]),
            arguments_keyword: Dictionary::from_iter([
                ("firstname".to_owned(), Value::String("John".to_owned())),
                ("surname".to_owned(), Value::String("Doe".to_owned())),
            ]),
        }));
    }
}
