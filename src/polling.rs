extern crate reqwest;
extern crate serde_json;

use std::fs::File;
use std::sync::{Arc, Mutex};

use crate::client::{get_blobs, Blob};
use crate::message::Message;
use crate::state::{GroupState, State};

/// Process a single message.
pub fn process_message(
    group_id: &str,
    group_state: &mut GroupState,
    message: Blob<Message>,
) {
    println!("{}: got {:?}", group_id, message);
    // TODO: we skip blobs that are older than what we've seen, but we don't check that they correspond to what we've seen.
    match message.index {
        ix if ix == group_state.next_blob => {
            group_state.crypto.process_handshake(message.content.0);
            group_state.next_blob += 1;
        }
        ix if ix > group_state.next_blob => println!(
            "Blob from the future: expected index {}, got {}",
            group_state.next_blob, message.index
        ),
        _ => {}
    }
}

/// Poll for messages in subscribed groups and perform scheduled updates.
///
/// Also save state to disk.
///
/// TODO this is hacky, doesn't belong here, and dumping unprotected private
/// keys isn't the best idea, too.
pub fn poll(state: Arc<Mutex<State>>) {
    let mut state = state.lock().unwrap();
    // Download blobs
    for (group_id, group_state) in state.groups.iter_mut() {
        let blobs =
            get_blobs(group_id, Some(group_state.next_blob), None).unwrap();
        for blob in blobs {
            process_message(&group_id, group_state, blob)
        }
    }
    // Save state to disk
    let file = File::create(format!("{}.state", state.name)).unwrap();
    serde_json::to_writer(file, &*state).unwrap();
}
