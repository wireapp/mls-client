extern crate reqwest;

use melissa::messages;
use std::sync::{Arc, Mutex};

use client::*;
use message::*;
use state::*;
use settings::*;

/// Process a single message.
pub fn process_message(
    group_id: String,
    group_state: &mut GroupState,
    message: Blob<Message>,
) {
    println!("{}: got {:?}", group_id, message);
    if message.index == group_state.next_blob {
        if let Some(ref mut group) = group_state.crypto {
            group.process_handshake(message.content.0);
        }
        group_state.next_blob += 1;
    } else {
        println!("Wrong blob index")
    }
}

/// Poll for messages in subscribed groups and perform scheduled updates.
pub fn poll(settings: &Settings, client: &reqwest::Client, state: Arc<Mutex<State>>) {
    let mut state = state.lock().unwrap();
    for (group_id, group_state) in state.groups.iter_mut() {
        // Download blobs
        let blobs =
            get_blobs(&settings, client, group_id, Some(group_state.next_blob), None)
                .unwrap();
        for blob in blobs {
            process_message(group_id.clone(), group_state, blob)
        }
        // Perform an update, if necessary
        if group_state.should_update {
            if let Some(ref mut group) = group_state.crypto {
                let update_op = messages::GroupOperation {
                    msg_type: messages::GroupOperationType::Update,
                    group_operation: messages::GroupOperationValue::Update(
                        group.create_update(),
                    ),
                };
                append_blob(
                    &settings,
                    client,
                    group_id.as_ref(),
                    &Blob {
                        index: group_state.next_blob,
                        content: Message(group.create_handshake(update_op)),
                    },
                ).unwrap_or_else(|err| println!("Error: {}", err));
            }
        }
    }
}
