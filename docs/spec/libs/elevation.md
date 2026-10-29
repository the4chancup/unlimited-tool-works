# Unlimited Tool Works — Library crates plan: elevation

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## `libs/elevation`

Admin rights for the tools that need them, in one small platform crate. Compiling
into the PES install directory fails on default Windows installs when PES lives
under `Program Files` — the compilers' normal output target — so all compiler
tools that write to the PES install directory need an elevation path.

- **Detect + relaunch**: `is_elevated()` plus an elevated relaunch of the current
  command line (`ShellExecuteExW` with `runas`, i.e. the UAC consent prompt) —
  Red's `admin_tools.py` replacement. The
  relaunch re-attaches to the same workspace state via ordinary startup (settings
  file, CLI args preserved).
- **CLI behavior**: an access-denied output path fails cleanly — a clear error and
  a non-zero exit — instead of half-written CPKs.

POSIX builds get an euid check in place of UAC. Consumers: `team_compiler`.

The crate's whole surface:

```rust
/// Whether this process already has administrator (Windows) or root (POSIX) rights; `false` on
/// `wasm32`, where the question has no meaning.
pub fn is_elevated() -> bool;
/// Starts `program` with `args` elevated (the UAC consent prompt on Windows) and returns once
/// the new process is launched; the caller then exits. On POSIX and `wasm32` this is
/// `Unsupported`: the tool prints the sudo hint itself, since it knows its own command line.
pub fn relaunch_elevated(program: &Path, args: &[OsString]) -> Result<(), ElevationError>;
/// Whether an I/O error is the access-denied kind that elevation would cure.
pub fn is_access_denied(error: &std::io::Error) -> bool;

pub enum ElevationError {
    Unsupported,       // this platform has no elevation prompt
    Declined,          // the user dismissed the UAC prompt (ERROR_CANCELLED)
    Failed(String),    // the OS call's message
}
```

Windows: `is_elevated` reads the process token's `TokenElevation`; `relaunch_elevated` is
`ShellExecuteExW` with the `runas` verb and `SEE_MASK_NOCLOSEPROCESS`, the arguments quoted for
`CommandLineToArgvW` (a `"` inside an argument doubles the preceding backslashes and escapes the
quote). POSIX: `geteuid() == 0` through `libc`. The `unsafe` blocks (token query, shell
execute, `geteuid`) each carry their `// SAFETY:` comment, as the coding rules require for this
crate. The
`requireAdministrator` manifest feature belongs to the `studio` binary (Phase 8), not here.
Tests can only cover what runs without a prompt: `is_elevated` agrees with the OS's own answer
(`IsUserAnAdmin` on Windows, `getuid` on POSIX), `is_access_denied` on a `PermissionDenied`
error and on another kind, the
argument quoting on the known awkward cases (spaces, embedded quotes, trailing backslashes, an
empty argument) checked against what `CommandLineToArgvW` gives back.

---
