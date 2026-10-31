//! The aesthetics fingerprint: a hash of the ingame-face run normalized by
//! masking the player-gloves and skin-colour bits (`IngameFace::normalized`,
//! seven bits of fields the comparison reports on their own). It is
//! how `ops::compare` says "the face changed" without decoding every facial
//! parameter, and how a fingerprint printed elsewhere can be checked against
//! ours.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::model::ingame_face::IngameFace;

/// The first four bytes of SHA-256 over `IngameFace::normalized()`; `Display`
/// is the eight lower-case hex characters of the digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceHash(pub [u8; 4]);

impl fmt::Display for FaceHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// `face`'s fingerprint.
pub fn face_hash(face: &IngameFace) -> FaceHash {
    let digest = Sha256::digest(face.normalized());
    FaceHash(digest[..4].try_into().expect("SHA-256 is 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_is_the_scripts_eight_hex_chars() {
        // A run with no two equal bytes.
        let face = IngameFace::from_bytes((0..50).collect());
        let digest = Sha256::digest(face.normalized());
        let expected: String = digest[..4].iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(face_hash(&face).to_string(), expected);
        assert_eq!(expected.len(), 8);
    }

    #[test]
    fn an_unread_run_hashes_without_panic() {
        let hash = face_hash(&IngameFace::default());
        assert_eq!(hash.to_string().len(), 8);
    }
}
