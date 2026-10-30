# Unlimited Tool Works — Library crates plan: dds_convert

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## In-process DDS conversion (replaces texconv)

### In-memory conversion cache

What exists of this is in `spec/libs/dds_convert.md` "In-memory conversion cache".

- The Studio creates one `Converter` at startup and passes it into each Team compiler run. The
  cache lives for the application session and is dropped on exit — **no disk persistence**, so
  it never fills the user's drive with temporary files.
- **Retention is separately bounded and budgeted.** Repeated edits or switching between teams must
  not accumulate every historical conversion for the whole session. Retained bytes count against
  the shared memory budget even after a task finishes; evict reusable cache entries under pressure
  so they cannot prevent working tasks from acquiring memory. Bypassing the cache for a large run
  does not by itself release entries retained by earlier small runs.
  The policy belongs to the Team compiler pipeline's memory budget and is built with the
  first caller that keeps a `Converter` across runs, the GUI's compile (Phase 8).

### JPEG XL sources (0.0.2)

What exists of the source formats is in `spec/libs/dds_convert.md` "Accepted image formats".

JPEG XL (`.jxl`) joins the accepted sources as `SourceFormat::Jxl`, decoded to the same RGBA8
as a PNG and converted on the same path, so what the game reads does not change. Authors'
tools have taken it up (browsers decode it since late 2026), and a texture made in one
should not need a PNG copy first. The decoder is Google's `jxl` (BSD-3-Clause, the pure-Rust
decoder Chrome and Firefox ship), version pinned, through `jxl-image-rs-integration` or its
own API, not `jxl-oxide`: on a member's 2048 × 2048 RGBA face texture `jxl` spends 492 ms of
CPU on a lossless decode and 155 ms on a lossy one, `jxl-oxide` 1116 ms and 817 ms (its
threads finish one picture sooner, 165 ms, but the compiler already decodes one texture
per core, so CPU time is what a compile pays). Both decode the lossless file to the PNG's
pixels exactly and the lossy one within one level of libjxl's. A PNG decodes in 24 ms, so a
JXL texture costs a compile more than its PNG. A lossy JXL normal map is taken as a lossy
JPEG one is, with no warning of its own; a lossy JXL also drops the colour under fully
transparent pixels unless its encoder is told to keep it (libjxl's default), as image
editors that zero it do. The rule is one wherever a picture is picked: the aesthetics
export's extension table (`aesthetics_export` "conventions") counts `.jxl` as a texture,
and the Save editor's portrait picker and the Ruleset editor's logo picker list it. The
Export upgrader reads no JXL: no old export can hold one, since Red never read it.

→ verify: a lossless JXL of a PNG fixture decodes to that PNG's pixels exactly; a lossy one
decodes at its size, each channel within a few levels of its source's; a `.jxl` in an
export compiles; the exe's size before and after, measured (`cargo bloat`).

### No BC7 encoding until the GPU encoder (0.0.2)

What exists of the codec rules is in `spec/libs/dds_convert.md` "Target codecs and CPU
encoder", whose table encodes a new texture for PES 19–21 as BC7.

Until the GPU BC7 encoder is wired in (`plans/libs/README.md` "First-release desktop GPU
BC7"), a texture that has to be encoded for PES 19–21 is encoded as for PES 15–18: BC1 when
it is an opaque color texture eligible for it, BC3 otherwise, DXT5nm for a normal map. A BC7
source's blocks are still kept on PES 19–21 (no encoding), and a BC7 source for PES 15–18
is still transcoded to BC3 or BC1. The CPU BC7 encoder stays in the crate for the GPU path's
fallback and tests, never chosen by a compile. Not the CPU's BC7: it spends 31.7 s of CPU on
one 2048 × 2048 face's mip chain where BC3 spends 128 ms (`.tmp/jxl_bench/findings.md`), a
team's textures minutes of all-core time, while BC3 is what every legacy compiler shipped for
those versions. Once the GPU encoder is in, a compile encodes BC7 on the GPU; a machine with
no usable GPU (or one that fails) keeps BC3/BC1 rather than falling back to the CPU's BC7,
for the same reason.

→ verify: a PNG compiled for PES 21 is BC3 (alpha) or BC1 (opaque) in its FTEX; a BC7 DDS
compiled for PES 21 keeps its blocks byte for byte; no compile reaches the BC7 encoder
(a test over the conversion's target table).
