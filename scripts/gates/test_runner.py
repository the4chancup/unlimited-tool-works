"""Windows test-runner memory cap: `test_runner.py <cap> <exe> [args...]`.

Windows has no OOM killer: a runaway test binary (a mutation that pushes
into a Vec forever) pages until the commit limit runs out and whichever
process allocates next — browser, game, IDE — fails instead. Wrapping the
child in a Job Object with `JOB_OBJECT_LIMIT_PROCESS_MEMORY` makes it fail
its own allocation and abort, alone. `cargo test` runs every test binary
through `target.'cfg(windows)'.runner` (see `.cargo/config.toml`).

The runner assigns *its own* process to the job before spawning, so the
child inherits it; `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` means a timeout kill
of the runner takes the child with it instead of orphaning it. A completion
port attached to the job reports `JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT` the
moment a member *attempts* an over-cap allocation — a peak-committed check
misses those, since a failed request never commits (a doubling Vec dies at
half the cap, having asked for more than it).
"""

import ctypes
import subprocess
import sys
from ctypes import wintypes
from pathlib import Path

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

JOB_OBJECT_LIMIT_PROCESS_MEMORY = 0x00000100
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_EXTENDED_LIMIT_INFORMATION = 9
JOB_OBJECT_ASSOCIATE_COMPLETION_PORT_INFORMATION = 7
JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT = 9
INVALID_HANDLE_VALUE = wintypes.HANDLE(-1)


class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_int64),
        ("PerJobUserTimeLimit", ctypes.c_int64),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class IO_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_uint64),
        ("WriteOperationCount", ctypes.c_uint64),
        ("OtherOperationCount", ctypes.c_uint64),
        ("ReadTransferCount", ctypes.c_uint64),
        ("WriteTransferCount", ctypes.c_uint64),
        ("OtherTransferCount", ctypes.c_uint64),
    ]


class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
        ("IoInfo", IO_COUNTERS),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class JOBOBJECT_ASSOCIATE_COMPLETION_PORT(ctypes.Structure):
    _fields_ = [
        ("CompletionKey", ctypes.c_void_p),
        ("CompletionPort", wintypes.HANDLE),
    ]


kernel32.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
kernel32.CreateJobObjectW.restype = wintypes.HANDLE
kernel32.SetInformationJobObject.argtypes = [
    wintypes.HANDLE,
    ctypes.c_int,
    wintypes.LPVOID,
    wintypes.DWORD,
]
kernel32.SetInformationJobObject.restype = wintypes.BOOL
kernel32.CreateIoCompletionPort.argtypes = [
    wintypes.HANDLE,
    wintypes.HANDLE,
    ctypes.c_void_p,
    wintypes.DWORD,
]
kernel32.CreateIoCompletionPort.restype = wintypes.HANDLE
kernel32.GetQueuedCompletionStatus.argtypes = [
    wintypes.HANDLE,
    wintypes.LPDWORD,
    ctypes.POINTER(ctypes.c_void_p),
    ctypes.POINTER(ctypes.c_void_p),
    wintypes.DWORD,
]
kernel32.GetQueuedCompletionStatus.restype = wintypes.BOOL
kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
kernel32.GetCurrentProcess.argtypes = []
kernel32.GetCurrentProcess.restype = wintypes.HANDLE


def make_job(cap_bytes: int, port: wintypes.HANDLE) -> wintypes.HANDLE:
    """A Job Object capping each member process at `cap_bytes`, reporting
    limit violations to `port`, killed when the job handle closes."""
    job = kernel32.CreateJobObjectW(None, None)
    if not job:
        raise ctypes.WinError(ctypes.get_last_error())
    info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
    info.BasicLimitInformation.LimitFlags = (
        JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    )
    info.ProcessMemoryLimit = cap_bytes
    if not kernel32.SetInformationJobObject(
        job,
        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
        ctypes.byref(info),
        ctypes.sizeof(info),
    ):
        raise ctypes.WinError(ctypes.get_last_error())
    assoc = JOBOBJECT_ASSOCIATE_COMPLETION_PORT()
    assoc.CompletionPort = port
    if not kernel32.SetInformationJobObject(
        job,
        JOB_OBJECT_ASSOCIATE_COMPLETION_PORT_INFORMATION,
        ctypes.byref(assoc),
        ctypes.sizeof(assoc),
    ):
        raise ctypes.WinError(ctypes.get_last_error())
    return job


def cap_bytes(text: str) -> int:
    """`<n>M` or `<n>G` to bytes; anything else is an error."""
    if len(text) > 1 and text[-1] in "MG" and text[:-1].isdigit() and int(text[:-1]) > 0:
        return int(text[:-1]) << (20 if text[-1] == "M" else 30)
    print(f"test_runner: invalid cap {text!r} (want e.g. 256M, 8G)", file=sys.stderr)
    sys.exit(2)


def hit_memory_limit(port: wintypes.HANDLE) -> bool:
    """Drain the job's completion port (non-blocking); True if a member hit
    its per-process memory cap. Each job message arrives as
    `lpNumberOfBytesTransferred` = `JOB_OBJECT_MSG_*`; the port is empty when
    the call fails with no packet (a 0 ms timeout)."""
    hit = False
    while True:
        msg, key, overlapped = wintypes.DWORD(), ctypes.c_void_p(), ctypes.c_void_p()
        ok = kernel32.GetQueuedCompletionStatus(
            port,
            ctypes.byref(msg),
            ctypes.byref(key),
            ctypes.byref(overlapped),
            0,
        )
        if not ok and not overlapped.value:
            return hit
        if ok and msg.value == JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT:
            hit = True


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(f"usage: {argv[0]} <cap> <exe> [args...]", file=sys.stderr)
        return 2
    cap, exe, args = cap_bytes(argv[1]), argv[2], argv[3:]
    port = kernel32.CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1)
    if not port:
        raise ctypes.WinError(ctypes.get_last_error())
    job = make_job(cap, port)
    # This process joins the job so the child inherits it; the runner itself
    # is tiny and nowhere near the cap.
    if not kernel32.AssignProcessToJobObject(job, kernel32.GetCurrentProcess()):
        raise ctypes.WinError(ctypes.get_last_error())
    child = subprocess.run([exe, *args])
    # Both handles stay open for the runner's life: `KILL_ON_JOB_CLOSE` kills
    # every member the moment the last handle closes — closing the job here
    # would kill the runner itself, and the timeout-kill path relies on it.
    if hit_memory_limit(port):
        print(
            f"test_runner: {Path(exe).name} hit the {argv[1]} per-process "
            "memory cap (.cargo/config.toml)",
            file=sys.stderr,
        )
    # `sys.exit` passes the code to ExitProcess, so a DWORD like Rust's
    # 0xC0000409 abort round-trips unchanged.
    return child.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv))
