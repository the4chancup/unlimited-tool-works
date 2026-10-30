//! The fixture payloads: each version's committed `payload.bin.zz` inflated
//! and wrapped in its `SaveContainer`, and the PES 17 Texport's the same
//! way. Shared by `test_support` (the crate's own tests) and, under
//! `test-support`, by `fixture_save` and `pes17_texport` for other crates'
//! tests.

use pes_version::PesVersion;

use crate::container::{MasterKey, SaveContainer, Scheme};

/// The inflated payload of one version's fixture.
pub(crate) fn payload(version: PesVersion) -> Vec<u8> {
    let bytes: &[u8] = match version {
        PesVersion::Pes15 => include_bytes!("../tests/fixtures/pes15_payload.bin.zz"),
        PesVersion::Pes16 => include_bytes!("../tests/fixtures/pes16_payload.bin.zz"),
        PesVersion::Pes17 => include_bytes!("../tests/fixtures/pes17_payload.bin.zz"),
        PesVersion::Pes18 => include_bytes!("../tests/fixtures/pes18_payload.bin.zz"),
        PesVersion::Pes19 => include_bytes!("../tests/fixtures/pes19_payload.bin.zz"),
        PesVersion::Pes20 => include_bytes!("../tests/fixtures/pes20_payload.bin.zz"),
        PesVersion::Pes21 => include_bytes!("../tests/fixtures/pes21_payload.bin.zz"),
    };
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(bytes), &mut out)
        .expect("fixture payload inflates");
    out
}

/// The master key matching a fixture version.
fn key(version: PesVersion) -> MasterKey {
    match version {
        PesVersion::Pes15 => panic!("PES 15 is not keyed"),
        PesVersion::Pes16 => MasterKey::Pes16,
        PesVersion::Pes17 => MasterKey::Pes17,
        PesVersion::Pes18 => MasterKey::Pes18,
        PesVersion::Pes19 => MasterKey::Pes19,
        PesVersion::Pes20 => MasterKey::Pes20,
        PesVersion::Pes21 => MasterKey::Pes21,
    }
}

/// A `SaveContainer` around a fixture's inflated payload.
pub(crate) fn fixture_container(version: PesVersion) -> SaveContainer {
    let mut description = b"Edit Data".to_vec();
    description.resize(384, 0);
    match version {
        PesVersion::Pes15 => SaveContainer {
            scheme: Scheme::Pes15,
            description,
            logo: Vec::new(),
            payload: payload(version),
            identifier: Vec::new(),
            serial: Vec::new(),
        },
        _ => {
            let key = key(version);
            SaveContainer {
                scheme: Scheme::Keyed(key),
                description,
                logo: Vec::new(),
                payload: payload(version),
                identifier: vec![0; key.header_size() - 80],
                serial: Vec::new(),
            }
        }
    }
}

/// The salt the tests write with.
pub(crate) fn test_salt() -> [u8; 320] {
    std::array::from_fn(|i| (i % 251) as u8 + 1)
}

/// The bytes of `version`'s fixture save: a valid encrypted `EDIT00000000`,
/// what `test_support::open` builds for the crate's own tests. Test support,
/// not a save API.
#[cfg(feature = "test-support")]
pub fn fixture_save(version: PesVersion) -> Vec<u8> {
    fixture_container(version)
        .to_bytes(&test_salt())
        .expect("the fixture container encodes")
}

/// The PES 17 Texport fixture's decrypted payload, sliced as a save
/// fixture's is.
pub(crate) fn pes17_texport_payload() -> Vec<u8> {
    let mut out = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::ZlibDecoder::new(
            &include_bytes!("../tests/fixtures/pes17_texport_payload.bin.zz")[..],
        ),
        &mut out,
    )
    .expect("the Texport payload inflates");
    out
}

/// `payload` as the PES 17 Texport's container (logo and serial dropped,
/// as the save fixtures are).
pub(crate) fn pes17_texport_container(payload: Vec<u8>) -> Vec<u8> {
    let mut description = b"Team Export Data 03".to_vec();
    description.resize(384, 0);
    SaveContainer {
        scheme: Scheme::Keyed(MasterKey::Pes17),
        description,
        logo: Vec::new(),
        payload,
        identifier: vec![0; MasterKey::Pes17.header_size() - 80],
        serial: Vec::new(),
    }
    .to_bytes(&test_salt())
    .expect("the Texport container encodes")
}

/// The bytes of the PES 17 Texport fixture, a `TEXPORT00000000` holding
/// team 736: its sliced payload in its container. Test support, not a
/// Texport API.
pub fn pes17_texport() -> Vec<u8> {
    pes17_texport_container(pes17_texport_payload())
}
