#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate rhai;
extern crate serde;

use rhai::{EvalAltResult, RegisterFn, RhaiResult};
use std::io::{stdin, stdout, Write};
use std::process::exit;

fn main() {
    let client = reqwest::Client::new();

    let mut engine = rhai::Engine::new();
    let mut scope = rhai::Scope::new();

    // All return types HAVE to be registered here, or else exception
    // handling won't work.
    engine.register_type::<()>();
    engine.register_type::<Blob>();
    engine.register_type::<Vec<Blob>>();

    // Create a blob:
    //
    // blob(index, content) -> Blob
    engine.register_fn("blob", |index: i64, content: String| -> Blob {
        Blob { index, content }
    });

    // Post a blob:
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

    // Fetch all blobs:
    //
    // recv(group_id) -> Vec<Blob>
    let c = client.clone();
    engine.register_fn(
        "recv",
        move |group_id: String| -> RhaiResult<Vec<Blob>> {
            get_blobs(&c, group_id.as_str(), None, None)
                .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string()))
        },
    );

    // Quit the program:
    //
    // quit()
    engine.register_fn("quit", || {
        exit(0);
    });

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

/// A blob, intended to be stored by the server. We can put any JSON we want
/// into blobs.
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Blob {
    index: i64,
    content: String, // should be JSON
}

/// Store a blob for a specific group.
fn append_blob(
    client: &reqwest::Client,
    group_id: &str,
    blob: &Blob,
) -> reqwest::Result<()> {
    client
        .post(
            format!("http://localhost:10100/groups/{}/blobs", group_id)
                .as_str(),
        ).json(blob)
        .send()?
        .error_for_status()
        .map(|_| ())
}

/// Receive all blobs for a specific groups.
fn get_blobs(
    client: &reqwest::Client,
    group_id: &str,
    from: Option<i64>,
    to: Option<i64>,
) -> reqwest::Result<Vec<Blob>> {
    let mut req = client.get(
        format!("http://localhost:10100/groups/{}/blobs", group_id)
            .as_str(),
    );
    if let Some(x) = from {
        req = req.query(&[("from", x)])
    };
    if let Some(x) = to {
        req = req.query(&[("to", x)])
    };
    req.send()?.json()
}
