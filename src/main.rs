mod client;
mod repl;
mod state;

#[macro_use]
extern crate serde_derive;

extern crate melissa;
extern crate names;
extern crate reqwest;
extern crate rhai;
extern crate rustyline;
extern crate serde;

use rustyline::error::ReadlineError;
use rustyline::Editor;
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

    // Local state
    let name = names::Generator::default().next().unwrap();
    let state: Arc<Mutex<State>> =
        Arc::new(Mutex::new(State::new(name.as_str())));

    // Set up polling
    let c = client.clone();
    let s = state.clone();
    thread::spawn(move || loop {
        poll(&c, s.clone());
        thread::sleep(Duration::from_secs(1));
    });

    // Prepare the REPL
    register_types(&mut engine);
    register_functions(&client, state.clone(), &mut engine);

    // Start the REPL
    let mut rl = Editor::<()>::new();
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());
                match engine.eval_with_scope_boxed(&mut scope, &line) {
                    Err(e) => println!("Error: {}", e),
                    Ok(x) => println!("{:#?}", x),
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
fn poll(client: &reqwest::Client, state: Arc<Mutex<State>>) {
    let mut state = state.lock().unwrap();
    for (group_id, group_state) in state.groups.iter_mut() {
        let blobs =
            get_blobs(client, group_id, Some(group_state.next_blob), None)
                .unwrap();
        for blob in blobs {
            process_message(group_id.clone(), group_state, blob)
        }
    }
}
