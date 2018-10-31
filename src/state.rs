use melissa::keys;
use std::collections::HashMap;

/// Group-related state that we track
pub struct GroupState {
    /// The blob after the last one we've seen. (Would be 0 if no blobs were
    /// received at all, for instance.)
    pub next_blob: i64,
}

/// All state that we track
pub struct State {
    pub identity: keys::Identity,
    pub credential: keys::BasicCredential,
    pub init_key_bundle: keys::UserInitKeyBundle,
    pub groups: HashMap<String, GroupState>,
}

impl State {
    pub fn new(name: &str) -> Self {
        let identity = keys::Identity::random();
        State {
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
