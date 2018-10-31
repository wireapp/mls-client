pub struct GroupState {
    /// The blob after the last one we've seen. (Would be 0 if no blobs were
    /// received at all, for instance.)
    pub next_blob: i64,
}
