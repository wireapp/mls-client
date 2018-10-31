mod client;
mod repl;
mod state;

#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate rhai;
extern crate serde;

use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use client::*;
use repl::*;
use state::*;

fn main() {
    // HTTP client instance
    let client = reqwest::Client::new();

    // REPL instances
    let mut engine = rhai::Engine::new();
    let mut scope = rhai::Scope::new();

    // Groups that we're subscribed to
    let state: Arc<Mutex<HashMap<String, GroupState>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Set up polling
    let c = client.clone();
    let s = state.clone();
    thread::spawn(move || loop {
        poll(&c, s.clone());
        thread::sleep(Duration::from_secs(1));
    });

    register_types(&mut engine);
    register_functions(&client, state.clone(), &mut engine);

    loop {
        print!("> ");
        let mut input = String::new();
        stdout().flush().expect("Couldn't flush stdout");
        if let Err(e) = stdin().read_line(&mut input) {
            println!("Input error: {}", e);
        }

        match engine.eval_with_scope_boxed(&mut scope, &input) {
            Err(e) => println!("Error: {}", e),
            Ok(x) => println!("{:#?}", x),
        }
    }
}

/// Process a single message.
fn process_message(
    group_id: String,
    group_state: &mut GroupState,
    blob: Blob,
) {
    println!("{}: got {:?}", group_id, blob);
    if blob.index == group_state.next_blob {
        group_state.next_blob += 1;
    } else {
        println!("Wrong blob index")
    }
}

/// Poll for messages in subscribed groups.
fn poll(
    client: &reqwest::Client,
    state: Arc<Mutex<HashMap<String, GroupState>>>,
) {
    let mut state = state.lock().unwrap();
    for (group_id, group_state) in state.iter_mut() {
        let blobs =
            get_blobs(client, group_id, Some(group_state.next_blob), None)
                .unwrap();
        for blob in blobs {
            process_message(group_id.clone(), group_state, blob)
        }
    }
}
