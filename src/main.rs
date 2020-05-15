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

use std::sync::{Arc, Mutex};

use settings::Settings;
use state::State;

use lazy_static::lazy_static;
use crate::polling::Polling;
use crate::repl::REPLDictionary;

lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new().unwrap();
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
    pub static ref POLLING: Arc<Mutex<Polling>> =  Arc::new(Mutex::new(Polling::new()));
    pub static ref REPL: Arc<Mutex<repl::REPLDictionary>> = Arc::new(Mutex::new(REPLDictionary::new()));
}

fn main() {
    // Read settings
    println!("{:?}", SETTINGS.server);

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

    // REPL instances
    let mut engine = rhai::Engine::new();
    // Prepare the REPL
    repl::register_types(&mut engine);
    repl::register_functions(state, &mut engine);

    // Start the REPL
    repl::start_repl(&mut engine);
}


