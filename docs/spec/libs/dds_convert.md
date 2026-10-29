# Unlimited Tool Works — Library crates plan: dds_convert

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## In-process DDS conversion (replaces texconv)

The PES Stadium Compiler (`Tools_4cc/pes-stadium-compiler-v1.0.0`) already proves the in-process
approach in production, in `Engines/stages/lib/dxt.py` + the `texture2ddecoder` package:

- **Decoding (DDS/FTEX)**: `block_compression`'s `decode` module (BC1–BC5, BC7), the same crate
  that encodes, so one dependency covers both directions; the `texture2ddecoder` crate the plan
  first named is not used. `dds_convert`/`ftex` handle the DDS/FTEX containers and normalize
  decoded channel order to RGBA. Parity is verified against DirectXTex `texconv`, the reference
  decoder: the `dds_convert` fixtures are texconv-encoded BC1/BC2/BC3/BC4/BC5/BC7, R8 and legacy
  luminance (L8) files with texconv's own decode of every mip as the expected output, and the
  tests require an exact match. The reference build is texconv 2024.1.1.1, the binary the legacy
  compilers shipped; a one-channel source (BC4, R8, L8) decodes grey (`R = G = B`, alpha 255) as
  it does, since its R-to-RGB conversion splats the channel (BC5 keeps `B = 0`).
- **Decoding (raster sources)**: the `image` crate decodes all accepted raster formats to RGBA
  in pure Rust (no C dependencies). See the format matrix below.
- **Encoding**: use `block_compression`'s Rust CPU backend for BC1, BC3, and BC7, rather than
  translating the stadium compiler's numpy encoder. Codec selection follows the PES-version and
  texture-role rules below; the GPU BC7 backend is in `plans/libs/README.md` "First-release
  desktop GPU BC7".
- **DXT5nm handling**: a normal map that has to be encoded (a BC5 source on PES 15–18, or any
  normal-role source that is not already BC3, raster and BC7 included, on every version) becomes
  BC3 with X in alpha and Y in green, which is what both
  engines ship: every Konami normal map measured is BC3 with `A = X`, `G = Y` (PES 17 `oral_nrm`,
  `bibs_nrm`, `skin_nrm`; PES 21 `dummy_nrm`). The two engines differ only in the channels the
  shader ignores, and the output copies each engine's own files rather than guessing what the
  shader reads: pre-Fox `R = G = B = Y` (the color block is a grey Y), Fox `R = 255`, `B = 0` (the
  DirectXTex `DXT5nm` layout). The legacy compilers never produced this conversion at all: released
  Red converts every DX10 texture with texconv's `-f DXT5 -srgb`, which drops X when the source is
  BC5 (its unreleased development branch asks for a format name texconv rejects, `DX5nm`), so
  there is no legacy output to be parity with; the Konami files are the standard.
- **Passthrough**: compressed blocks a target can read are kept and only the container changes.
  PES 15–18 read BC1, BC2 and BC3; PES 19–21 also read BC4, BC5 and BC7 (the legacy compilers
  passed BC5 through to PES 19–21 as FTEX format 9). The role narrows this: a normal-role source
  passes through only as BC3 (taken to be laid out already) or, on PES 19–21, as BC5; a
  normal-role BC7 or BC1 source is decoded and encoded to DXT5nm, because keeping its blocks would
  keep a color layout the shader reads as a normal map. A DX10-header BC1/BC3 is therefore rewritten
  with a legacy header for PES 15–18 rather than re-encoded (the legacy compilers re-encoded every
  DX10 header to DXT5, a lossy step with no purpose). Uncompressed sources are always encoded:
  PES 15–17 crash on uncompressed kit textures.
- **Mipmaps**: each mip level is decoded and re-encoded individually; a DDS/FTEX source keeps its
  own mip count whatever its codec, uncompressed included (Konami ships single-mip pre-Fox
  textures; on Fox the Team compiler's deep pass refuses a mipped texture with neither side a
  power of two, as `texture_not_pow2`), a raster source gets the
  full chain down to 1×1 by 2×2 box averaging on
  straight alpha (odd sides floor, never below 1). Output headers are rebuilt from the emitted
  layout: legacy DX9 headers for BC1/BC3 (what texconv wrote for the legacy compilers and what
  Konami ships), a DX10 header for BC7.
- **FTEX texture type**: every Fox output is written with texture type `0x9`, the value the legacy
  compilers wrote for every texture, color and normal alike, for years of PES 19–21 exports; Konami
  uses `0x1`/`0x3` for color textures, but the parity standard is what is known to render.

This covers the actual conversion cases (DX10 BC7 → DXT5, BC5/ATI2 → DXT5nm) that `texconv -f
DXT5` was handling, or failing to handle, in practice. Benefits over the subprocess approach:

- No Windows-only binary; identical behavior on Linux (removes the ImageMagick fallback path
  entirely)
- No temp-folder write/read roundtrip; operates on in-memory buffers, matching the VirtualTree
  pipeline
- Much faster for batches (no process spawn per texture)

Any texture in a format outside the accepted set fails with a clear error asking the user to
resave it — the same behavior the current compilers have for unsupported codecs.

### Target codecs and CPU encoder

**BC7 is supported only by PES 19–21.** PES18 uses Fox models/FTEX but does not support BC7;
model-engine generation alone is therefore not the texture-codec selector.

| Target PES | BC7 input | Newly encoded ordinary raster/color input |
|---|---|---|
| 15–18 | Transcode to BC3, or BC1 when eligible below | BC3, or BC1 when eligible below |
| 19–21 | Retain compatible BC7 blocks | BC7 |

Keep already-compatible compressed blocks when only DDS/FTEX container/header adaptation is needed.
Normal/data textures retain their role-specific channel and codec requirements.

**BC1 optimization:** use BC1 for fully opaque ordinary color textures (alpha 255 in every emitted mip)
that do not require alpha as a data channel. It uses 8 bytes per block instead of BC3's 16 and uses
compatible four-color RGB coding, but neither encoding is lossless relative to raster/BC7 sources.
“No semitransparent pixels” alone is insufficient: BC1's one-bit transparency reduces affected
blocks to three visible colors, and generated mips may introduce intermediate alpha. Keep such
textures in BC3 for the older targets rather than promising unchanged quality. DXT5nm keeps BC3:
its alpha stores a normal component, not opacity.

**Selected encoder:** `block_compression` (reviewed release **0.10.0**) supplies the CPU
encoder (the reference/fallback when the GPU step lands, `plans/libs/README.md` "First-release
desktop GPU BC7"). It is built with `default-features = false` and `bc15`/`bc7`; the `wgpu`
feature joins with the GPU step and BC6H is not needed. Use basic opaque/alpha BC7 presets initially, preserving
alpha when required; tune against representative textures, not assumed timings. `texpresso` lacks
BC7, while `intel_tex_2` adds native ISPC kernels; one library serves both paths without maintaining
our own encoders. This is a best-fit selection, not a measured claim of optimal throughput.

Before integration, check decode/quality parity, repeatability, and native/WASM builds. Exercise
PES18 versus PES19 output, opaque versus alpha-bearing textures, DXT5nm, and the complete mip chain;
the CPU API requires 4-aligned input dimensions, so edge padding must also handle 1×1/2×2 tail mips
without changing their logical DDS/FTEX dimensions.

### Accepted image formats

All image formats are interchangeable as texture sources for any model format (FMDL, .model,
glTF). Material references name textures by stem (filename without extension); the compiler
resolves each stem to whichever file exists in the model folder and converts to the target
format. Two image files with the same stem but different extensions is a conflict
(`texture_stem_conflict`).

**Accepted:**

| Format | Decoder | Extensions | Notes |
|--------|---------|------------|-------|
| DDS | `block_compression::decode` | `.dds` | BC7, BC5, BC4, BC3/DXT5, BC2, BC1/DXT1, uncompressed with byte-aligned 8-bit channel masks (R8/L8 included) |
| FTEX | `ftex` crate | `.ftex` | Fox Engine native texture |
| PNG | `image` crate | `.png` | Pure Rust |
| JPEG | `image` crate | `.jpg`, `.jpeg` | Pure Rust |
| BMP | `image` crate | `.bmp` | Pure Rust |
| WebP | `image` crate | `.webp` | Pure Rust |
| TGA | `image` crate | `.tga` | Pure Rust, under `image`'s default allocation limit. A TGA 2.0 extension declaring premultiplied alpha (attributes type 4) is un-multiplied like TIFF's; alpha that is 0 in every pixel is read as opaque, as texconv does by default. 16-bit TGA (5-bit primaries) is refused with a resave message, since `image` drops its alpha bit and DirectXTex keeps it; interleaved TGA is refused, as DirectXTex does |
| TIFF | `image` crate | `.tif`, `.tiff` | Pure Rust; Photoshop users. Associated (premultiplied) alpha, `ExtraSamples = 1`, is un-multiplied after the decode, at the source's precision (f32 for float samples, 16 bits otherwise; the tag is read with `tiff`, the crate `image` decodes through) |

**Not accepted** (enable only the accepted decoders explicitly; the `image` crate's default
features and AVIF backends vary by release):

| Format | Reason |
|--------|--------|
| AVIF | Outside the accepted source set; decoder/backend portability is not part of the initial scope |
| GIF | Animation format; not suitable for static textures |
| ICO | Windows icon container; not a texture format |
| HDR (Radiance RGBE) | HDR photography; not used in game modding |
| OpenEXR | Film/VFX HDR; not used in game modding |
| Farbfeld | Obscure pure-Rust format; no community use |
| PNM (PPM/PGM/PBM) | Simple academic format; no community use |
| QOI | Modern pure-Rust format; no community adoption yet — can be added if demand arises |

### `dds_convert` API

The crate is three pure steps and one stateful wrapper. `decode` turns any accepted source into
straight-alpha RGBA8 mips plus, for DDS/FTEX, the compressed blocks it carried; `convert` applies
the codec rules above to a decoded texture and returns the finished container bytes (a DDS for
PES 15–17, an FTEX for PES 18–21); `Converter` is the session cache in front of both, which
skips the decode for a DDS or FTEX source whose blocks the target keeps, rewrapping them into
the bytes `decode` + `convert` would make.
`encode_dds` is the one exception to choosing by version: a file every engine reads as a DDS
in a codec the plan fixes (a player portrait, BC3: "Portraits" in the [player folders
plan](../aesthetics_export/player_folders.md)) names its codec, and gets a DDS on Fox targets too.
`resize` is the suite's one resampler (Lanczos3, through the `image` crate this crate already
decodes rasters with, so no tool crate depends on `image` for it): the Team compiler resamples a
sock band of a kit and a number atlas's digit glyphs with it ("Layout conversion" in the
[pipeline plan](../team_compiler/pipeline.md)), and sizes the logo with it.
`encode_png` is the one raster this crate writes: the game reads a team's logo as three PNGs
("Logo" in the pipeline plan), and it lives here for `resize`'s reason.

```rust
/// The accepted source formats (table above), named by the file extension the
/// compiler resolved the texture stem to. TGA has no magic, so the format is
/// never sniffed from bytes.
pub enum SourceFormat { Dds, Ftex, Png, Jpeg, Bmp, WebP, Tga, Tiff }
impl SourceFormat {
    /// Case-insensitive, without the dot; `jpg`/`jpeg` and `tif`/`tiff` both accepted.
    pub fn from_extension(extension: &str) -> Option<Self>;
}

/// What the texture is for; with the PES version it selects the codec and the channel layout.
pub enum TextureRole { Color, Normal }

/// Block codecs this crate keeps or emits.
pub enum BlockCodec { Bc1, Bc2, Bc3, Bc4, Bc5, Bc7 }

/// Compressed blocks a DDS/FTEX source carried, one buffer per mip, kept so a
/// target that reads the codec gets them unchanged.
pub struct Blocks { pub codec: BlockCodec, pub mips: Vec<Vec<u8>> }

/// A source decoded to straight-alpha RGBA8, top mip first, every mip the source carried.
pub struct Decoded { pub width: u32, pub height: u32, pub mips: Vec<Vec<u8>>, pub blocks: Option<Blocks>, pub authored_mips: bool }

pub struct Target { pub version: PesVersion, pub role: TextureRole }

pub fn decode(bytes: &[u8], format: SourceFormat) -> Result<Decoded, ConvertError>;
pub fn convert(decoded: &Decoded, target: Target) -> Result<Vec<u8>, ConvertError>;
/// A DDS in `codec` (`Bc1`, `Bc3` or `Bc7`; any other is `Unsupported`) at the source's own
/// size, whatever the target engine, color layout: the source's mip count kept, a raster
/// source's chain generated, a block source already in `codec` keeping its blocks.
pub fn encode_dds(decoded: &Decoded, codec: BlockCodec) -> Result<Vec<u8>, ConvertError>;
/// `pixels` (straight-alpha RGBA8, `width` x `height`, row-major) resampled to `new_width` x
/// `new_height` with Lanczos3, each channel as it is. Near an edge the filter's taps that fall
/// past it are left out and the rest rescaled, so a flat image stays flat. A zero size or a
/// `pixels` of the wrong length is `InvalidDecoded`.
pub fn resize(pixels: &[u8], width: u32, height: u32, new_width: u32, new_height: u32) -> Result<Vec<u8>, ConvertError>;
/// `pixels` (straight-alpha RGBA8, `width` x `height`, row-major) as a PNG file, 8-bit RGBA.
/// A zero size or a `pixels` of the wrong length is `InvalidDecoded`.
pub fn encode_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError>;

/// What a source's header says of its texture, read without decoding a pixel.
pub struct Probe { pub width: u32, pub height: u32, pub mipmaps: u32 }
pub fn probe(bytes: &[u8], format: SourceFormat) -> Result<Probe, ConvertError>;

/// SHA-256 of the source bytes, computed once when the file is materialized and reused.
pub struct SourceHash(pub [u8; 32]);
pub fn source_hash(bytes: &[u8]) -> SourceHash;

pub enum CachePolicy { Use, Bypass }

/// The session cache: finished container bytes by (source hash, source format, target).
pub struct Converter { /* Mutex<HashMap<CacheKey, Arc<[u8]>>> */ }
impl Converter {
    pub fn new() -> Self;
    pub fn convert(&self, hash: SourceHash, bytes: &[u8], format: SourceFormat, target: Target, cache: CachePolicy) -> Result<Arc<[u8]>, ConvertError>;
    /// Bytes currently retained, for the pipeline's memory budget.
    pub fn retained_bytes(&self) -> usize;
    pub fn clear(&self);
}
```

`decode` rejects cube maps, volume textures (the caps2 bit, or a legacy header's `DDSD_DEPTH`
with a depth above 1, as DirectXTex classifies them), texture arrays and the signed block formats (BC4/BC5
SNORM, BC6H SF16), every one of them as `ConvertError::Ftex(UnsupportedDds)` from
`ftex::dds::read_layout`, which `probe` and `decode` both run first (the compiler reports each
as `texture_codec_unsupported`, the file dropped; `ConvertError::Unsupported` is for what the
layout reads but the decoder or a target cannot take: an unknown pixel format or mask layout, a
block texture past the decoder's 4 GiB limit). The compiler sends no cube map here: it routes one by `ftex::dds::is_cube_map`
before any decode (`team_compiler/pipeline.md` step 5), since referee exports carry copies of
the template `env.dds`. A signed block relabelled unsigned would silently change the texture. It also rejects a zero width or height and
a mip count past what the dimensions halve into (`log2(larger side) + 1`), as D3D and texconv's
default loader do; `ftex`'s own FTEX↔DDS conversions keep pes-file-tools' acceptance of both. The
mip count is the header's count field alone, as DirectXTex reads it; `dds_to_ftex` keeps
pes-file-tools' rule that a cleared `DDSCAPS_MIPMAP` means one level. The alpha mask of an
uncompressed header counts only under `DDPF_ALPHAPIXELS` or `DDPF_ALPHA` (DirectXTex reads an RGB
header as opaque). A block texture whose padded top mip decodes past 4 GiB is refused: the block
decoder offsets its output in 32 bits, and any buffer a texture's dimensions size past the
target's `isize::MAX` (2 GiB on wasm32) is refused rather than a capacity panic. The named
uncompressed layouts (32-bit ARGB8, 8-bit luminance) are matched on the bit count as well as the
masks, as DirectXTex matches them. A DX10
header declaring premultiplied alpha, a paletted header (`DDPF_PALETTEINDEXED8`), a legacy bump-map
header (`DDPF_BUMPDUDV`/`DDPF_BUMPLUMINANCE`, signed) and an uncompressed header with no channel
mask at all are refused too, since none has a straight-alpha unsigned decode; BC4 is read under
both its FourCCs (`ATI1`, `BC4U`); NVTT v1's L8 header (the RGB flag, 8 bits, an R mask only) reads as R8, as DirectXTex
reads it. `convert` checks a
`Decoded` it did not produce against the same rules (`ConvertError::InvalidDecoded`: nonzero
dimensions, a top level of at most 4 GiB of RGBA since mip generation and the codecs index in
32 bits, one mip per level with `width * height * 4` bytes, block buffers sized for their
codec). An uncompressed DDS whose header declares a row pitch wider
than its rows (some exporters pad rows to 4 bytes) is read at that pitch, each lower mip at the
same 4-byte row alignment. `Decoded.authored_mips` says whether the source format carries a mip
chain (DDS/FTEX, its level count is kept) or not (raster, a chain is generated). A WESYS-wrapped DDS (PES 15–17 sources) is unwrapped first through `wezlib`. The DDS
header knowledge (`DdsHeader`, `Dx10Header`, the FourCC/mask/DXGI table) lives in `ftex`, which
already needed it in both directions; `ftex` exposes it as `ftex::dds` (`read_layout`,
`header_bytes`, and `DdsPixel::row_bytes`, the one tight-row rule) and `dds_convert` uses that
instead of carrying a second copy. Encoder settings
join the cache key with 2.5b, when a second encoder exists.

`probe` reads a texture's size from its header alone. It exists for the Team compiler's deep
pass, which checks every texture of an export and must not pay a decode for each: a DDS's layout
through `ftex::dds::read_layout` (a WESYS-wrapped one unwrapped first), an FTEX's through
`ftex::info`, a raster source's through the `image` decoder's dimensions. `Probe.mipmaps` is the
level count the converted texture carries: a DDS or FTEX source's own count, and for a raster
source the full chain down to 1x1, which conversion generates. `probe` fails on a header `decode`
would fail to read; it says nothing of the pixel data behind the header, so a texture that
probes can still fail to decode.

### In-memory conversion cache

CPU texture conversion is reusable for a given source hash and complete conversion configuration
(the key below), so a run caches converted output bytes in memory for its length: a source
converted once is not converted again in that run. This targets the edit→compile→test loop — the hot path, where
the same export is compiled dozens of times with minor edits to one or two files; carrying the
cache across runs is `plans/libs/dds_convert.md`. While an entry is retained,
unchanged textures reuse its converted bytes; eviction may require conversion again.

**Design:**
- A `Converter` struct in `dds_convert` holds an internal `HashMap<CacheKey, Arc<[u8]>>` mapping
  to the finished DDS/FTEX bytes (not the decoded RGBA — the output is what the packing step
  needs).
- The cache key includes the source content hash and every output-affecting conversion choice:
  target codec/container, texture role and channel swizzle, and mip/color-space settings —
  source hash, source format and target. The source hash is computed at materialization and reused; DXT5 color data and
  DXT5nm normal data must not alias merely because they share a codec.
- Cache misses fall through to the normal decode → mipmap → encode path, or, for a source
  whose blocks the target keeps, to the passthrough, which rewraps them without a decode. The
  cache is consulted after the build manifest has determined the target codec for each texture, so the
  same source PNG cached as DXT5 for a pre-Fox compile is not wrongly reused for a Fox BC7
  compile (different `target_codec` → different key → separate entry).
- **Engagement limit:** the cache engages only when compiling **≤2 teams**. The build manifest
  knows the team count at planning time; for larger compiles (3+ teams, e.g. a 48-team DLC),
  the cache is bypassed entirely — entries are never inserted. Rationale: a large compile has
  mostly first-seen textures (no cache benefit), and caching thousands of converted DDS/FTEX
  buffers would consume a sizeable amount of memory for no gain, hurting parallelism by
  reducing the budget available to worker tasks. The ≤2 team threshold covers the typical
  edit→test scenario (one or two teams) while keeping large compiles from adding cache entries.
- A CLI run is one process: it makes its `Converter` per run and drops it with the run, so what
  the cache holds is at most two exports' conversions, which no session accumulates.
  `Converter` retains every distinct conversion until `clear`, and exposes `retained_bytes`
  and `clear`; the retention policy that calls them is in `plans/libs/dds_convert.md`.

**What it covers and doesn't:** repeated one/two-team compiles benefit when their working set
fits the cache. A fresh full-cup compile bypasses it and must pay for every required conversion.
Benchmark both workloads with the selected codecs, representative textures, and intended worker
count; a cache does not establish a fixed compile-time guarantee.

### PNG-texture trajectory

Blender cannot write DDS natively, and the ported addons convert DDS→PNG on import. As the
community migrates to glTF authoring (the plan's end-state path), PNG becomes the norm for
source textures, not the corner case. The conversion pipeline is designed for this: the `image`
crate handles decoding, mipmap generation fills the chain that raster sources don't carry, and
the in-memory cache (above) keeps the edit→compile→test loop fast. The worst-case first-compile
cost is accepted as the price of dropping the texconv.exe dependency and enabling glTF authoring
— strictly better than the current workflow (manual texconv runs or no PNG support at all).

---
