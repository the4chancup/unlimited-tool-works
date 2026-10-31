"""List a PE file's static imports, grouped by DLL, and flag the ones absent on Windows 7.

Run: python scripts/win7_imports.py <exe> [<dll name, lower case>...]   (needs `pefile`;
each DLL named also gets its symbols listed)
"""
import sys

import pefile

# Imports known to be Windows 8+ (not exported by Win7 SP1's system DLLs).
WIN8_PLUS = {
    "WaitOnAddress", "WakeByAddressSingle", "WakeByAddressAll", "ProcessPrng",
    "GetSystemTimePreciseAsFileTime", "CreateFile2", "GetDpiForWindow",
    "SetThreadDescription", "GetThreadDescription", "CreateDXGIFactory2",
    "PrefetchVirtualMemory", "GetCurrentThreadStackLimits", "SetProcessDpiAwarenessContext",
    "GetDpiForMonitor", "SetProcessDpiAwareness", "AdjustWindowRectExForDpi",
}

pe = pefile.PE(sys.argv[1], fast_load=True)
pe.parse_data_directories(directories=[
    pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"],
    pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT"],
])
print("subsystem version:", pe.OPTIONAL_HEADER.MajorSubsystemVersion, pe.OPTIONAL_HEADER.MinorSubsystemVersion)
print("os version:", pe.OPTIONAL_HEADER.MajorOperatingSystemVersion, pe.OPTIONAL_HEADER.MinorOperatingSystemVersion)
for kind, entries in (("import", getattr(pe, "DIRECTORY_ENTRY_IMPORT", [])),
                      ("delay", getattr(pe, "DIRECTORY_ENTRY_DELAY_IMPORT", []))):
    for entry in entries:
        names = [i.name.decode() if i.name else f"#{i.ordinal}" for i in entry.imports]
        flagged = [n for n in names if n in WIN8_PLUS]
        print(f"[{kind}] {entry.dll.decode()}: {len(names)} symbols" + (f"  WIN8+: {flagged}" if flagged else ""))
        if entry.dll.decode().lower() in sys.argv[2:]:
            print("    ", ", ".join(names))
