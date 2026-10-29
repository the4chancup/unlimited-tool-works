//! The built exe is a Windows GUI-subsystem application on every build, so a
//! double-click opens no console window beside the GUI. The subsystem is a field of the PE header the linker wrote: the DOS
//! header's `e_lfanew` (the `u32` little-endian at `0x3C`) points at the `PE\0\0`
//! signature, the 20-byte file header follows it, and the optional header's
//! `Subsystem` field sits at that header's offset 68 — `e_lfanew + 4 + 20 + 68`,
//! `e_lfanew + 0x5C`.
#![cfg(windows)] // the subsystem split is Windows's

use std::fs;

/// IMAGE_SUBSYSTEM_WINDOWS_GUI is 2; 3 is IMAGE_SUBSYSTEM_WINDOWS_CUI, the console's.
#[test]
fn the_binary_is_a_gui_subsystem_executable() {
    let bytes = fs::read(env!("CARGO_BIN_EXE_utw")).unwrap();
    let e_lfanew = u32::from_le_bytes(bytes[0x3C..0x40].try_into().unwrap()) as usize;
    assert_eq!(
        &bytes[e_lfanew..e_lfanew + 4],
        b"PE\0\0",
        "no PE signature where e_lfanew points"
    );
    let subsystem = u16::from_le_bytes(bytes[e_lfanew + 0x5C..e_lfanew + 0x5E].try_into().unwrap());
    assert_eq!(subsystem, 2, "the exe is not a GUI-subsystem application");
}
