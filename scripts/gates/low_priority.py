"""The justfile's Windows shell: runs a recipe line at below-normal priority and off the first
two logical processors (one core), unless `UTW_PRIORITY=normal` is set. Usage:
    python scripts/gates/low_priority.py <shell> [args...] <recipe line>

A build or a test gate at normal priority leaves the machine barely usable while it runs, and
one that fills every logical processor slows the desktop even at below-normal priority. Both
settings are made on this process, before it starts the shell, because every process started
from it inherits them: PowerShell, then cargo, rustc, the linker and the test binaries. So a
recipe needs no wrapper of its own, and a run that should take the whole machine asks for it:
`$env:UTW_PRIORITY = 'normal'; just checkpoint`. Exits with the shell's exit code."""

import ctypes
import os
import subprocess
import sys

BELOW_NORMAL_PRIORITY_CLASS = 0x4000


def free_processors(count: int) -> int:
    """How many of the first logical processors a job leaves free: two (one core) of a
    machine with eight or more, one of four to seven, none of fewer."""
    return min(2, count // 4)


def lower_this_process() -> None:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.GetCurrentProcess.restype = ctypes.c_void_p
    kernel32.SetPriorityClass.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    kernel32.SetProcessAffinityMask.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    this = kernel32.GetCurrentProcess()
    if not kernel32.SetPriorityClass(this, BELOW_NORMAL_PRIORITY_CLASS):
        raise ctypes.WinError(ctypes.get_last_error())
    count = os.cpu_count() or 1
    mask = (1 << count) - (1 << free_processors(count))
    if not kernel32.SetProcessAffinityMask(this, mask):
        raise ctypes.WinError(ctypes.get_last_error())


def main() -> int:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    if os.environ.get("UTW_PRIORITY") != "normal":
        lower_this_process()
    return subprocess.run(sys.argv[1:]).returncode


if __name__ == "__main__":
    sys.exit(main())
