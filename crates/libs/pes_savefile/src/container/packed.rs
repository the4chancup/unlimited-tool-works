//! The packed form: a save's decrypted sections and salt in one buffer. An
//! encrypted save does not compress (5 to 11 MB each); its sections do (25 to
//! 46 times), and the salt writes them back to the file byte for byte — so a
//! store can keep the packed form compressed and rebuild the file from it.
//!
//! Layout: byte 0 the scheme (0 = PES 15, `1 + i` for `MasterKey::ALL[i]`),
//! then the 320 salt bytes, then `description`, `logo`, `payload`,
//! `identifier` and `serial`, each a little-endian u32 length and its bytes.

use super::{ContainerError, MasterKey, SaveContainer, Scheme, salt_of};

/// `bytes` decrypted and packed under the salt `file_salt` would return.
/// The container's own errors when `bytes` is no save.
pub fn pack_file(bytes: &[u8]) -> Result<Vec<u8>, ContainerError> {
    let container = SaveContainer::decrypt(bytes)?;
    Ok(container.pack(&salt_of(container.scheme, bytes)))
}

/// `packed` (a `pack` form) read back and written with `to_bytes` under its
/// salt: the file `pack_file` read, byte for byte. `NotPacked` when `packed`
/// is not a packed form.
pub fn rebuild(packed: &[u8]) -> Result<Vec<u8>, ContainerError> {
    let (container, salt) = unpack(packed)?;
    container.to_bytes(&salt)
}

impl SaveContainer {
    /// The packed form this module's header describes: the scheme byte, the
    /// salt `to_bytes` writes the container back under, then each section
    /// length-prefixed.
    pub(crate) fn pack(&self, salt: &[u8; 320]) -> Vec<u8> {
        let scheme = match self.scheme {
            Scheme::Pes15 => 0,
            Scheme::Keyed(key) => MasterKey::ALL
                .iter()
                .position(|each| *each == key)
                .map(|index| index as u8 + 1)
                .expect("a container's key is one of MasterKey::ALL"),
        };
        let mut packed = Vec::with_capacity(1 + 320 + 5 * 4 + self.sections_len());
        packed.push(scheme);
        packed.extend_from_slice(salt);
        for section in self.sections() {
            packed.extend_from_slice(
                &u32::try_from(section.len())
                    .expect("a save's section is under 4 GB")
                    .to_le_bytes(),
            );
            packed.extend_from_slice(section);
        }
        packed
    }

    /// The five sections in packed order.
    fn sections(&self) -> [&Vec<u8>; 5] {
        [
            &self.description,
            &self.logo,
            &self.payload,
            &self.identifier,
            &self.serial,
        ]
    }

    /// The five sections' total byte count, for `pack`'s capacity.
    fn sections_len(&self) -> usize {
        self.sections().iter().map(|section| section.len()).sum()
    }
}

/// `packed` read into its container and salt. `NotPacked` when `packed` is
/// short, names a scheme past `MasterKey::ALL`, declares a length past its
/// end or has bytes after `serial`.
fn unpack(packed: &[u8]) -> Result<(SaveContainer, [u8; 320]), ContainerError> {
    let (scheme_byte, rest) = packed.split_first().ok_or(ContainerError::NotPacked)?;
    let scheme = match scheme_byte {
        0 => Scheme::Pes15,
        _ => MasterKey::ALL
            .get(usize::from(*scheme_byte) - 1)
            .copied()
            .map(Scheme::Keyed)
            .ok_or(ContainerError::NotPacked)?,
    };
    let (salt, mut rest) = rest
        .split_at_checked(320)
        .ok_or(ContainerError::NotPacked)?;
    let salt: [u8; 320] = salt.try_into().expect("split_at_checked(320) gave 320");
    let section = |rest: &mut &[u8]| -> Result<Vec<u8>, ContainerError> {
        let (length, tail) = rest.split_at_checked(4).ok_or(ContainerError::NotPacked)?;
        let length = u32::from_le_bytes(length.try_into().expect("4")) as usize;
        let (section, tail) = tail
            .split_at_checked(length)
            .ok_or(ContainerError::NotPacked)?;
        *rest = tail;
        Ok(section.to_vec())
    };
    let container = SaveContainer {
        scheme,
        description: section(&mut rest)?,
        logo: section(&mut rest)?,
        payload: section(&mut rest)?,
        identifier: section(&mut rest)?,
        serial: section(&mut rest)?,
    };
    if !rest.is_empty() {
        return Err(ContainerError::NotPacked);
    }
    Ok((container, salt))
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::{FIXTURES, fixture_container, test_salt};

    /// `version`'s fixture save's bytes (what `fixture_save` hands other
    /// crates' tests).
    fn fixture_save(version: PesVersion) -> Vec<u8> {
        fixture_container(version)
            .to_bytes(&test_salt())
            .expect("the fixture container encodes")
    }

    /// `form`'s `rebuild` error.
    fn rebuild_err(form: &[u8]) -> ContainerError {
        rebuild(form).expect_err("not a packed form")
    }

    // Every fixture's save packs and rebuilds to the file byte for byte:
    // the pack keeps the decrypted sections and the file's own salt, which
    // `to_bytes` reproduces it under.
    #[test]
    fn a_packed_save_rebuilds_byte_for_byte() {
        for version in FIXTURES {
            let save = fixture_save(version);
            let packed = pack_file(&save).expect("the fixture packs");
            assert_eq!(
                rebuild(&packed).expect("the packed form rebuilds"),
                save,
                "{version:?}: the rebuilt bytes are the file's"
            );
        }
    }

    // A packed form is refused as `NotPacked` when it is empty, one byte
    // short, names a scheme no `MasterKey` owns or carries a byte after
    // `serial`.
    #[test]
    fn a_malformed_packed_form_is_not_packed() {
        let save = fixture_save(PesVersion::Pes17);
        let packed = pack_file(&save).expect("the fixture packs");
        assert_eq!(rebuild_err(&[]), ContainerError::NotPacked);
        assert_eq!(
            rebuild_err(&packed[..packed.len() - 1]),
            ContainerError::NotPacked,
            "one byte short"
        );
        let mut bad_scheme = packed.clone();
        bad_scheme[0] = 8;
        assert_eq!(rebuild_err(&bad_scheme), ContainerError::NotPacked);
        let mut long = packed.clone();
        long.push(0);
        assert_eq!(rebuild_err(&long), ContainerError::NotPacked);
    }
}
