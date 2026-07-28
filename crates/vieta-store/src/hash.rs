//! Word-at-a-time hashing for the intern tables.
//!
//! The store hashes millions of short keys that are already `u32` words, and it
//! is not exposed to adversarial input, so it uses a multiply-rotate mix rather
//! than a keyed construction.

const SEED: u32 = 0x9e37_79b9;

/// The starting state of a hash chain.
#[inline]
pub(crate) const fn seed() -> u32 {
    SEED
}

/// Fold one word into a hash chain.
#[inline]
pub(crate) const fn mix(hash: u32, word: u32) -> u32 {
    (hash.rotate_left(5) ^ word).wrapping_mul(SEED)
}

/// Hash a byte string, folding four bytes at a time and closing over the length
/// so that a trailing zero byte does not collide with its absence.
pub(crate) fn hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash = SEED;
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        hash = mix(hash, u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut tail = [0u8; 4];
        tail[..rest.len()].copy_from_slice(rest);
        hash = mix(hash, u32::from_le_bytes(tail));
    }
    mix(hash, bytes.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, mix, seed};

    #[test]
    fn mixing_is_order_sensitive() {
        assert_ne!(mix(mix(seed(), 1), 2), mix(mix(seed(), 2), 1));
    }

    #[test]
    fn byte_hashing_separates_length() {
        assert_ne!(hash_bytes(b"x"), hash_bytes(b"x\0"));
        assert_ne!(hash_bytes(b""), hash_bytes(b"\0"));
    }

    #[test]
    fn byte_hashing_is_deterministic() {
        assert_eq!(hash_bytes(b"Integrate"), hash_bytes(b"Integrate"));
        assert_ne!(hash_bytes(b"Integrate"), hash_bytes(b"Integrale"));
    }
}
