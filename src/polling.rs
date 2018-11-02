extern crate reqwest;

use std::sync::{Arc, Mutex};

use client::*;
use message::*;
use state::*;

/// Process a single message.
pub fn process_message(
    group_id: &str,
    group_state: &mut GroupState,
    message: Blob<Message>,
) {
    println!("{}: got {:?}", group_id, message);
    // TODO: we skip blobs that are older than what we've seen, but we don't
    // check that they correspond to what we've seen.
    if message.index == group_state.next_blob {
        group_state.crypto.process_handshake(message.content.0);
        group_state.next_blob += 1;
    } else if message.index > group_state.next_blob {
        println!(
            "Blob from the future: expected index {}, got {}",
            group_state.next_blob, message.index
        )
    }
}

/// Poll for messages in subscribed groups and perform scheduled updates.
pub fn poll(client: &reqwest::Client, state: Arc<Mutex<State>>) {
    let mut state = state.lock().unwrap();
    for (group_id, group_state) in state.groups.iter_mut() {
        // Download blobs
        let blobs =
            get_blobs(client, group_id, Some(group_state.next_blob), None)
                .unwrap();
        for blob in blobs {
            process_message(&group_id, group_state, blob)
        }
    }
}
