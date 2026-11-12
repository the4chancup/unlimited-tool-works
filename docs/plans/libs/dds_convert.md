# Unlimited Tool Works: Library crates plan: dds_convert

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## In-process DDS conversion (replaces texconv)

### In-memory conversion cache

What exists of this is in `spec/libs/dds_convert.md` "In-memory conversion cache".

- The Studio creates one `Converter` at startup and passes it into each Team compiler run. The
  cache lives for the application session and is dropped on exit: **no disk persistence**, so
  it never fills the user's drive with temporary files.
- **Retention is separately bounded and budgeted.** Repeated edits or switching between teams must
  not accumulate every historical conversion for the whole session. Retained bytes count against
  the shared memory budget even after a task finishes; evict reusable cache entries under pressure
  so they cannot prevent working tasks from acquiring memory. Bypassing the cache for a large run
  does not by itself release entries retained by earlier small runs.

  The policy belongs to the Team compiler pipeline's memory budget and is built with the
  first caller that keeps a `Converter` across runs, the GUI's compile (Phase 8).

### Once the GPU encoder is in: BC7 and uncompressed sources

Built today (`spec/libs/dds_convert.md` "Target codecs and CPU encoder" and the
"Passthrough" item of "In-process DDS conversion (replaces texconv)"): no compile encodes
BC7, and an uncompressed source stays uncompressed but for a normal map and a PES 15-17
kit's main texture.

Once the GPU BC7 encoder is wired in (`plans/libs/README.md` "First-release desktop GPU
BC7"), a compile for PES 19-21 encodes BC7 on the GPU; a machine with no usable GPU (or one
that fails) keeps BC3/BC1 rather than falling back to the CPU's BC7, whose 31.7 s of CPU per
2048 face is the reason none is encoded today.

Encoding uncompressed sources again is reconsidered then, and not for every one:

- BC7 does not reliably keep a pixel's alpha at zero, so a texture relying on it for full
  transparency stays uncompressed;
- a texture under a size floor of about 6 MiB stays as it is.

The floor is what keeps UV Step timing textures uncompressed: they are all small, so it covers
every one without telling them apart from other textures.
