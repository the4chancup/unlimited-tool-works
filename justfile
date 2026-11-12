# Unlimited Tool Works developer recipes. Rules: CONTRIBUTING.md "Testing and verification".
# A recipe is a list of commands, never logic; anything with a branch or a loop is a script
# under scripts/ that the recipe calls.

# Git for Windows' `sh` is not on PATH from a plain PowerShell, so recipes run under PowerShell
# there (the fallback CONTRIBUTING.md names). Recipe lines stay bare commands either way.
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

python := if os_family() == "windows" { "python" } else { "python3" }
# A script's tests run with a temp folder that is not its own real path, as a CI runner's is
# (scripts/gates/temp_alias.py)
script_test := python + " scripts/gates/temp_alias.py"

# The verification gates, in order. CI runs this same recipe.
gates: line-endings fmt-check clippy test wasm-check acceptance doc-pointers comment-refs colour-literals commit-messages

# The same gates on the mutants host (STUDIO_MUTANTS_REMOTE) against the working tree, while this machine should not carry the load
gates-remote:
    {{python}} scripts/mutation/remote_gates.py

# One cargo or just command on the mutants host against the working tree (`just remote cargo test -p save_editor`)
remote +command:
    {{python}} scripts/mutation/remote_gates.py {{command}}

# Gate 1: formatting
fmt-check:
    cargo fmt --all --check

# Gate 2: lints, warnings denied
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Gate 3: tests, the workspace's test binaries four at once (scripts/gates/run_tests.py says why)
test:
    {{script_test}} scripts/gates/run_tests_test.py
    {{script_test}} scripts/gates/checkpoint_test.py
    {{python}} scripts/gates/run_tests.py
    {{script_test}} scripts/mutation/mutants_test.py
    {{script_test}} scripts/review/perturb_check_test.py
    {{script_test}} scripts/review/clones_test.py
    {{script_test}} scripts/drafts/bundle_drafts_test.py
    {{script_test}} scripts/drafts/draft_compare_test.py
    {{script_test}} scripts/census/census_test.py
    {{script_test}} scripts/library_parity/checkpoint_test.py

# The gates with gate 3 over the packages the working tree changes since `base` and the packages depending on them (CONTRIBUTING.md "Testing and verification")
gates-changed base="HEAD": line-endings fmt-check clippy (test-changed base) wasm-check acceptance doc-pointers comment-refs colour-literals (commit-messages base)

# Every commit since `base` has a message of the project's shape (CONTRIBUTING.md "Commits"); `scripts/commit.py` checks one before it commits it
commit-messages base="HEAD":
    {{script_test}} scripts/gates/commit_message_test.py
    {{python}} scripts/gates/commit_message.py --range {{base}}

# A checkpoint, in one command: `gates-changed` and `mutants-diff` since the last gated commit, which it then
# moves to HEAD (scripts/gates/checkpoint.py; `--no-mutants` skips the mutation run)
checkpoint *args:
    {{python}} scripts/gates/checkpoint.py run {{args}}

# The checkpoint, then the Linux gates, on HEAD's commit in the gate worktree, so this tree stays free
# meanwhile (scripts/gates/pipeline.py; start it detached and logged)
pipeline *args:
    {{python}} scripts/gates/pipeline.py {{args}}

# How far HEAD is past the last gated commit, what is not pushed, and CI's verdict on the pushed head
checkpoint-status *args:
    {{python}} scripts/gates/checkpoint.py status {{args}}

# Gate 3 over the packages the working tree changes since `base` and the packages depending on them
test-changed base="HEAD":
    {{script_test}} scripts/gates/run_tests_test.py
    {{script_test}} scripts/gates/checkpoint_test.py
    {{python}} scripts/gates/run_tests.py --changed {{base}}
    {{script_test}} scripts/mutation/mutants_test.py
    {{script_test}} scripts/review/perturb_check_test.py
    {{script_test}} scripts/review/clones_test.py
    {{script_test}} scripts/drafts/bundle_drafts_test.py
    {{script_test}} scripts/drafts/draft_compare_test.py
    {{script_test}} scripts/census/census_test.py
    {{script_test}} scripts/library_parity/checkpoint_test.py

# The GPU BC7 encoder's tests, ignored by `test` because they need a Vulkan or Metal device and
# no CI runner has one: run by hand before landing a change to `dds_convert`'s `gpu` module.
gpu-tests:
    cargo test -p dds_convert --lib -- --ignored gpu::

# Gate 4: studio_core and every lib crate check for wasm32 (list derived from the workspace)
wasm-check:
    {{python}} scripts/gates/wasm_check.py

# Gate 5: the acceptance-ID scanner's own tests, then the scan (`report` fails on untrustworthy citations; `strict`, at converge, also on unproven scenarios)
acceptance mode="report":
    {{script_test}} scripts/gates/acceptance_test.py
    {{script_test}} scripts/gates/citation_audit_test.py
    {{python}} scripts/gates/acceptance.py {{mode}}

# Does each test prove the scenario it cites? SubQ Snap asks every citation (converge; needs the network and STUDIO_SNAP_KEY_FILE); report in snap.out/citations.md
citation-audit:
    {{script_test}} scripts/gates/citation_audit_test.py
    {{python}} scripts/gates/citation_audit.py

# Gate 6: the section-pointer check's own tests, then the check (every `file.md` "Section" pointer resolves)
doc-pointers:
    {{script_test}} scripts/gates/doc_pointers_test.py
    {{script_test}} scripts/release/symbolize_log_test.py
    {{python}} scripts/gates/doc_pointers.py

# Gate 7: the comment check's own tests, then the check (no code comment cites a plan, a decision,
# a step, a review or who decided, instead of standing on its own: CONTRIBUTING.md "Code style")
comment-refs:
    {{script_test}} scripts/gates/comment_refs_test.py
    {{python}} scripts/gates/comment_refs.py

# Gate 8: the colour check's own tests, then the check (no colour is written from literal values
# outside `studio_core`'s theme.rs, whose palette gives each colour its dark and its light value)
colour-literals:
    {{script_test}} scripts/gates/colour_literals_test.py
    {{python}} scripts/gates/colour_literals.py

# Gate 0, first since it takes a second: the line-endings check's own tests, then the check (every
# tracked file's working copy has the line endings .gitattributes asks: CONTRIBUTING.md "Toolchain notes")
line-endings:
    {{script_test}} scripts/gates/line_endings_test.py
    {{python}} scripts/gates/line_endings.py

# Mutation run over one crate (converge); survivors are test gaps, equivalents or dead code; every mutant on the STUDIO_MUTANTS_REMOTE host, refused without one
mutants crate:
    {{python}} scripts/mutation/mutants.py {{crate}}

# The remote half of a whole or split run whose command stopped early: waits for it, fetches it, prints the summary
mutants-collect:
    {{python}} scripts/mutation/mutants.py --collect

# Mutation run over the lines changed since `base` (review of a landed diff), under
# `crates` or the paths given (a crate's folder)
mutants-diff base="HEAD" *paths:
    {{python}} scripts/mutation/mutants_diff.py {{base}} {{paths}}

# The mutants cargo-mutants lacks (dropped `if` guards, literal-list elements) over one crate;
# `just mutants` runs the bool, literal, normalize and order kinds inside the whole-crate run, so this stays
# for `--list`, single `--kind` runs and trials of the other kinds
extra-mutants crate *args:
    {{python}} scripts/mutation/extra_mutants.py {{crate}} {{args}}

# The python_bindings wheel: maturin build, then the smoke test with the wheel on sys.path
# (`just bindings <interpreter>` runs the smoke test under that Python, e.g. Blender's)
bindings interpreter=python:
    {{python}} scripts/gates/bindings_check.py --python {{quote(interpreter)}}

# The two Red-parity tests alone (the Fox, pre-Fox and referee tracer exports against Red's CPKs)
parity:
    cargo test -p team_compiler --test parity --test parity_prefox

# docs/web/crate_map.html, the crates' dependency map, redrawn from the manifests (after a crate is added)
crate-map:
    {{python}} scripts/crate_map.py

# The census index of the real exports' textures and FMDL texture references, brought up to date
# (.census/index.sqlite; queries: scripts/census/README.md)
census-index *args:
    {{python}} scripts/census/index.py {{args}}

# Guardrail 4 (fmdl/pes_model dependency denylist), the license allowlist (deny.toml) and the
# crates allowed to compile C (scripts/gates/c_crates.py)
deps-check:
    {{python}} scripts/gates/deps_check.py
    cargo deny check licenses
    {{script_test}} scripts/gates/c_crates_test.py
    {{python}} scripts/gates/c_crates.py

# The portable release bundle of the platform it runs on, target/dist/unlimited-tool-works-<version>-
# windows.7z or -linux.tar.gz, its .sha256 and the kept symbols' archive beside it
release version:
    {{script_test}} scripts/release/release_test.py
    {{python}} scripts/release/release.py {{version}}
