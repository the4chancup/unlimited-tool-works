"""Print each monitor's rectangle and DPI in physical pixels (per-monitor-v2 DPI aware): run
before a built-window check, since which monitor is primary and its scaling decide where the
app's window opens and what a capture's pixels mean. Run: python scripts/built_window/monitors.py"""
import ctypes
from ctypes import wintypes

user32 = ctypes.windll.user32
shcore = ctypes.windll.shcore
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))  # PER_MONITOR_AWARE_V2


class MONITORINFOEX(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.DWORD), ("rcMonitor", wintypes.RECT), ("rcWork", wintypes.RECT),
                ("dwFlags", wintypes.DWORD), ("szDevice", wintypes.WCHAR * 32)]


def callback(hmon, hdc, rect, data):
    info = MONITORINFOEX()
    info.cbSize = ctypes.sizeof(info)
    user32.GetMonitorInfoW(hmon, ctypes.byref(info))
    x, y = ctypes.c_uint(), ctypes.c_uint()
    shcore.GetDpiForMonitor(hmon, 0, ctypes.byref(x), ctypes.byref(y))
    r, w = info.rcMonitor, info.rcWork
    print(info.szDevice, "monitor", (r.left, r.top, r.right, r.bottom), "work", (w.left, w.top, w.right, w.bottom),
          "dpi", x.value, "primary" if info.dwFlags & 1 else "")
    return True


PROC = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(wintypes.RECT), ctypes.c_double)
user32.EnumDisplayMonitors(None, None, PROC(callback), 0)
