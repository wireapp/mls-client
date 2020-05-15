extern crate reqwest;
extern crate serde_json;

use melissa::group;
use melissa::keys;
use melissa::messages;
use rhai::*;
use std::collections::{hash_map, HashMap};
use std::fs::File;
use std::process::exit;
use std::sync::{Arc, Mutex, MutexGuard};
use rustyline::Editor;
use rustyline::error::ReadlineError;

use crate::client::{append_blob, get_blobs, Blob, Blobs};
use crate::message::Message;
use crate::polling::process_message;
use crate::state::{GroupState, State};
use crate::utils::{read_codec, write_codec};

use super::POLLING;
use super::REPL;

#[derive(Clone, Copy, Debug)]
pub enum REPLReturnType {
    Unit,
    Boolean,
    Message,
    Blob,
    Blobs,
    String,
    Strings,
    UnitResult,
    BlobsResult,
    StringsResult
}

impl Default for REPLReturnType {
    fn default() -> Self {
        REPLReturnType::Unit
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct REPLFunction {
    pub name: &'static str,
    pub return_type: REPLReturnType,
    // TODO: add the actual function here
}

#[derive(Clone, Debug, Default)]
pub struct REPLDictionary(HashMap<&'static str, REPLFunction>);

impl REPLDictionary {
    pub fn new() -> REPLDictionary {
        REPLDictionary(HashMap::new())
    }

    pub fn add(&mut self, name: &'static str, return_type: &REPLReturnType) {
        self.0.insert(name, REPLFunction { name: name, return_type: return_type.clone()});
    }

    pub fn get_return_type(&self, name: &'static str) -> REPLReturnType {
        self.0.get(name)
            .expect(format!("No function with name: {}", name).as_str())
            .return_type
    }
}

fn register_fn(name: &'static str, return_type: &REPLReturnType) {
    let mut repl = REPL.lock().unwrap();
    repl.add(name, return_type);
}

pub fn register_types(engine: &mut Engine) {
    // All return types HAVE to be registered here, or else exception
    // handling won't work.
    engine.register_type::<()>();
    engine.register_type::<Message>();
    engine.register_type::<Blob>();
    engine.register_type::<Blobs>();
    engine.register_type::<String>();
    engine.register_type::<Vec<String>>();

    engine.register_type::<Result<(), String>>();
    engine.register_type::<Result<Blobs, String>>();
    engine.register_type::<Result<Vec<String>, String>>()
}

// Create a blob.
//
// blob(index, message) -> Blob<Message>
fn blob(index: i64, content: Message) -> Blob {
    Blob { index, content }
}

// Post a blob without adding it to the group state (though it will be
// added anyway when doing polling).
//
// send(group_id, blob)
fn send(group_id: String, blob: Blob) -> Result<(), String> {
    append_blob(group_id.as_str(), &blob).map_err(|err| err.to_string())
}

// Fetch all blobs without adding them to the group state.
//
// recv(group_id) -> Vec<Blob<Message>>
// recv_from(group_id, from_index) -> Vec<Blob<Message>>
// recv_to(group_id, to_index) -> Vec<Blob<Message>>
// recv_from_to(group_id, from_index, to_index) -> Vec<Blob<Message>>

fn recv(group_id: String) -> Result<Blobs, String> {
    get_blobs(group_id.as_str(), None, None)
        .map_err(|err| err.to_string())
}

fn recv_from(
    group_id: String,
    from: i64,
) -> Result<Blobs, String> {
    get_blobs(group_id.as_str(), Some(from), None)
        .map_err(|err| err.to_string())
}

fn recv_to(
    group_id: String,
    to: i64,
) -> Result<Blobs, String> {
    get_blobs(group_id.as_str(), None, Some(to))
        .map_err(|err| err.to_string())
}

fn recv_from_to(
    group_id: String,
    from: i64,
    to: i64,
) -> Result<Blobs, String> {
    get_blobs(group_id.as_str(), Some(from), Some(to))
        .map_err(|err| err.to_string())
}

pub fn register_functions(state: Arc<Mutex<State>>, engine: &mut Engine) {
    // TODO: write a macro which can take any type of function, just us engine.register_fn
    engine.register_fn("blob", blob);
    register_fn("blob", &REPLReturnType::Blob);
    engine.register_fn("send", send);
    register_fn("send", &REPLReturnType::UnitResult);
    engine.register_fn("recv", recv);
    register_fn("recv", &REPLReturnType::BlobsResult);
    engine.register_fn("recv_from", recv_from);
    register_fn("recv_from", &REPLReturnType::BlobsResult);
    engine.register_fn("recv_to", recv_to);
    register_fn("recv_to", &REPLReturnType::BlobsResult);
    engine.register_fn("recv_from_to", recv_from_to);
    register_fn("recv_from_to", &REPLReturnType::BlobsResult);

    // Create a group with the user as a single member.
    //
    // create(group_id)
    let create_closure = |state: Arc<Mutex<State>>| {
        move |group_id: String| -> Result<(), String> {
            create_group(state.clone(), group_id)
        }
    };
    engine.register_fn("create", create_closure(state.clone()));
    register_fn("create", &REPLReturnType::UnitResult);

    // Add a user to a group and generate an invitation file for them.
    // Assumes that the user's data is stored in `<user>.pub` and
    // `<user>.init`. Saves the welcome package to `<group>_<user>.welcome`.
    //
    // add(group_id, user_name)
    let add_closure = |state: Arc<Mutex<State>>| {
        move |group_id: String, user_name: String| -> Result<(), String> {
            add_to_group(state.clone(), group_id, user_name)
                .map_err(|err| { println!("{}", err); err } )
        }
    };
    engine.register_fn("add", add_closure(state.clone()));
    register_fn("add", &REPLReturnType::UnitResult);

    // Add the current user to a group and generate an invitation file for them.
    // Assumes that the user's data is stored in `<user>.pub` and
    // `<user>.init`. Saves the welcome package to `<group>_<user>.welcome`.
    //
    // add_self(group_id)
    let add_self_closure = |state: Arc<Mutex<State>>| {
        move |group_id: String| -> Result<(), String> {
            add_self_to_group(state.clone(), group_id)
                .map_err(|err| { println!("{}", err); err } )
        }
    };
    engine.register_fn("add_self", add_self_closure(state.clone()));
    register_fn("add_self", &REPLReturnType::UnitResult);

    // Join a group. The welcome file has to be present.
    //
    // join(group_id)
    let join_closure = |state: Arc<Mutex<State>>| {
        move |group_id: String| -> Result<(), String> {
            join_group(state.clone(), group_id)
        }
    };
    engine.register_fn("join", join_closure(state.clone()));
    register_fn("join", &REPLReturnType::UnitResult);

    // Do an update.
    //
    // update(group_id)
    let update_closure = |s: Arc<Mutex<State>>| {
        move |group_id: String| -> Result<(), String> {
            let mut state = s.lock().unwrap();
            do_update(&mut state, group_id)
        }
    };
    engine.register_fn("update", update_closure(state.clone()));
    register_fn("update", &REPLReturnType::UnitResult);

    // Remove a user from the group. Assumes that the user's data is stored
    // in `<user>.pub` and `<user>.init`.
    //
    // remove(group_id, user_name)
    let remove_closure = |s: Arc<Mutex<State>>| {
        move |group_id: String, user_name: String| -> Result<(), String> {
            let mut state = s.lock().unwrap();
            remove_from_group(&mut state, group_id, user_name)
        }
    };
    engine.register_fn("remove", remove_closure(state.clone()));
    register_fn("remove", &REPLReturnType::UnitResult);

    // See group's roster.
    //
    // roster(group_id)
    let roster_closure = |s: Arc<Mutex<State>>| {
        move |group_id: String| -> Result<Vec<String>, String> {
            let state = s.lock().unwrap();
            if let Some(group_state) = state.groups.get(&group_id) {
                Ok(group_state
                    .crypto
                    .get_members()
                    .iter()
                    .map(|cred| {
                        String::from_utf8_lossy(&cred.identity).into()
                    })
                    .collect())
            } else {
                Err("Unknown group!".into())
            }
        }
    };
    engine.register_fn("roster", roster_closure(state.clone()));
    register_fn("roster", &REPLReturnType::StringsResult);

    // List groups.
    //
    // list()
    let list_closure = |s: Arc<Mutex<State>>| {
        move || -> Result<Vec<String>, String> {
            let state = s.lock().unwrap();
            Ok(state.groups.keys().cloned().collect())
        }
    };
    engine.register_fn("list", list_closure(state.clone()));
    register_fn("list", &REPLReturnType::StringsResult);

    // Load state from disk (from `<user>.state`).
    //
    // load(user_name)
    let load_closure = |s: Arc<Mutex<State>>| {
        move |user_name: String| -> Result<(), String> {
            let mut state = s.lock().unwrap();
            let file = File::open(format!("{}.state", user_name))
                .map_err(|e| e.to_string())?;
            *state =
                serde_json::from_reader(file).map_err(|e| e.to_string())?;
            println!("Loaded {}", state.name);
            Ok(())
        }
    };
    engine.register_fn("load", load_closure(state.clone()));
    register_fn("load", &REPLReturnType::UnitResult);

    // Quit the program.
    //
    // quit()
    // exit()
    engine.register_fn("quit", || { exit(0); });
    register_fn("quit", &REPLReturnType::Unit);
    engine.register_fn("exit", || { exit(0); });
    register_fn("exit", &REPLReturnType::Unit);

    // Start querying the server for data
    engine.register_fn("start_poll", move || {
        let mut poll = POLLING.lock().unwrap();
        poll.start_polling(state.clone());
    });
    register_fn("start_poll", &REPLReturnType::Unit);

    // Stop querying the server for data
    engine.register_fn("stop_poll", || {
        let mut poll = POLLING.lock().unwrap();
        poll.stop_polling();
    });
    register_fn("stop_poll", &REPLReturnType::Unit);

    engine.register_fn("is_polling", || {
        let poll = POLLING.lock().unwrap();
        poll.is_polling()
    });
    register_fn("is_polling", &REPLReturnType::Boolean);
}

pub fn start_repl(engine: &mut Engine) {
    // Start the REPL
    let mut scope = rhai::Scope::new();
    let mut rl = Editor::<()>::new();
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                if let Err(e) = engine.consume_with_scope(&mut scope, &line)
                {
                    println!("Error: {}", e)
                }
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

fn add_self_to_group(
    st: Arc<Mutex<State>>,
    group_id: String,
) -> Result<(), String> {
    let mut state = st.lock().unwrap();
    let name = state.name.clone();
    _add_to_group(&mut state, group_id, name.as_str())
}

fn add_to_group(
    st: Arc<Mutex<State>>,
    group_id: String,
    user_name: String,
) -> Result<(), String> {
    let mut state = st.lock().unwrap();
    _add_to_group(&mut state, group_id, user_name.as_str())
}

fn _add_to_group(
    state: &mut MutexGuard<State>,
    group_id: String,
    user_name: &str,
) -> Result<(), String> {
    println!("add to group {}, {}", group_id, user_name);
    if let hash_map::Entry::Occupied(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        let group_state = entry_group_state.into_mut();
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
        append_blob(&group_id, &blob).map_err(|e| e.to_string())?;
        // Save the welcome package
        write_codec(
            format!("{}_{}.welcome", group_id, user_name),
            &welcome,
        )
        .map_err(|e| e.to_string())?;
        println!("Wrote {}_{}.welcome", group_id, user_name);
        Ok(())
    } else {
        Err("Group doesn't exist!".into())
    }
}

fn join_group(
    st: Arc<Mutex<State>>,
    group_id: String,
) -> Result<(), String> {
    let mut state = st.lock().unwrap();
    let state_name = state.name.clone();
    let identity = state.identity.clone();
    if let hash_map::Entry::Vacant(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        // Import the group
        let welcome: messages::Welcome =
            read_codec(format!("{}_{}.welcome", group_id, state_name))
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

fn do_update(state: &mut State, group_id: String) -> Result<(), String> {
    if let hash_map::Entry::Occupied(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        let group_state = entry_group_state.into_mut();
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
        append_blob(&group_id, &blob).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Group doesn't exist!".into())
    }
}

fn remove_from_group(
    state: &mut State,
    group_id: String,
    user_name: String,
) -> Result<(), String> {
    if let hash_map::Entry::Occupied(entry_group_state) =
        state.groups.entry(group_id.clone())
    {
        let group_state = entry_group_state.into_mut();
        // Find the user; we can't find them by username because we don't
        // get usernames from add operations, so we have to look at the key
        let credential: keys::BasicCredential =
            read_codec(format!("{}.pub", user_name))
                .map_err(|e| e.to_string())?;
        let slot = group_state
            .crypto
            .get_members()
            .iter()
            .position(|k| k.public_key == credential.public_key);
        if let Some(slot) = slot {
            // Create a remove operation
            let remove_raw = group_state.crypto.create_remove(slot);
            let remove_op = messages::GroupOperation {
                msg_type: messages::GroupOperationType::Remove,
                group_operation: messages::GroupOperationValue::Remove(
                    remove_raw,
                ),
            };
            // Process the remove operation
            let blob = Blob {
                index: group_state.next_blob,
                content: Message(
                    group_state.crypto.create_handshake(remove_op),
                ),
            };
            process_message(&group_id, group_state, blob.clone());
            // Send the operation;
            // TODO restart if sending fails
            append_blob(&group_id, &blob).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("User not found!".into())
        }
    } else {
        Err("Group doesn't exist!".into())
    }
}

fn create_group(
    state: Arc<Mutex<State>>,
    group_id: String,
) -> Result<(), String> {
    let mut st = state.lock().unwrap();
    let identity = st.identity.clone();
    let credential = st.credential.clone();
    match st.groups.entry(group_id) {
        hash_map::Entry::Occupied(_) => Err("Group already exists!".into()),
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
            Ok(())
        }
    }
}
