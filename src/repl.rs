extern crate reqwest;
extern crate serde_json;

use melissa::group;
use melissa::messages;
use rhai::*;
use std::collections::hash_map;
use std::process::exit;
use std::sync::{Arc, Mutex};

use client::*;
use message::*;
use state::*;
use utils::*;

pub fn register_types(engine: &mut Engine) {
    // All return types HAVE to be registered here, or else exception
    // handling won't work.
    engine.register_type::<()>();
    engine.register_type::<Message>();
    engine.register_type::<Blob<Message>>();
    engine.register_type::<Vec<Blob<Message>>>();
}

pub fn register_functions(
    client: &reqwest::Client,
    state: Arc<Mutex<State>>,
    engine: &mut Engine,
) {
    // Create a blob.
    //
    // blob(index, message) -> Blob<Message>
    engine.register_fn(
        "blob",
        |index: i64, content: Message| -> Blob<Message> {
            Blob { index, content }
        },
    );

    // Post a blob without adding it to the group state (though it will be
    // added anyway when doing polling).
    //
    // send(group_id, blob)
    let c = client.clone();
    engine.register_fn(
        "send",
        move |group_id: String, blob: Blob<Message>| -> RhaiResult<()> {
            append_blob(&c, group_id.as_str(), &blob)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Fetch all blobs without adding them to the group state.
    //
    // recv(group_id) -> Vec<Blob<Message>>
    // recv_from(group_id, from_index) -> Vec<Blob<Message>>
    // recv_to(group_id, to_index) -> Vec<Blob<Message>>
    // recv_from_to(group_id, from_index, to_index) -> Vec<Blob<Message>>
    let c = client.clone();
    engine.register_fn(
        "recv",
        move |group_id: String| -> RhaiResult<Vec<Blob<Message>>> {
            get_blobs(&c, group_id.as_str(), None, None)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );
    let c = client.clone();
    engine.register_fn(
        "recv_from",
        move |group_id: String,
              from: i64|
              -> RhaiResult<Vec<Blob<Message>>> {
            get_blobs(&c, group_id.as_str(), Some(from), None)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );
    let c = client.clone();
    engine.register_fn(
        "recv_to",
        move |group_id: String,
              to: i64|
              -> RhaiResult<Vec<Blob<Message>>> {
            get_blobs(&c, group_id.as_str(), None, Some(to))
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );
    let c = client.clone();
    engine.register_fn(
        "recv_from_to",
        move |group_id: String,
              from: i64,
              to: i64|
              -> RhaiResult<Vec<Blob<Message>>> {
            get_blobs(&c, group_id.as_str(), Some(from), Some(to))
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Subscribe to a group. This is possible even if the user does not
    // belong to the group (they can spy on group operations but they can't
    // usefully interpret them).
    //
    // subscribe(group_id)
    let s = state.clone();
    engine.register_fn(
        "subscribe",
        move |group_id: String| -> RhaiResult<()> {
            let mut state = s.lock().unwrap();
            match state.groups.entry(group_id) {
                hash_map::Entry::Occupied(_) => {
                    println!("Already subscribed!");
                }
                hash_map::Entry::Vacant(slot) => {
                    slot.insert(GroupState {
                        next_blob: 0,
                        crypto: None,
                    });
                }
            }
            Ok(())
        },
    );

    // Create a group with the user as a single member.
    //
    // create_group(group_id)
    let s = state.clone();
    engine.register_fn(
        "create_group",
        move |group_id: String| -> RhaiResult<()> {
            let mut state = s.lock().unwrap();
            let identity = state.identity.clone();
            let credential = state.credential.clone();
            match state.groups.entry(group_id) {
                hash_map::Entry::Occupied(_) => {
                    println!("Group already exists!");
                }
                hash_map::Entry::Vacant(slot) => {
                    let group_crypto = group::Group::new(
                        identity,
                        credential,
                        group::GroupId::random(),
                    );
                    slot.insert(GroupState {
                        next_blob: 0,
                        crypto: Some(group_crypto),
                    });
                }
            }
            Ok(())
        },
    );

    // Add a user to a group and generate an invitation file for them.
    // Assumes that the user's data is stored in `<user>.pub` and
    // `<user>.init`. Saves the welcome package to `<group>_<user>.welcome`.
    //
    // add(group_id, user_name)
    let s = state.clone();
    let c = client.clone();
    engine.register_fn(
        "add",
        move |group_id: String, user_name: String| -> RhaiResult<()> {
            let mut state = s.lock().unwrap();
            add_to_group(&c, &mut state, group_id, user_name)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Quit the program.
    //
    // quit()
    engine.register_fn("quit", || {
        exit(0);
    });
}

fn add_to_group(
    client: &reqwest::Client,
    state: &mut State,
    group_id: String,
    user_name: String,
) -> Result<(), String> {
    if let hash_map::Entry::Occupied(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        let mut group_state = entry_group_state.into_mut();
        if let Some(ref mut group_crypto) = group_state.crypto {
            // Read user info
            let credential = read_codec(format!("{}.pub", user_name))
                .map_err(|e| e.to_string())?;
            let init_key = read_codec(format!("{}.init", user_name))
                .map_err(|e| e.to_string())?;
            // Generate a welcome package
            let (welcome, add_op) =
                group_crypto.create_add(credential, &init_key);
            // Send the `add_op`;
            // TODO restart if the index is wrong
            let message = Message::Operation(messages::GroupOperation {
                msg_type: messages::GroupOperationType::Add,
                group_operation: messages::GroupOperationValue::Add(add_op),
            });
            append_blob(
                client,
                group_id.as_ref(),
                &Blob {
                    index: group_state.next_blob,
                    content: message,
                },
            ).map_err(|e| e.to_string())?;
            // Save the welcome package
            write_codec(
                format!("{}_{}.welcome", group_id, user_name),
                &welcome,
            ).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("You're not a part of the group \
                 (even though you're subscribed to the updates"
                .into())
        }
    } else {
        Err("Group doesn't exist!".into())
    }
}

/*

    // Alice adds Bob
    let (welcome_alice_bob, add_alice_bob) = group_alice.create_add(bob_credential, &bob_init_key);
    group_alice.process_add(&add_alice_bob);

    let mut group_bob = Group::new_from_welcome(bob_identity, &welcome_alice_bob);
    assert_eq!(group_alice.get_init_secret(), group_bob.get_init_secret());

    // Bob updates
    let update_bob = group_bob.create_update();
    group_bob.process_update(1, &update_bob);
    group_alice.process_update(1, &update_bob);
    assert_eq!(group_alice.get_init_secret(), group_bob.get_init_secret());

    // Alice updates
    let update_alice = group_alice.create_update();
    group_alice.process_update(0, &update_alice);
    group_bob.process_update(0, &update_alice);

*/
