//! The session conversion cache: finished container bytes keyed by source
//! hash, source format and target.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{CachePolicy, ConvertError, SourceFormat, SourceHash, Target, dds};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    hash: SourceHash,
    format: SourceFormat,
    target: Target,
}

/// The session cache in front of `decode` + `convert`: finished container
/// bytes by (source hash, source format, target). `Bypass` never inserts
/// and never reads.
pub struct Converter {
    entries: Mutex<HashMap<CacheKey, Arc<[u8]>>>,
}

impl Converter {
    /// An empty cache.
    pub fn new() -> Self {
        Converter {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Decodes `bytes` as `format`, converts for `target`, and caches the
    /// result under `hash` when `cache` is `Use`. A DDS or FTEX source whose
    /// blocks `target` keeps is rewrapped without the decode: the output is
    /// the bytes `decode` + `convert` would make. `hash` is the caller's
    /// [`crate::source_hash`] of `bytes`, computed once at materialization.
    pub fn convert(
        &self,
        hash: SourceHash,
        bytes: &[u8],
        format: SourceFormat,
        target: Target,
        cache: CachePolicy,
    ) -> Result<Arc<[u8]>, ConvertError> {
        if cache == CachePolicy::Bypass {
            return Ok(decode_and_convert(bytes, format, target)?.into());
        }
        let key = CacheKey {
            hash,
            format,
            target,
        };
        if let Some(hit) = self.entries.lock().unwrap().get(&key).cloned() {
            log::debug!("conversion cache hit");
            return Ok(hit);
        }
        let output: Arc<[u8]> = decode_and_convert(bytes, format, target)?.into();
        log::debug!("conversion cache miss, storing {} bytes", output.len());
        self.entries.lock().unwrap().insert(key, output.clone());
        Ok(output)
    }

    /// Bytes currently retained, for the pipeline's memory budget.
    pub fn retained_bytes(&self) -> usize {
        self.entries
            .lock()
            .unwrap()
            .values()
            .map(|entry| entry.len())
            .sum()
    }

    /// Drops every retained entry.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

/// `decode` + `convert`, skipping the decode when the source's blocks pass
/// through: a whole mip chain decoded to RGBA (about 22 MB for a 2048x2048
/// texture) would be thrown away unread.
fn decode_and_convert(
    bytes: &[u8],
    format: SourceFormat,
    target: Target,
) -> Result<Vec<u8>, ConvertError> {
    let dds_bytes = match format {
        SourceFormat::Dds => wezlib::decompress_if_wrapped(bytes)?,
        SourceFormat::Ftex => Cow::Owned(ftex::ftex_to_dds(bytes)?),
        SourceFormat::Png
        | SourceFormat::Jpeg
        | SourceFormat::Bmp
        | SourceFormat::WebP
        | SourceFormat::Tga
        | SourceFormat::Tiff
        | SourceFormat::Jxl => return crate::convert(&crate::decode(bytes, format)?, target),
    };
    if let Some(output) = dds::pass_through(&dds_bytes, target)? {
        return Ok(output);
    }
    crate::convert(&dds::decode_dds(&dds_bytes)?, target)
}
