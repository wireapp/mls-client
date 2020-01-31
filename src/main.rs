mod client;
mod message;
mod polling;
mod repl;
mod state;
mod utils;
mod settings;

#[macro_use]
extern crate serde_derive;

extern crate melissa;
extern crate names;
extern crate reqwest;
extern crate rhai;
extern crate rustyline;
extern crate serde;
extern crate config;

use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use polling::*;
use repl::*;
use state::*;
use settings::*;
use utils::*;

fn main() {
    // Read settings
    let settings = Settings::new().unwrap();
    println!("{:?}", settings);

    // HTTP client instance
    let client = reqwest::Client::new();

    // REPL instances
    let mut engine = rhai::Engine::new();
    let mut scope = rhai::Scope::new();

    // Local state
    let name = names::Generator::default().next().unwrap();
    let state: Arc<Mutex<State>> =
        Arc::new(Mutex::new(State::new(name.as_str())));
    println!("\nCreated new user '{}'", name);

    // Write user's keys
    {
        let state = state.lock().unwrap();
        write_codec(format!("{}.pub", state.name), &state.credential)
            .unwrap();
        write_codec(
            format!("{}.init", state.name),
            &state.init_key_bundle.init_key,
        ).unwrap();
        println!("Wrote {}.pub and {}.init", state.name, state.name);
    }

    // Set up polling
    let c = client.clone();
    let s = state.clone();
    let set = settings.clone();
    thread::spawn(move || loop {
        poll(&set, &c, s.clone());
        thread::sleep(Duration::from_secs(1));
    });

    // Prepare the REPL
    register_types(&mut engine);
    register_functions(&settings, &client, state.clone(), &mut engine);

    // Start the REPL
    let mut rl = Editor::<()>::new();
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                if let Err(e) = engine.consume_with_scope(&mut scope, &line) {
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
