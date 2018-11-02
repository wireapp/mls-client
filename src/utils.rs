use melissa::codec::Codec;
use serde::Deserialize;
use serde::{de, ser};
use std::fs;
use std::io;
use std::path::Path;

/// Read a value from a file using `Codec`.
pub fn read_codec<P: AsRef<Path>, T: Codec>(path: P) -> io::Result<T> {
    Codec::decode_detached(fs::read(path)?.as_ref()).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
    })
}

/// Write a value into a file using `Codec`.
pub fn write_codec<P: AsRef<Path>, T: Codec>(
    path: P,
    value: &T,
) -> io::Result<()> {
    fs::write(path, Codec::encode_detached(value).as_slice())
}

/// Implement `Serialize` with `Codec`.
pub fn serialize_codec<C, S>(
    value: &C,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
    C: Codec,
{
    serializer.serialize_bytes(value.encode_detached().as_slice())
}

/// Implement `Deserialize` with `Codec`.
pub fn deserialize_codec<'de, C, D>(deserializer: D) -> Result<C, D::Error>
where
    D: de::Deserializer<'de>,
    C: Codec,
{
    Vec::deserialize(deserializer).and_then(|v| {
        C::decode_detached(v.as_slice())
            .map_err(|_| de::Error::custom("Failed to decode"))
    })
}
