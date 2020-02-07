use melissa::messages;
use std::fmt;

use crate::utils::{deserialize_codec, serialize_codec};

/// Any kind of message stored by the server.
#[derive(Clone, Serialize, Deserialize)]
pub struct Message(
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub messages::Handshake,
);

impl fmt::Debug for Message {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let Message(handshake) = self;
        fmt.debug_tuple("Handshake")
            .field(&handshake.operation.msg_type)
            .finish()
    }
}
