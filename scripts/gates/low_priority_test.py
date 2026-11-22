"""Tests for low_priority.py: what a command it starts inherits, and its exit code."""

import os
import subprocess
import sys
import unittest
from pathlib import Path

from low_priority import BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, free_processors

SCRIPT = Path(__file__).with_name("low_priority.py")

# Prints the priority class and the affinity mask of the process it runs in.
PROBE = """
import ctypes
k = ctypes.WinDLL("kernel32")
k.GetCurrentProcess.restype = ctypes.c_void_p
k.GetPriorityClass.argtypes = [ctypes.c_void_p]
k.GetProcessAffinityMask.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t), ctypes.POINTER(ctypes.c_size_t)]
mask, system = ctypes.c_size_t(), ctypes.c_size_t()
k.GetProcessAffinityMask(k.GetCurrentProcess(), ctypes.byref(mask), ctypes.byref(system))
print(k.GetPriorityClass(k.GetCurrentProcess()), mask.value)
"""

# Sets its own process to Idle and to the affinity mask in its first argument, as a confined
# job's process is, then runs the command after it, which inherits both.
CONFINE = """
import ctypes, subprocess, sys
k = ctypes.WinDLL("kernel32")
k.GetCurrentProcess.restype = ctypes.c_void_p
k.SetPriorityClass.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
k.SetProcessAffinityMask.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
this = k.GetCurrentProcess()
assert k.SetPriorityClass(this, 0x40) and k.SetProcessAffinityMask(this, int(sys.argv[1]))
sys.exit(subprocess.run(sys.argv[2:]).returncode)
"""


def default_mask() -> int:
    """The processors the wrapper leaves a command on a machine using none of them."""
    count = os.cpu_count()
    return (1 << count) - (1 << free_processors(count))


def probe(priority: str | None, wrapped: bool = True, confined: int | None = None) -> tuple[int, int]:
    """The priority class and affinity mask of a probe started through the wrapper with
    `UTW_PRIORITY` set to `priority` (unset for None), or started directly; with `confined`,
    from a process at Idle on that mask."""
    env = {key: value for key, value in os.environ.items() if key != "UTW_PRIORITY"}
    if priority is not None:
        env["UTW_PRIORITY"] = priority
    confine = [sys.executable, "-c", CONFINE, str(confined)] if confined is not None else []
    wrapper = [sys.executable, str(SCRIPT)] if wrapped else []
    out = subprocess.run(
        [*confine, *wrapper, sys.executable, "-c", PROBE],
        env=env,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    return int(out[0]), int(out[1])


@unittest.skipUnless(os.name == "nt", "the justfile's Windows shell")
class LowPriorityTest(unittest.TestCase):
    def test_a_command_inherits_below_normal_and_leaves_the_first_core(self):
        # Within the processors the test itself runs on: a confined job's gates run it.
        own_mask = probe(None, wrapped=False)[1]
        priority, mask = probe(None)
        self.assertEqual(priority, BELOW_NORMAL_PRIORITY_CLASS)
        self.assertEqual(mask, default_mask() & own_mask)

    def test_a_command_started_lower_stays_idle_and_its_processors_only_narrow(self):
        # Confined to the first and the last processor: the wrapper takes the first away
        # (on a machine with four or more) and adds none.
        last = 1 << (os.cpu_count() - 1)
        priority, mask = probe(None, confined=1 | last)
        self.assertEqual(priority, IDLE_PRIORITY_CLASS)
        self.assertEqual(mask, default_mask() & (1 | last))

    def test_normal_asked_for_leaves_the_command_as_started(self):
        # Compared with a probe started directly, since the test itself may run under a
        # recipe the wrapper lowered.
        self.assertEqual(probe("normal"), probe(None, wrapped=False))

    def test_the_command_s_exit_code_is_returned(self):
        run = subprocess.run(
            [sys.executable, str(SCRIPT), sys.executable, "-c", "raise SystemExit(3)"]
        )
        self.assertEqual(run.returncode, 3)


class FreeProcessorsTest(unittest.TestCase):
    def test_two_of_eight_or_more_one_of_four_to_seven_none_of_fewer(self):
        self.assertEqual([free_processors(n) for n in (3, 4, 7, 8, 16)], [0, 1, 1, 2, 2])


if __name__ == "__main__":
    unittest.main()
