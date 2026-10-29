# Unlimited Tool Works — Library crates plan: Format crates

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Recommended approach

### Layout of the format crates: `format/` and `ops/`

What exists of this is in `spec/libs/format_crates.md` "Layout of the format crates: `format/` and `ops/`".

- The `python_bindings` surface is `format/` + `ops/`, exposed as-is — the entry points (file
  read/write, vertex and face decode/encode, the ops), not the codec's internals. The wheel
  carries each codec's `read`/`write` today; the rest joins when the `pes-models` extension's
  hot-path step needs it.
