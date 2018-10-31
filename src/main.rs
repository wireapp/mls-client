mod client;
mod repl;

#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate rhai;
extern crate serde;

use std::io::{stdin, stdout, Write};

use repl::*;

fn main() {
    let client = reqwest::Client::new();

    let mut engine = rhai::Engine::new();
    let mut scope = rhai::Scope::new();

    register_types(&mut engine);
    register_functions(&client, &mut engine);

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
