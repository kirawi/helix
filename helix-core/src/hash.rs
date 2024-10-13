use std::io::{self, Read};

pub const HASH_DIGEST_LENGTH: usize = 20;

pub fn get_hash<R: Read>(reader: &mut R) -> io::Result<[u8; HASH_DIGEST_LENGTH]> {
    let mut hasher = tenthash::TentHasher::new();
    let mut buf = [0u8; 8192];

    // Read until empty
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hasher.finalize())
}

pub type Digest = [u8; HASH_DIGEST_LENGTH];
