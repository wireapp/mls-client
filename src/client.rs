//! Low-level logic for interacting with the server.

extern crate reqwest;

use serde::Serialize;
use settings::*;
use message::Message;

/// A blob, intended to be stored by the server. We can put any JSON we want
/// into blobs.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Blob<T> {
    pub index: i64,
    pub content: T,
}

/// Store a blob for a specific group.
pub fn append_blob<T: Serialize>(
    settings: &Settings,
    client: &reqwest::Client,
    group_id: &str,
    blob: &Blob<T>,
) -> reqwest::Result<()> {
    client
        .post(
            format!("{}/groups/{}/blobs", settings.server, group_id)
                .as_str(),
        ).json(blob)
        .send()?
        .error_for_status()
        .map(|_| ())
}

/// Receive all blobs for a specific groups.
pub fn get_blobs(
    settings: &Settings,
    client: &reqwest::Client,
    group_id: &str,
    from: Option<i64>,
    to: Option<i64>,
) -> reqwest::Result<Vec<Blob<Message>>> {
    let mut req = client.get(
        format!("{}/groups/{}/blobs", settings.server, group_id)
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
