"""Run a command under a Windows pseudo-console (ConPTY) and dump the console's
output stream, so a native binary sees a real terminal on stdout/stderr.

AttachPseudoConsole is not exported on this build, so a helper process is spawned
into the pseudo console first (PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE) and this
process attaches to it through AttachConsole(helper_pid). With our own standard
handles then set to CONIN$/CONOUT$, a plainly spawned child inherits console
handles: its std::io::IsTerminal checks all come out true.

Run: python scripts/conpty_run.py "<command line>" [<working directory>]
"""

import ctypes
import sys
import threading
from ctypes import wintypes

k = ctypes.WinDLL("kernel32", use_last_error=True)

HANDLE = wintypes.HANDLE
DWORD = wintypes.DWORD
LPVOID = wintypes.LPVOID
LPWSTR = wintypes.LPWSTR
WORD = wintypes.WORD
SHORT = wintypes.SHORT
BYTE = wintypes.BYTE
SIZE_T = ctypes.c_size_t


class COORD(ctypes.Structure):
    _fields_ = [("X", SHORT), ("Y", SHORT)]


class STARTUPINFOW(ctypes.Structure):
    _fields_ = [
        ("cb", DWORD),
        ("lpReserved", LPWSTR),
        ("lpDesktop", LPWSTR),
        ("lpTitle", LPWSTR),
        ("dwX", DWORD),
        ("dwY", DWORD),
        ("dwXSize", DWORD),
        ("dwYSize", DWORD),
        ("dwXCountChars", DWORD),
        ("dwYCountChars", DWORD),
        ("dwFillAttribute", DWORD),
        ("dwFlags", DWORD),
        ("wShowWindow", WORD),
        ("cbReserved2", WORD),
        ("lpReserved2", ctypes.POINTER(BYTE)),
        ("hStdInput", HANDLE),
        ("hStdOutput", HANDLE),
        ("hStdError", HANDLE),
    ]


class STARTUPINFOEXW(ctypes.Structure):
    _fields_ = [("StartupInfo", STARTUPINFOW), ("lpAttributeList", LPVOID)]


class PROCESS_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("hProcess", HANDLE),
        ("hThread", HANDLE),
        ("dwProcessId", DWORD),
        ("dwThreadId", DWORD),
    ]


INFINITE = 0xFFFFFFFF
EXTENDED_STARTUPINFO_PRESENT = 0x00080000
PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE = 0x00020016
STD_INPUT_HANDLE = DWORD(-10).value & 0xFFFFFFFF
STD_OUTPUT_HANDLE = DWORD(-11).value & 0xFFFFFFFF
STD_ERROR_HANDLE = DWORD(-12).value & 0xFFFFFFFF
GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 1
FILE_SHARE_WRITE = 2
OPEN_EXISTING = 3
ERROR_ACCESS_DENIED = 5


def winerror(text="win32 error"):
    raise ctypes.WinError(ctypes.get_last_error(), text)


def make_pipe():
    read = HANDLE()
    write = HANDLE()
    if not k.CreatePipe(ctypes.byref(read), ctypes.byref(write), None, 0):
        winerror("CreatePipe")
    return read, write


def open_console(name):
    handle = k.CreateFileW(
        name,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        0,
        None,
    )
    if handle in (None, HANDLE(-1).value) or handle == 0:
        winerror(f"CreateFileW {name}")
    return handle


def spawn(command, cwd, pseudo_console=None):
    """CreateProcessW; when `pseudo_console` is an HPCON the child is attached to
    it through the extended startup attribute."""
    if pseudo_console is None:
        info = STARTUPINFOW()
        info.cb = ctypes.sizeof(info)
        info_arg = ctypes.byref(info)
        flags = 0
        cleanup = []
    else:
        size = SIZE_T()
        k.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(size))
        attrs = ctypes.create_string_buffer(size.value)
        if not k.InitializeProcThreadAttributeList(attrs, 1, 0, ctypes.byref(size)):
            winerror("InitializeProcThreadAttributeList")
        if not k.UpdateProcThreadAttribute(
            attrs,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            pseudo_console,
            ctypes.sizeof(pseudo_console),
            None,
            None,
        ):
            winerror("UpdateProcThreadAttribute")
        info = STARTUPINFOEXW()
        info.StartupInfo.cb = ctypes.sizeof(info)
        info.lpAttributeList = ctypes.cast(attrs, LPVOID)
        info_arg = ctypes.byref(info)
        flags = EXTENDED_STARTUPINFO_PRESENT
        cleanup = [attrs]
    proc = PROCESS_INFORMATION()
    if not k.CreateProcessW(None, command, None, None, False, flags, None, cwd, info_arg, ctypes.byref(proc)):
        winerror("CreateProcessW")
    return proc, cleanup


def main() -> int:
    cmd = sys.argv[1]
    cwd = sys.argv[2] if len(sys.argv) > 2 else None

    in_read, in_write = make_pipe()
    out_read, out_write = make_pipe()

    console = HANDLE()
    hr = k.CreatePseudoConsole(COORD(160, 40), in_read, out_write, 0, ctypes.byref(console))
    if hr != 0:
        raise OSError(hr, "CreatePseudoConsole")
    k.CloseHandle(in_read)
    k.CloseHandle(out_write)

    data = bytearray()
    buf = ctypes.create_string_buffer(65536)

    def reader():
        while True:
            read = DWORD(0)
            if not k.ReadFile(out_read, buf, len(buf), ctypes.byref(read), None):
                break
            if read.value == 0:
                break
            data.extend(buf.raw[: read.value])

    pump = threading.Thread(target=reader, daemon=True)
    pump.start()

    # Null our std handles: an attributed child inherits only real handles and
    # gets the console's for the rest, so with none it gets all three console
    # handles and every stream is a terminal.
    for which in (STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE):
        if not k.SetStdHandle(which, None):
            winerror("SetStdHandle")

    proc, _attrs = spawn(cmd, cwd, console)
    k.WaitForSingleObject(proc.hProcess, INFINITE)
    pump.join(timeout=10)

    exit_code = DWORD()
    k.GetExitCodeProcess(proc.hProcess, ctypes.byref(exit_code))

    k.ClosePseudoConsole(console)
    for handle in (proc.hProcess, proc.hThread, out_read, in_write):
        k.CloseHandle(handle)

    sys.stdout.buffer.write(bytes(data))
    print(f"[exit_code={exit_code.value}]", file=sys.stderr)
    return exit_code.value


if __name__ == "__main__":
    sys.exit(main())
