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
use polling::*;
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

    // Export your public credentials.
    //
    // export()
    let s = state.clone();
    engine.register_fn("export", move || -> RhaiResult<()> {
        let state = s.lock().unwrap();
        write_codec(format!("{}.pub", state.name), &state.credential)
            .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))?;
        write_codec(
            format!("{}.init", state.name),
            &state.init_key_bundle.init_key,
        ).map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))?;
        Ok(())
    });

    // Create a group with the user as a single member.
    //
    // create(group_id)
    let s = state.clone();
    engine.register_fn(
        "create",
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
                        crypto: group_crypto,
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

    // Join a group. The welcome file has to be present.
    //
    // join(group_id)
    let s = state.clone();
    engine.register_fn("join", move |group_id: String| -> RhaiResult<()> {
        let mut state = s.lock().unwrap();
        join_group(&mut state, group_id)
            .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
    });

    // Do an update.
    //
    // update(group_id)
    let s = state.clone();
    let c = client.clone();
    engine.register_fn(
        "update",
        move |group_id: String| -> RhaiResult<()> {
            let mut state = s.lock().unwrap();
            do_update(&c, &mut state, group_id)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Quit the program.
    //
    // quit()
    // exit()
    engine.register_fn("quit", || {
        exit(0);
    });
    engine.register_fn("exit", || {
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
        // Read user info
        let credential = read_codec(format!("{}.pub", user_name))
            .map_err(|e| e.to_string())?;
        let init_key = read_codec(format!("{}.init", user_name))
            .map_err(|e| e.to_string())?;
        // Generate a welcome package
        let (welcome, add_raw) =
            group_state.crypto.create_add(credential, &init_key);
        let add_op = messages::GroupOperation {
            msg_type: messages::GroupOperationType::Add,
            group_operation: messages::GroupOperationValue::Add(add_raw),
        };
        // Process the add operation
        let blob = Blob {
            index: group_state.next_blob,
            content: Message(group_state.crypto.create_handshake(add_op)),
        };
        process_message(&group_id, group_state, blob.clone());
        // Send the operation;
        // TODO restart if sending fails
        append_blob(client, &group_id, &blob).map_err(|e| e.to_string())?;
        // Save the welcome package
        write_codec(
            format!("{}_{}.welcome", group_id, user_name),
            &welcome,
        ).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Group doesn't exist!".into())
    }
}

fn join_group(state: &mut State, group_id: String) -> Result<(), String> {
    let identity = state.identity.clone();
    if let hash_map::Entry::Vacant(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        // Import the group
        let welcome: messages::Welcome =
            read_codec(format!("{}_{}.welcome", group_id, state.name))
                .map_err(|e| e.to_string())?;
        let group_crypto =
            group::Group::new_from_welcome(identity, &welcome);
        let group_state = GroupState {
            crypto: group_crypto,
            // TODO: this will break if blobs can include things other than group operations
            next_blob: welcome.transcript.len() as i64,
        };
        entry_group_state.insert(group_state);
        Ok(())
    } else {
        Err("You're already a member of the group!".into())
    }
}

fn do_update(
    client: &reqwest::Client,
    state: &mut State,
    group_id: String,
) -> Result<(), String> {
    if let hash_map::Entry::Occupied(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        let mut group_state = entry_group_state.into_mut();
        let update_op = messages::GroupOperation {
            msg_type: messages::GroupOperationType::Update,
            group_operation: messages::GroupOperationValue::Update(
                group_state.crypto.create_update(),
            ),
        };
        let blob = Blob {
            index: group_state.next_blob,
            content: Message(
                group_state.crypto.create_handshake(update_op),
            ),
        };
        process_message(&group_id, group_state, blob.clone());
        // TODO: we should try resending the blob if the sending fails.
        append_blob(client, &group_id, &blob).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Group doesn't exist!".into())
    }
}
