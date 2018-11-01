extern crate reqwest;

use melissa::messages;
use std::sync::{Arc, Mutex};

use client::*;
use message::*;
use state::*;

/// Process a single message.
pub fn process_message(
    group_id: String,
    group_state: &mut GroupState,
    message: Blob<Message>,
) {
    println!("{}: got {:?}", group_id, message);
    if message.index == group_state.next_blob {
        // TODO: actually process the message
        group_state.next_blob += 1;
    } else {
        println!("Wrong blob index")
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
            process_message(group_id.clone(), group_state, blob)
        }
        // Perform an update, if necessary
        if group_state.should_update {
            if let Some(ref mut group) = group_state.crypto {
                let update_op = group.create_update();
                let message = Message::Operation(
                    messages::GroupOperation {
                        msg_type: messages::GroupOperationType::Update,
                        group_operation:
                            messages::GroupOperationValue::Update(update_op),
                    },
                );
                append_blob(
                    client,
                    group_id.as_ref(),
                    &Blob {
                        index: group_state.next_blob,
                        content: message,
                    },
                ).unwrap_or_else(|err| println!("Error: {}", err));
            }
        }
    }
}
