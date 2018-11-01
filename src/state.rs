use melissa::{group, keys};
use std::collections::HashMap;

/// Group-related state that we track
pub struct GroupState {
    /// Blob index after the last one we've seen. (Would be 0 if no blobs
    /// were received at all, for instance.)
    pub next_blob: i64,

    /// The cryptographic state, if we're in the group.
    ///
    /// Note that the cryptographic group ID will be random.
    pub crypto: Option<group::Group>,

    /// Whether we should perform an update as soon as possible (e.g. if we
    /// were just added to the group).
    pub should_update: bool,
}

/// All state that we track
pub struct State {
    pub name: String,
    pub identity: keys::Identity,
    pub credential: keys::BasicCredential,
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
