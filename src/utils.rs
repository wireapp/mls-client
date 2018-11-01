use melissa::codec::Codec;
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
