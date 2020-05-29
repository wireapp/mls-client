//! Low-level logic for interacting with the server.

use crate::message::Message;
use serde::Serialize;
use serde_json::json;

use super::CLIENT;
use super::SETTINGS;

/// A blob, intended to be stored by the server. We can put any JSON we want
/// into blobs.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Blob {
    pub index: i64,
    pub content: Message,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Blobs {
    pub blobs: Vec<Blob>,
}

/// Store a blob for a specific group.
pub fn append_blob(group_id: &str, blob: &Blob) -> reqwest::Result<()> {
    let json = json!({
        "index": blob.index,
        "content": serde_json::to_string(&blob.content).unwrap()
    });

    println!(
        "append_blob: {}/groups/{}/blobs, blob: {:?}",
        SETTINGS.server, group_id, json
    );
    CLIENT
        .post(
            format!("{}/groups/{}/blobs", SETTINGS.server, group_id)
                .as_str(),
        )
        .json(&json)
        .send()?
        .error_for_status()
        .map(|_| ())
}

/// Receive all blobs for a specific groups.
pub fn get_blobs(
    group_id: &str,
    from: Option<i64>,
    to: Option<i64>,
) -> reqwest::Result<Blobs> {
    println!("get_blobs: {}/groups/{}/blobs", SETTINGS.server, group_id);
    let mut req = CLIENT.get(
        format!("{}/groups/{}/blobs", SETTINGS.server, group_id).as_str(),
    );
    if let Some(x) = from {
        req = req.query(&[("from", x)])
    };
    if let Some(x) = to {
        req = req.query(&[("to", x)])
    };
    req.send()?.json()
}
