//! The savefile container: bytes ↔ decrypted sections. Two schemes exist — the PES
//! 16-21 MT19937 stream under one of the master keys, and the PES 15 LCG/MD5 chunk
//! format — but the API is one struct: `file.rs` composes it with the codec the same
//! way whichever scheme held the bytes.
//!
//! `decrypt` tries the seven master keys (header decrypt + 64-byte effective-key
//! compare, cheap), then the PES 15 shape; `to_bytes` dispatches on the container's
//! scheme.

mod keys;
mod mt19937;
mod pes15;
mod pes16_21;

pub use keys::MasterKey;

use pes_version::PesVersion;

/// Which encryption scheme a save uses and, for the keyed one, which master key
/// opened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// PES 15: keyless LCG stream, three MD5 digests.
    Pes15,
    /// PES 16-21: MT19937 stream under one of the seven master keys.
    Keyed(MasterKey),
}

/// A save's decrypted sections plus everything needed to write it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveContainer {
    /// Which scheme holds the bytes.
    pub scheme: Scheme,
    /// 384 bytes, `Edit Data` then zeros.
    pub description: Vec<u8>,
    /// A PNG.
    pub logo: Vec<u8>,
    /// The EDIT data the codec parses.
    pub payload: Vec<u8>,
    /// Header bytes 80.. on 16-21 (96 or 128 bytes), retained verbatim; empty on
    /// PES 15.
    pub identifier: Vec<u8>,
    /// UTF-16LE text on 16-21 (even length); empty on PES 15.
    pub serial: Vec<u8>,
}

/// Why a buffer is not a savefile container, or a container cannot be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContainerError {
    /// Neither a keyed header nor the PES 15 shape/digests matched.
    #[error("not a recognized savefile container")]
    Unrecognized,
    /// The file ends before the declared sizes.
    #[error("truncated: the declared sizes need {needed} bytes, the input has {available}")]
    Truncated {
        /// The byte count the declared sizes require.
        needed: usize,
        /// The input's byte count.
        available: usize,
    },
    /// The declared sizes end before the file does.
    #[error("{0} trailing bytes past the declared sections")]
    TrailingBytes(usize),
    /// `to_bytes` got a serial that is not a whole number of UTF-16 units.
    #[error("serial byte count {0} is odd; the serial is UTF-16LE text")]
    OddSerial(usize),
    /// `to_bytes` got an identifier that is not the header's tail length.
    #[error("identifier is {got} bytes, the header's tail wants {expected}")]
    BadIdentifier {
        /// `header_size - 80` for the container's scheme.
        expected: usize,
        /// The identifier's actual byte count.
        got: usize,
    },
    /// `to_bytes` got a PES 15 description that is not the section's fixed
    /// size; a wrong size would write the file misaligned.
    #[error("description is {got} bytes, the PES 15 section wants {expected}")]
    BadDescription {
        /// The section's fixed byte count.
        expected: usize,
        /// The description's actual byte count.
        got: usize,
    },
    /// A section grew past the header's u32 size field.
    #[error("section {section} is {size} bytes, past the header's u32 field")]
    SectionTooLarge {
        /// The section index in header order.
        section: u8,
        /// The size that did not fit.
        size: usize,
    },
}

impl SaveContainer {
    /// The game version the container belongs to.
    pub fn version(&self) -> PesVersion {
        match self.scheme {
            Scheme::Pes15 => PesVersion::Pes15,
            Scheme::Keyed(key) => key.version(),
        }
    }

    /// Tries the seven keys (header decrypt + 64-byte compare, cheap), then PES 15's
    /// shape (the description digest identifies it, then the declared lengths and the
    /// remaining digests). The first match wins.
    pub fn decrypt(bytes: &[u8]) -> Result<SaveContainer, ContainerError> {
        for key in MasterKey::ALL {
            match pes16_21::decrypt(bytes, key) {
                Err(ContainerError::Unrecognized) => {}
                result => return result,
            }
        }
        pes15::decrypt(bytes)
    }

    /// The container written back to bytes. PES 15 uses `salt[0]` as its seed byte;
    /// 16-21 use all 320 bytes.
    pub fn to_bytes(&self, salt: &[u8; 320]) -> Result<Vec<u8>, ContainerError> {
        match self.scheme {
            Scheme::Pes15 => pes15::encrypt(self, salt[0]),
            Scheme::Keyed(key) => pes16_21::encrypt(self, salt, key),
        }
    }
}

/// The salt under which `bytes` were encrypted — the one `to_bytes` needs to
/// write them again: a keyed save carries its 320-byte salt as its first 320
/// bytes, PES 15's seed is its byte 0 (the rest of the salt unused). The
/// container's own errors when `bytes` is no save.
pub fn file_salt(bytes: &[u8]) -> Result<[u8; 320], ContainerError> {
    match SaveContainer::decrypt(bytes)?.scheme {
        Scheme::Pes15 => {
            let mut salt = [0u8; 320];
            salt[0] = bytes[0];
            Ok(salt)
        }
        Scheme::Keyed(_) => Ok(bytes[..320]
            .try_into()
            .expect("a keyed save's decrypt passes only with a 320-byte salt")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::{Digest, Md5};
    use std::io::Read;

    const P15_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes15_head.bin");
    const P15_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes15_payload.bin.zz");
    const P15_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes15_payload_enc_head.bin");
    const P16_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes16_head.bin");
    const P16_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes16_payload.bin.zz");
    const P16_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes16_payload_enc_head.bin");
    const P17_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes17_head.bin");
    const P17_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes17_payload.bin.zz");
    const P17_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes17_payload_enc_head.bin");
    const P18_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes18_head.bin");
    const P18_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes18_payload.bin.zz");
    const P18_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes18_payload_enc_head.bin");
    const P19_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes19_head.bin");
    const P19_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes19_payload.bin.zz");
    const P19_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes19_payload_enc_head.bin");
    const P20_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes20_head.bin");
    const P20_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes20_payload.bin.zz");
    const P20_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes20_payload_enc_head.bin");
    const P21_HEAD: &[u8] = include_bytes!("../../tests/fixtures/pes21_head.bin");
    const P21_PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/pes21_payload.bin.zz");
    const P21_ENC: &[u8] = include_bytes!("../../tests/fixtures/pes21_payload_enc_head.bin");

    /// A keyed fixture: head bytes, encrypted payload prefix, compressed payload and
    /// the README's payload/logo/serial-unit sizes and game version string.
    struct KeyedFixture {
        key: MasterKey,
        head: &'static [u8],
        enc_head: &'static [u8],
        payload_zz: &'static [u8],
        payload_size: usize,
        logo_size: usize,
        serial_units: usize,
        /// The version string ending the identifier on 18-21; `None` on 16/17, where
        /// the identifier ends `EDIT` + 28 zeros.
        version_string: Option<&'static [u8]>,
    }

    const KEYED: [KeyedFixture; 6] = [
        KeyedFixture {
            key: MasterKey::Pes16,
            head: P16_HEAD,
            enc_head: P16_ENC,
            payload_zz: P16_PAYLOAD,
            payload_size: 5886176,
            logo_size: 67218,
            serial_units: 46,
            version_string: None,
        },
        KeyedFixture {
            key: MasterKey::Pes17,
            head: P17_HEAD,
            enc_head: P17_ENC,
            payload_zz: P17_PAYLOAD,
            payload_size: 5266180,
            logo_size: 28689,
            serial_units: 46,
            version_string: None,
        },
        KeyedFixture {
            key: MasterKey::Pes18,
            head: P18_HEAD,
            enc_head: P18_ENC,
            payload_zz: P18_PAYLOAD,
            payload_size: 5203344,
            logo_size: 66179,
            serial_units: 45,
            version_string: Some(b"PRO EVOLUTION SOCCER 2018"),
        },
        KeyedFixture {
            key: MasterKey::Pes19,
            head: P19_HEAD,
            enc_head: P19_ENC,
            payload_zz: P19_PAYLOAD,
            payload_size: 7350036,
            logo_size: 27504,
            serial_units: 46,
            version_string: Some(b"PRO EVOLUTION SOCCER 2019"),
        },
        KeyedFixture {
            key: MasterKey::Pes20,
            head: P20_HEAD,
            enc_head: P20_ENC,
            payload_zz: P20_PAYLOAD,
            payload_size: 10964500,
            logo_size: 13951,
            serial_units: 44,
            version_string: Some(b"eFootball PES 2020"),
        },
        KeyedFixture {
            key: MasterKey::Pes21,
            head: P21_HEAD,
            enc_head: P21_ENC,
            payload_zz: P21_PAYLOAD,
            payload_size: 10995800,
            logo_size: 14235,
            serial_units: 45,
            version_string: Some(b"eFootball PES 2021 SEASON UPDATE"),
        },
    ];

    fn inflate(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(bytes)
            .read_to_end(&mut out)
            .expect("zlib inflate");
        out
    }

    /// The decrypted file header of a head fixture under `key`.
    fn header(f: &KeyedFixture) -> Vec<u8> {
        let file_key = pes16_21::salt_file_key(f.key, &f.head[..320]);
        mt19937::crypt(
            &pes16_21::section_key(&file_key, f.key.header_size() as u64),
            &f.head[320..320 + f.key.header_size()],
        )
    }

    /// The decrypted description of a head fixture under `key`.
    fn description(f: &KeyedFixture) -> Vec<u8> {
        let file_key = pes16_21::salt_file_key(f.key, &f.head[..320]);
        let start = 320 + f.key.header_size();
        mt19937::crypt(
            &pes16_21::section_key(&file_key, 0),
            &f.head[start..start + 384],
        )
    }

    /// The decrypted 4096-byte payload prefix of a fixture under `key`.
    fn payload_head(f: &KeyedFixture) -> Vec<u8> {
        let file_key = pes16_21::salt_file_key(f.key, &f.head[..320]);
        mt19937::crypt(&pes16_21::section_key(&file_key, 2), f.enc_head)
    }

    /// The container a fixture's bytes describe: real description, identifier and
    /// inflated payload; zero logo and serial of the declared sizes.
    fn fixture_container(f: &KeyedFixture) -> SaveContainer {
        SaveContainer {
            scheme: Scheme::Keyed(f.key),
            description: description(f),
            logo: vec![0; f.logo_size],
            payload: inflate(f.payload_zz),
            identifier: header(f)[80..].to_vec(),
            serial: vec![0; f.serial_units * 2],
        }
    }

    /// The salt half of a head fixture.
    fn salt(f: &KeyedFixture) -> [u8; 320] {
        f.head[..320].try_into().expect("320 salt bytes")
    }

    /// A fixed non-zero salt for the synthetic round trips.
    fn test_salt() -> [u8; 320] {
        std::array::from_fn(|i| (i % 251) as u8 + 1)
    }

    /// The file's own salt reproduces it byte for byte on every version;
    /// another salt does not, so the check cannot pass on a wrong one.
    #[test]
    fn file_salt_reproduces_every_fixture() {
        for version in crate::test_support::FIXTURES {
            let (file, bytes) = crate::test_support::open(version);
            assert_ne!(
                file.to_bytes(&[7; 320]).expect("the save writes"),
                bytes,
                "{version:?}: not just any salt reproduces the file"
            );
            assert_eq!(
                file.to_bytes(&file_salt(&bytes).expect("the file's salt"))
                    .expect("the save writes"),
                bytes,
                "{version:?}: the file's own salt reproduces it"
            );
        }
    }

    #[test]
    fn each_fixture_opens_with_exactly_its_own_key() {
        for f in &KEYED {
            let header_size = f.key.header_size();
            assert_eq!(f.head.len(), 320 + header_size + 384);
            for key in MasterKey::ALL {
                let outcome = pes16_21::decrypt(f.head, key);
                if key == f.key {
                    // The compare passes; the declared sizes then run past the head.
                    assert!(
                        matches!(outcome, Err(ContainerError::Truncated { .. })),
                        "{key:?} on {:?}: {outcome:?}",
                        f.key
                    );
                } else {
                    assert_eq!(
                        outcome,
                        Err(ContainerError::Unrecognized),
                        "{key:?} on {:?}",
                        f.key
                    );
                }
            }
        }
    }

    #[test]
    fn fixture_headers_carry_the_readme_sizes_and_identifiers() {
        for f in &KEYED {
            let header = header(f);
            assert_eq!(header[..64], f.key.effective());
            let size = |offset: usize| {
                u32::from_le_bytes(header[offset..offset + 4].try_into().expect("u32")) as usize
            };
            assert_eq!(
                (size(64), size(68), size(72), size(76)),
                (f.payload_size, f.logo_size, 384, f.serial_units),
                "{:?}",
                f.key
            );
            let identifier = &header[80..];
            assert_eq!(identifier.len(), f.key.header_size() - 80);
            let mut type_string = [0u8; 32];
            type_string[..4].copy_from_slice(b"EDIT");
            match f.version_string {
                None => assert_eq!(&identifier[64..96], &type_string),
                Some(version) => {
                    assert_eq!(&identifier[64..96], &type_string);
                    let mut version_string = [0u8; 32];
                    version_string[..version.len()].copy_from_slice(version);
                    assert_eq!(&identifier[96..128], &version_string);
                }
            }
        }
    }

    #[test]
    fn fixture_descriptions_and_payload_heads_decrypt() {
        let mut expected_description = b"Edit Data".to_vec();
        expected_description.resize(384, 0);
        for f in &KEYED {
            assert_eq!(description(f), expected_description, "{:?}", f.key);
            assert_eq!(
                payload_head(f),
                inflate(f.payload_zz)[..f.enc_head.len()],
                "{:?}",
                f.key
            );
        }
    }

    #[test]
    fn to_bytes_reproduces_the_fixture_bytes() {
        for f in &KEYED {
            let out = fixture_container(f).to_bytes(&salt(f)).expect("to_bytes");
            assert_eq!(&out[..f.head.len()], f.head, "{:?}", f.key);
            let payload_at = f.head.len() + f.logo_size;
            assert_eq!(
                &out[payload_at..payload_at + f.enc_head.len()],
                f.enc_head,
                "{:?}",
                f.key
            );
        }
    }

    /// A small synthetic container for `key`.
    fn synthetic(key: MasterKey) -> SaveContainer {
        SaveContainer {
            scheme: Scheme::Keyed(key),
            description: (0..384).map(|i| (i % 13) as u8).collect(),
            logo: vec![9; 37],
            payload: (0..1029).map(|i| (i % 7) as u8).collect(),
            identifier: (0..key.header_size() - 80)
                .map(|i| (i % 11) as u8)
                .collect(),
            serial: vec![b'S'; 46 * 2],
        }
    }

    #[test]
    fn every_key_round_trips() {
        for key in MasterKey::ALL {
            let container = synthetic(key);
            let bytes = container.to_bytes(&test_salt()).expect("to_bytes");
            let back = SaveContainer::decrypt(&bytes).expect("decrypt");
            assert_eq!(back, container, "{key:?}");
        }
    }

    #[test]
    fn a_real_payload_container_round_trips() {
        for f in &KEYED {
            let container = fixture_container(f);
            let bytes = container.to_bytes(&salt(f)).expect("to_bytes");
            assert_eq!(
                SaveContainer::decrypt(&bytes).expect("decrypt"),
                container,
                "{:?}",
                f.key
            );
        }
    }

    #[test]
    fn pes15_fixture_decrypts() {
        let seed = P15_HEAD[0];
        assert_eq!(seed, 195);
        let description = pes15::lcg(seed, &P15_HEAD[49..49 + 384]);
        let mut expected = b"Edit Data".to_vec();
        expected.resize(384, 0);
        assert_eq!(description, expected);
        assert_eq!(&Md5::digest(&description)[..], &P15_HEAD[1..17]);
        let payload = inflate(P15_PAYLOAD);
        assert_eq!(
            &Md5::digest(&payload)[..],
            [
                0xc8, 0xd4, 0xd7, 0x8b, 0xf4, 0x33, 0x17, 0xb0, 0x2b, 0x66, 0x78, 0x6b, 0x92, 0xce,
                0x2d, 0x0c
            ]
        );
        assert_eq!(&Md5::digest(&payload)[..], &P15_HEAD[33..49]);
        let logo_len = u32::from_le_bytes(P15_HEAD[433..437].try_into().expect("u32")) as usize;
        assert_eq!(logo_len, 28795);
        assert_eq!(pes15::lcg(seed, P15_ENC), payload[..P15_ENC.len()]);
    }

    #[test]
    fn pes15_to_bytes_reproduces_the_fixture() {
        let mut salt = test_salt();
        salt[0] = 195;
        let description = pes15::lcg(195, &P15_HEAD[49..49 + 384]);
        let container = SaveContainer {
            scheme: Scheme::Pes15,
            description,
            logo: vec![0; 28795],
            payload: inflate(P15_PAYLOAD),
            identifier: Vec::new(),
            serial: Vec::new(),
        };
        let out = container.to_bytes(&salt).expect("to_bytes");
        // The logo digest at [17..33] legitimately differs (zero logo, not the PNG).
        assert_eq!(&out[..17], &P15_HEAD[..17]);
        assert_eq!(&out[33..437], &P15_HEAD[33..437]);
        let payload_at = 437 + 28795 + 4;
        assert_eq!(&out[payload_at..payload_at + P15_ENC.len()], P15_ENC);
    }

    #[test]
    fn pes15_round_trips() {
        let container = SaveContainer {
            scheme: Scheme::Pes15,
            description: (0..384).map(|i| (i % 9) as u8).collect(),
            logo: vec![5; 41],
            payload: (0..777).map(|i| (i % 5) as u8).collect(),
            identifier: Vec::new(),
            serial: Vec::new(),
        };
        let bytes = container.to_bytes(&test_salt()).expect("to_bytes");
        assert_eq!(SaveContainer::decrypt(&bytes), Ok(container));
    }

    #[test]
    fn errors() {
        assert_eq!(
            SaveContainer::decrypt(&[]),
            Err(ContainerError::Unrecognized)
        );
        assert_eq!(
            SaveContainer::decrypt(&[0; 100]),
            Err(ContainerError::Unrecognized)
        );
        // A salt byte flipped or a byte inside the header's 64-byte integrity
        // compare flipped: no key's compare holds. (A flip later in the header
        // corrupts that field alone — the stream XORs per byte — and stays a
        // valid container as far as `decrypt` can tell.)
        for at in [0usize, 320 + 10] {
            let mut corrupt = P16_HEAD.to_vec();
            corrupt[at] ^= 0xff;
            assert_eq!(
                SaveContainer::decrypt(&corrupt),
                Err(ContainerError::Unrecognized)
            );
        }
        let container = synthetic(MasterKey::Pes19);
        let bytes = container.to_bytes(&test_salt()).expect("to_bytes");
        let short = &bytes[..bytes.len() - 20];
        assert_eq!(
            SaveContainer::decrypt(short),
            Err(ContainerError::Truncated {
                needed: bytes.len(),
                available: short.len()
            })
        );
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            SaveContainer::decrypt(&long),
            Err(ContainerError::TrailingBytes(1))
        );
    }

    /// The crate's own PES 15 writer, cut at each boundary: the first missing
    /// span is reported exactly — 433 bytes is a whole valid head, so it is
    /// `Truncated` at the logo-length field, not `Unrecognized`.
    #[test]
    fn pes15_boundaries_report_the_first_missing_span() {
        let container = SaveContainer {
            scheme: Scheme::Pes15,
            description: (0..384).map(|i| (i % 9) as u8).collect(),
            logo: vec![7; 41],
            payload: (0..777).map(|i| (i % 5) as u8).collect(),
            identifier: Vec::new(),
            serial: Vec::new(),
        };
        let bytes = container.to_bytes(&test_salt()).expect("to_bytes");
        assert_eq!(
            SaveContainer::decrypt(&bytes[..433]),
            Err(ContainerError::Truncated {
                needed: 437,
                available: 433
            })
        );
        // Inside the 4-byte logo-length field, then exactly at its end:
        // what is missing next is the 41-byte logo plus the payload-length
        // field.
        assert_eq!(
            SaveContainer::decrypt(&bytes[..435]),
            Err(ContainerError::Truncated {
                needed: 437,
                available: 435
            })
        );
        assert_eq!(
            SaveContainer::decrypt(&bytes[..437]),
            Err(ContainerError::Truncated {
                needed: 437 + 41 + 4,
                available: 437
            })
        );
        // The whole logo plus its length field (482 bytes) reads fine; the
        // payload declared in the header is what is missing.
        assert_eq!(
            SaveContainer::decrypt(&bytes[..482]),
            Err(ContainerError::Truncated {
                needed: 482 + 777,
                available: 482
            })
        );
        // Bytes after the declared sections are TrailingBytes, counted
        // exactly.
        let mut long = bytes.clone();
        long.extend_from_slice(&[9u8; 7]);
        assert_eq!(
            SaveContainer::decrypt(&long),
            Err(ContainerError::TrailingBytes(7))
        );
        // A corruption only inside the logo or only inside the payload
        // still trips that section's digest.
        let mut bad_logo = bytes.clone();
        bad_logo[437] ^= 0xff;
        assert_eq!(
            SaveContainer::decrypt(&bad_logo),
            Err(ContainerError::Unrecognized)
        );
        let mut bad_payload = bytes.clone();
        let last = bad_payload.len() - 1;
        bad_payload[last] ^= 0xff;
        assert_eq!(
            SaveContainer::decrypt(&bad_payload),
            Err(ContainerError::Unrecognized)
        );
    }

    /// A keyed file cut inside the header reports `salt + header_size` as
    /// `needed` — the salt is not folded into the header length.
    #[test]
    fn a_short_keyed_header_reports_salt_plus_header() {
        let bytes = synthetic(MasterKey::Pes17)
            .to_bytes(&test_salt())
            .expect("to_bytes");
        // PES 17's header is 176 bytes; salt + 175 decrypts a partial header
        // and reports 320 + 176.
        assert_eq!(
            SaveContainer::decrypt(&bytes[..495]),
            Err(ContainerError::Truncated {
                needed: 496,
                available: 495
            })
        );
        // 384 bytes is the salt plus the whole 64-byte compare span, but the
        // header is still incomplete: `needed` is salt + header (496). At
        // 496 the whole header is present and what is missing is past it.
        assert_eq!(
            SaveContainer::decrypt(&bytes[..384]),
            Err(ContainerError::Truncated {
                needed: 496,
                available: 384
            })
        );
        let err = SaveContainer::decrypt(&bytes[..496]).expect_err("past the header");
        assert!(
            matches!(err, ContainerError::Truncated { needed, available } if needed > 496 && available == 496),
            "{err:?}"
        );
        // Trailing bytes are counted as `len - needed`, not a quotient.
        let mut long = bytes.clone();
        long.extend_from_slice(&[0; 7]);
        assert_eq!(
            SaveContainer::decrypt(&long),
            Err(ContainerError::TrailingBytes(7))
        );
    }

    #[test]
    fn to_bytes_rejects_a_malformed_container() {
        let mut container = synthetic(MasterKey::Pes17);
        container.serial = vec![0; 3];
        assert_eq!(
            container.to_bytes(&test_salt()),
            Err(ContainerError::OddSerial(3))
        );
        let mut container = synthetic(MasterKey::Pes17);
        container.identifier = vec![0; 10];
        assert_eq!(
            container.to_bytes(&test_salt()),
            Err(ContainerError::BadIdentifier {
                expected: 96,
                got: 10
            })
        );
        // PES 15's description is a fixed-size encrypted section: anything
        // else would be written misaligned, so it is refused.
        let container = SaveContainer {
            scheme: Scheme::Pes15,
            description: vec![0; 9],
            logo: vec![7; 41],
            payload: (0..777).map(|i| (i % 5) as u8).collect(),
            identifier: Vec::new(),
            serial: Vec::new(),
        };
        assert_eq!(
            container.to_bytes(&test_salt()),
            Err(ContainerError::BadDescription {
                expected: 384,
                got: 9
            })
        );
    }
}
