pub mod client;
pub mod message;
pub mod polling;
pub mod repl;
pub mod settings;
pub mod state;
pub mod utils;

#[macro_use]
extern crate serde_derive;

extern crate config;
extern crate lazy_static;
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

use settings::Settings;
use state::State;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new().unwrap();
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

fn main() {
    // Read settings
    println!("{:?}", SETTINGS.server);

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
        utils::write_codec(
            format!("{}.pub", state.name),
            &state.credential,
        )
        .unwrap();
        utils::write_codec(
            format!("{}.init", state.name),
            &state.init_key_bundle.init_key,
        )
        .unwrap();
        println!("Wrote {}.pub and {}.init", state.name, state.name);
    }

    // Set up polling
    let s = state.clone();
    thread::spawn(move || loop {
        polling::poll(s.clone());
        thread::sleep(Duration::from_secs(1));
    });

    // Prepare the REPL
    repl::register_types(&mut engine);
    repl::register_functions(state, &mut engine);

    // Start the REPL
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
