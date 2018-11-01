use melissa::codec::Codec;
use melissa::messages;
use serde::Deserialize;
use serde::{de, ser};
use std::fmt;

/// Any kind of message stored by the server.
#[derive(Clone, Serialize, Deserialize)]
pub enum Message {
    Operation(
        #[serde(
            serialize_with = "serialize_group_operation",
            deserialize_with = "deserialize_group_operation"
        )]
        messages::GroupOperation,
    ),
}

impl fmt::Debug for Message {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Operation(op) => fmt
                .debug_tuple("Operation")
                .field(&op.msg_type)
                .field(&format_args!("<not shown>"))
                .finish(),
        }
    }
}

// TODO use Codec, not Serialize/Deserialize

fn serialize_group_operation<S>(
    x: &messages::GroupOperation,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    serializer.serialize_bytes(x.encode_detached().as_slice())
}

fn deserialize_group_operation<'de, D>(
    deserializer: D,
) -> Result<messages::GroupOperation, D::Error>
where
    D: de::Deserializer<'de>,
{
    Vec::deserialize(deserializer).and_then(|v| {
        messages::GroupOperation::decode_detached(v.as_slice()).map_err(
            |_| de::Error::custom("Failed to decode a GroupOperation"),
        )
    })
}
