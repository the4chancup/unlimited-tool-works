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
