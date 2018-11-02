use melissa::codec::Codec;
use melissa::messages;
use serde::Deserialize;
use serde::{de, ser};
use std::fmt;

/// Any kind of message stored by the server.
#[derive(Clone, Serialize, Deserialize)]
pub struct Message(
    #[serde(
        serialize_with = "serialize_handshake",
        deserialize_with = "deserialize_handshake"
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

/// Serialize a `Handshake` inside the message.
fn serialize_handshake<S>(
    x: &messages::Handshake,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    serializer.serialize_bytes(x.encode_detached().as_slice())
}

/// Deserialize a `Handshake` inside the message.
fn deserialize_handshake<'de, D>(
    deserializer: D,
) -> Result<messages::Handshake, D::Error>
where
    D: de::Deserializer<'de>,
{
    Vec::deserialize(deserializer).and_then(|v| {
        messages::Handshake::decode_detached(v.as_slice())
            .map_err(|_| de::Error::custom("Failed to decode a Handshake"))
    })
}
