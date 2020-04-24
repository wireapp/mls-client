extern crate reqwest;
extern crate serde_json;

use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::client::{get_blobs, Blob};
use crate::state::{GroupState, State};

pub struct Polling {
    handle: Option<thread::JoinHandle<()>>
}

impl Polling {
    pub fn new() -> Polling {
        Polling { handle: Option::None }
    }

    pub fn start_polling(&mut self, state: Arc<Mutex<State>>) {
        self.stop_polling();
        self.handle = Option::Some(thread::spawn(move || loop {
            Polling::poll(state.clone());
            thread::sleep(Duration::from_secs(1));
        }));
    }

    pub fn stop_polling(&mut self) {
        if let Some(h) = self.handle.take() {
            h.join().expect("Handle panicked during joining");
            // TODO: this doesn't work yet. we need a way to stop the thread before it can be joined.
        }
    }

    /// Poll for messages in subscribed groups and perform scheduled updates.
    /// Also save state to disk.
    /// TODO this is hacky, doesn't belong here, and dumping unprotected private
    /// keys isn't the best idea, too.
    fn poll(state: Arc<Mutex<State>>) {
        println!("poll");
        let mut state = state.lock().unwrap();
        // Download blobs
        for (group_id, group_state) in state.groups.iter_mut() {
            let blobs =
                get_blobs(group_id, Some(group_state.next_blob), None).unwrap();
            for blob in blobs.blobs {
                process_message(&group_id, group_state, blob)
            }
        }
        // Save state to disk
        let file = File::create(format!("{}.state", state.name)).unwrap();
        serde_json::to_writer(file, &*state).unwrap();
    }
}

/// Process a single message.
pub fn process_message(
    group_id: &str,
    group_state: &mut GroupState,
    message: Blob,
) {
    println!("{}: got {:?}", group_id, message);
    // TODO: we skip blobs that are older than what we've seen, but we don't check that they correspond to what we've seen.
    match message.index {
        ix if ix == group_state.next_blob => {
            group_state.crypto.process_handshake(message.content.0);
            group_state.next_blob += 1;
        }
        ix if ix > group_state.next_blob => println!(
            "Blob from the future: expected index {}, got {}",
            group_state.next_blob, message.index
        ),
        _ => {}
    }
}


