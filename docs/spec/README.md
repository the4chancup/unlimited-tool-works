# Unlimited Tool Works — Spec index

What the suite does now, and why: one document per concern, with the same files and headings as
the [plans](../plans/README.md), which hold the work not built yet. A section partly built
appears in both trees under the same heading. Start with the [core spec](core/README.md)
(overview, crate structure, tool plugin system, parallelism, the phases delivered so far, key
decisions); the Team compiler and the library crates have their own documents.

| Spec | Covers |
|------|--------|
| [Core](core/README.md) | Platform architecture and cross-cutting decisions |
| [Team compiler](team_compiler/README.md) | The flagship tool: pipeline, planning, processing, packing, deployment, referee support, messages, settings, the CLI |
| [Save editor](save_editor.md) | The Save editor's CLI over `pes_savefile`: Team TOML, the aesthetics patch, AATF checks, the comparator, the teams list; its messages |
| [AATF rules](aatf_rules.md) | `libs/aatf`: the ruleset schema, the interpreter and host, the embedded rulesets |
| [Export upgrader](export_upgrader.md) | The one-time migration of AET exports to the Studio format, and of old model folders, through the CLI; its messages |
| [Aesthetics export](aesthetics_export/README.md) | The `aesthetics_export` lib and the Studio aesthetics export format: object model and validation progression, player folders, FPC markers, `settings.toml` |
| [Kit config editor](kit_config_editor.md) | The kit config format reference (recovered by disassembly), TOML configs in exports, and the `kit_config` lib |
| [Unified model format](model_format.md) | The material schema (shader families, canonical texture roles, per-engine tables), `.mtl` search, `.common` links |
| [Model conversion](model_conversion/README.md) | `model_convert` lib: IR, conversion between the engines, mesh splitting, the Python bindings' side of the Blender integration |
| [Savefile](pes_savefile/README.md) | `pes_savefile` lib: crypto, schema-driven codec, player/team/tactics model, interchange formats |
| [Library crates](libs/README.md) | The remaining lib crates: format parsers, DDS conversion, color tools (kit-color extraction), archives, the task pipeline |

How the two trees evolve is in the plan index, "How these documents evolve". Coding rules live
in [`../CONTRIBUTING.md`](../CONTRIBUTING.md), domain terms in [`../GLOSSARY.md`](../GLOSSARY.md),
implementation status in [`../WORKLOG.md`](../WORKLOG.md), and choices made where the documents
were silent in [`../DECISIONS.md`](../DECISIONS.md).
