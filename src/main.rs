extern crate rhai;

use rhai::{Engine, RegisterFn, Scope};
use std::io::*;

fn main() {
    let mut engine = Engine::new();
    let mut scope = Scope::new();

    engine.register_fn("send", append_blob);
    engine.register_fn("recv", get_blobs);

    loop {
        print!("> ");
        let mut input = String::new();
        stdout().flush().expect("couldn't flush stdout");
        if let Err(e) = stdin().read_line(&mut input) {
            println!("input error: {}", e);
        }

        if let Err(e) = engine.consume_with_scope(&mut scope, &input) {
            println!("error: {}", e);
        }
    }
}

/// Store a blob for a specific group.
pub fn append_blob(group_id: String, blob: String) {
    println!("{} <>= {}", group_id, blob)
}

/// Receive all blobs for a specific groups.
pub fn get_blobs(group_id: String) {
    println!("{}...", group_id)
}
