use melissa::{group, keys};
use std::collections::HashMap;

use utils::*;

/// Group-related state that we track
#[derive(Clone, Serialize, Deserialize)]
pub struct GroupState {
    /// Blob index after the last one we've seen. (Would be 0 if no blobs
    /// were received at all, for instance.)
    pub next_blob: i64,

    /// The cryptographic group state.
    ///
    /// Note that the cryptographic group ID will be random, with no
    /// correlation to the `group_id` used elsewhere in the code.
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub crypto: group::Group,
}

/// All state that we track
#[derive(Serialize, Deserialize)]
pub struct State {
    pub name: String,
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub identity: keys::Identity,
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub credential: keys::BasicCredential,
    #[serde(
        serialize_with = "serialize_codec",
        deserialize_with = "deserialize_codec"
    )]
    pub init_key_bundle: keys::UserInitKeyBundle,
    pub groups: HashMap<String, GroupState>,
}

impl State {
    pub fn new(name: &str) -> Self {
        let identity = keys::Identity::random();
        State {
            name: name.into(),
            identity: identity.clone(),
            credential: keys::BasicCredential {
                identity: name.as_bytes().to_vec(),
                public_key: identity.public_key,
            },
            init_key_bundle: keys::UserInitKeyBundle::new(1, &identity),
            groups: HashMap::new(),
        }
    }
}
