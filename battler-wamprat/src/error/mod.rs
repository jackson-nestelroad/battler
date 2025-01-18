mod error;

pub use error::{
    WampratDeserializeError,
    WampratEventMissingTopic,
    WampratInvocationMissingProcedure,
    WampratSerializeError,
};
