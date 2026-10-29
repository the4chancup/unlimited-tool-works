# Unlimited Tool Works developer recipes. Rules: CONTRIBUTING.md "Testing and verification".
# A recipe is a list of commands, never logic; anything with a branch or a loop is a script
# under scripts/ that the recipe calls.

# Git for Windows' `sh` is not on PATH from a plain PowerShell, so recipes run under PowerShell
# there (the fallback CONTRIBUTING.md names). Recipe lines stay bare commands either way.
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

python := if os_family() == "windows" { "python" } else { "python3" }

# The verification gates, in order. CI runs this same recipe.
gates: fmt-check clippy test wasm-check acceptance doc-pointers comment-refs colour-literals

# The same gates on the mutants host (STUDIO_MUTANTS_REMOTE) against the working tree, while this machine should not carry the load
gates-remote:
    {{python}} scripts/remote_gates.py

# One cargo or just command on the mutants host against the working tree (`just remote cargo test -p save_editor`)
remote +command:
    {{python}} scripts/remote_gates.py {{command}}

# Gate 1: formatting
fmt-check:
    cargo fmt --all --check

# Gate 2: lints, warnings denied
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Gate 3: tests, the workspace's test binaries four at once (scripts/run_tests.py says why)
test:
    {{python}} scripts/run_tests_test.py
    {{python}} scripts/run_tests.py
    {{python}} scripts/mutants_test.py
    {{python}} scripts/bundle_drafts_test.py
    {{python}} scripts/draft_compare_test.py

# The gates with gate 3 over the packages the working tree changes since `base` and the packages depending on them (CONTRIBUTING.md "Testing and verification")
gates-changed base="HEAD": fmt-check clippy (test-changed base) wasm-check acceptance doc-pointers comment-refs colour-literals

# Gate 3 over the packages the working tree changes since `base` and the packages depending on them
test-changed base="HEAD":
    {{python}} scripts/run_tests_test.py
    {{python}} scripts/run_tests.py --changed {{base}}
    {{python}} scripts/mutants_test.py
    {{python}} scripts/bundle_drafts_test.py
    {{python}} scripts/draft_compare_test.py

# The GPU BC7 encoder's tests, ignored by `test` because they need a Vulkan or Metal device and
# no CI runner has one: run by hand before landing a change to `dds_convert`'s `gpu` module.
gpu-tests:
    cargo test -p dds_convert --lib -- --ignored gpu::

# Gate 4: studio_core and every lib crate check for wasm32 (list derived from the workspace)
wasm-check:
    {{python}} scripts/wasm_check.py

# Gate 5: the acceptance-ID scanner's own tests, then the scan (`report` fails on untrustworthy citations; `strict`, at converge, also on unproven scenarios)
acceptance mode="report":
    {{python}} scripts/acceptance_test.py
    {{python}} scripts/citation_audit_test.py
    {{python}} scripts/acceptance.py {{mode}}

# Does each test prove the scenario it cites? SubQ Snap asks every citation (converge; needs the network and STUDIO_SNAP_KEY_FILE); report in snap.out/citations.md
citation-audit:
    {{python}} scripts/citation_audit_test.py
    {{python}} scripts/citation_audit.py

# Gate 6: the section-pointer check's own tests, then the check (every `file.md` "Section" pointer resolves)
doc-pointers:
    {{python}} scripts/doc_pointers_test.py
    {{python}} scripts/symbolize_log_test.py
    {{python}} scripts/doc_pointers.py

# Gate 7: the comment check's own tests, then the check (no code comment cites a plan, a decision,
# a step, a review or who decided, instead of standing on its own: CONTRIBUTING.md "Code style")
comment-refs:
    {{python}} scripts/comment_refs_test.py
    {{python}} scripts/comment_refs.py

# Gate 8: the colour check's own tests, then the check (no colour is written from literal values
# outside `studio_core`'s theme.rs, whose palette gives each colour its dark and its light value)
colour-literals:
    {{python}} scripts/colour_literals_test.py
    {{python}} scripts/colour_literals.py

# Mutation run over one crate (converge); survivors are test gaps, equivalents or dead code; split with STUDIO_MUTANTS_REMOTE when set
mutants crate:
    {{python}} scripts/mutants.py {{crate}}

# The remote half of a split run whose `just mutants` stopped early: waits for it, fetches it, prints the summary
mutants-collect:
    {{python}} scripts/mutants.py --collect

# Mutation run over the lines changed since `base` (review of a landed diff)
mutants-diff base="HEAD":
    {{python}} scripts/mutants_diff.py {{base}}

# The mutants cargo-mutants lacks (dropped `if` guards, literal-list elements) over one crate;
# `just mutants` runs the bool, literal, normalize and order kinds inside the whole-crate run, so this stays
# for `--list`, single `--kind` runs and trials of the other kinds
extra-mutants crate *args:
    {{python}} scripts/extra_mutants.py {{crate}} {{args}}

# Retired from every review (CONTRIBUTING.md "Clef scan"); kept for re-measuring a model.
# Clef bug scan of the production lines changed since `base`; report in clef.out/report.md
clef-diff base="HEAD":
    {{python}} scripts/clef_scan.py diff {{base}}

# Clef bug scan of a crate's whole production code (`all` for every crate)
clef crate:
    {{python}} scripts/clef_scan.py crate {{crate}}

# The python_bindings wheel: maturin build, then the smoke test with the wheel on sys.path
# (`just bindings <interpreter>` runs the smoke test under that Python, e.g. Blender's)
bindings interpreter=python:
    {{python}} scripts/bindings_check.py --python {{quote(interpreter)}}

# The two Red-parity tests alone (the Fox, pre-Fox and referee tracer exports against Red's CPKs)
parity:
    cargo test -p team_compiler --test parity --test parity_prefox

# Guardrail 4 (fmdl/pes_model dependency denylist) and the license allowlist (deny.toml)
deps-check:
    {{python}} scripts/deps_check.py
    cargo deny check licenses

# The portable release bundle, target/dist/unlimited-tool-works-<version>-windows.7z, its .sha256
# and the kept symbols' -symbols.7z beside it; Windows only
release version:
    {{python}} scripts/release_test.py
    {{python}} scripts/release.py {{version}}
