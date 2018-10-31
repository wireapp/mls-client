extern crate reqwest;

use rhai::*;
use std::collections::hash_map;
use std::collections::HashMap;
use std::process::exit;
use std::sync::{Arc, Mutex};

use client::*;
use state::*;

pub fn register_types(engine: &mut Engine) {
    // All return types HAVE to be registered here, or else exception
    // handling won't work.
    engine.register_type::<()>();
    engine.register_type::<Blob>();
    engine.register_type::<Vec<Blob>>();
}

pub fn register_functions(
    client: &reqwest::Client,
    state: Arc<Mutex<HashMap<String, GroupState>>>,
    engine: &mut Engine,
) {
    // Create a blob:
    //
    // blob(index, content) -> Blob
    engine.register_fn("blob", |index: i64, content: String| -> Blob {
        Blob { index, content }
    });

    // Post a blob without adding it to the group state (though it will be
    // added anyway when doing polling):
    //
    // send(group_id, blob)
    let c = client.clone();
    engine.register_fn(
        "send",
        move |group_id: String, blob: Blob| -> RhaiResult<()> {
            append_blob(&c, group_id.as_str(), &blob)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Fetch all blobs without adding them to the group state:
    //
    // recv(group_id) -> Vec<Blob>
    // recv_from(group_id, from_index) -> Vec<Blob>
    // recv_to(group_id, to_index) -> Vec<Blob>
    // recv_from_to(group_id, from_index, to_index) -> Vec<Blob>
    let c = client.clone();
    engine.register_fn(
        "recv",
        move |group_id: String| -> RhaiResult<Vec<Blob>> {
            get_blobs(&c, group_id.as_str(), None, None)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );
    let c = client.clone();
    engine.register_fn(
        "recv_from",
        move |group_id: String, from: i64| -> RhaiResult<Vec<Blob>> {
            get_blobs(&c, group_id.as_str(), Some(from), None)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );
    let c = client.clone();
    engine.register_fn(
        "recv_to",
        move |group_id: String, to: i64| -> RhaiResult<Vec<Blob>> {
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
              -> RhaiResult<Vec<Blob>> {
            get_blobs(&c, group_id.as_str(), Some(from), Some(to))
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Subscribe to a group
    //
    // subscribe(group_id)
    engine.register_fn(
        "subscribe",
        move |group_id: String| -> RhaiResult<()> {
            let mut state = state.lock().unwrap();
            match state.entry(group_id) {
                hash_map::Entry::Occupied(_) => {
                    println!("Already subscribed!");
                }
                hash_map::Entry::Vacant(slot) => {
                    slot.insert(GroupState { next_blob: 0 });
                }
            }
            Ok(())
        },
    );

    // Quit the program:
    //
    // quit()
    engine.register_fn("quit", || {
        exit(0);
    });
}
