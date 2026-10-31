"""Input and captures for the built app's own window only: the portable copy `setup.py` fills
(`UTW.exe` in the folder `UTW_APP` names, `.tmp/0_0_1s4t_v2/app` by default). Every point is in
client coordinates (on a 96 dpi monitor points = pixels) and asserted inside the client rect, so
a wrong point never lands in another program; input is sent only once the app's window is the
foreground window. Captures use PrintWindow, never a screen grab, so a window over the app does
not end up in the picture.

Usage:
  gui.py info                         client rect and window rect
  gui.py close                        WM_CLOSE to the app's window (never kill it)
  gui.py shot OUT.png                 one PrintWindow capture of the client area
  gui.py wshot OUT.png                the whole window, its title bar included
  gui.py click X,Y [X,Y ...]          left clicks
  gui.py move X,Y                     pointer to a point (hover)
  gui.py drag X1,Y1 X2,Y2             press, move in 20 steps, release
  gui.py wheel X,Y NOTCHES            wheel at a point (negative scrolls down)
  gui.py keys ctrl+z | esc | ...      one chord sent to the foreground app
  gui.py burst X,Y OUT_PREFIX N MS    pointer to X,Y then N captures MS apart
  gui.py open PATH | folder PATH      fill the app's own Open / folder dialog and confirm it
  gui.py cancel                       cancel the app's open dialog
  gui.py size W H | maximize          resize (outer size) or maximize the window
"""
import ctypes
import ctypes.wintypes as wt
import os
import sys
import time

import psutil
from PIL import Image

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
# The portable copy's folder, matched in its exe's path; another copy through UTW_APP.
DEFAULT_APP = os.path.join(os.path.dirname(__file__), "..", "..", ".tmp", "0_0_1s4t_v2", "app")
APP = os.path.normcase(os.path.abspath(os.environ.get("UTW_APP", DEFAULT_APP))) + "\\"


def app_window():
    pids = {p.pid for p in psutil.process_iter(["exe"])
            if os.path.normcase(p.info["exe"] or "").startswith(APP)}
    assert pids, "the portable copy is not running"
    found = []
    proto = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

    def cb(hwnd, _):
        owner = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value in pids and user32.IsWindowVisible(hwnd):
            cls = ctypes.create_unicode_buffer(256)
            user32.GetClassNameW(hwnd, cls, 256)
            if cls.value != "#32770":
                rect = wt.RECT()
                user32.GetClientRect(hwnd, ctypes.byref(rect))
                found.append((rect.right * rect.bottom, hwnd))
        return True

    user32.EnumWindows(proto(cb), 0)
    assert found, "no visible window of the portable copy"
    return max(found)[1]


HWND = app_window()


def client():
    rect = wt.RECT()
    user32.GetClientRect(HWND, ctypes.byref(rect))
    return rect


def foreground():
    # Windows refuses SetForegroundWindow to a process that does not hold the foreground; joined
    # to the foreground window's input thread for the call, this one may (no key is sent).
    if user32.GetForegroundWindow() != HWND:
        kernel32 = ctypes.windll.kernel32
        fg_thread = user32.GetWindowThreadProcessId(user32.GetForegroundWindow(), None)
        own = kernel32.GetCurrentThreadId()
        user32.AttachThreadInput(own, fg_thread, True)
        user32.BringWindowToTop(HWND)
        user32.SetForegroundWindow(HWND)
        user32.AttachThreadInput(own, fg_thread, False)
    time.sleep(0.3)
    assert user32.GetForegroundWindow() == HWND, "the app's window is not foreground: no input"


def to_screen(arg):
    x, y = (float(v) for v in arg.split(","))
    rect = client()
    assert 0 < x < rect.right and 0 < y < rect.bottom, f"{arg} is outside the client rect"
    pt = wt.POINT(int(x), int(y))
    user32.ClientToScreen(HWND, ctypes.byref(pt))
    return pt


def move(arg):
    pt = to_screen(arg)
    user32.SetCursorPos(pt.x, pt.y)


def shot(out, whole=False):
    if whole:
        win = wt.RECT()
        user32.GetWindowRect(HWND, ctypes.byref(win))
        w, h = win.right - win.left, win.bottom - win.top
    else:
        rect = client()
        w, h = rect.right, rect.bottom
    hdc = user32.GetDC(HWND)
    mem = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mem, bmp)
    # PW_RENDERFULLCONTENT, plus PW_CLIENTONLY unless the whole window (title bar included)
    assert user32.PrintWindow(HWND, mem, 2 if whole else 3), "PrintWindow failed"
    buf = ctypes.create_string_buffer(w * h * 4)

    class BMI(ctypes.Structure):
        _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG),
                    ("biPlanes", wt.WORD), ("biBitCount", wt.WORD), ("biCompression", wt.DWORD),
                    ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG),
                    ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD),
                    ("biClrImportant", wt.DWORD)]

    bmi = BMI(ctypes.sizeof(BMI), w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
    gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bmi), 0)
    Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB").save(out)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(HWND, hdc)
    print(f"saved {out} ({w}x{h})")


KEYS = {"ctrl": 0x11, "shift": 0x10, "alt": 0x12, "esc": 0x1B, "enter": 0x0D, "tab": 0x09}


def keys(chord):
    codes = [KEYS.get(k) or ord(k.upper()) for k in chord.split("+")]
    for code in codes:
        user32.keybd_event(code, 0, 0, None)
        time.sleep(0.03)
    for code in reversed(codes):
        user32.keybd_event(code, 0, 2, None)
        time.sleep(0.03)
    time.sleep(0.4)
    print(f"sent {chord}")


def open_dialog(path):
    """The app's own Open dialog: its file-name edit set by WM_SETTEXT, then its OK (IDOK);
    `path` None cancels it (IDCANCEL)."""
    owner_pid = wt.DWORD()
    user32.GetWindowThreadProcessId(HWND, ctypes.byref(owner_pid))
    dialogs = []
    proto = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

    def top(hwnd, _):
        pid = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        cls = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, cls, 256)
        if pid.value == owner_pid.value and cls.value == "#32770" and user32.IsWindowVisible(hwnd):
            dialogs.append(hwnd)
        return True

    user32.EnumWindows(proto(top), 0)
    assert len(dialogs) == 1, f"the app's dialogs: {dialogs}"
    if path is None:
        user32.SendMessageW(dialogs[0], 0x0111, 2, 0)  # WM_COMMAND IDCANCEL
        print("cancelled the dialog")
        return
    edits = []

    def child(hwnd, _):
        cls = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, cls, 256)
        parent = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(user32.GetParent(hwnd), parent, 256)
        if cls.value == "Edit" and parent.value == "ComboBox":
            edits.append(hwnd)
        return True

    user32.EnumChildWindows(dialogs[0], proto(child), 0)
    assert edits, "no file-name edit in the dialog"
    user32.SendMessageW(edits[0], 0x000C, 0, ctypes.c_wchar_p(path))  # WM_SETTEXT
    time.sleep(0.3)
    user32.SendMessageW(dialogs[0], 0x0111, 1, 0)  # WM_COMMAND IDOK
    print(f"opened {path}")


def folder_dialog(path):
    """The app's own folder picker: its Folder box (the dialog's Edit with id 1152; the hidden
    address bar's edit sits in a ComboBox) set by WM_SETTEXT, then Select Folder (IDOK)."""
    owner_pid = wt.DWORD()
    user32.GetWindowThreadProcessId(HWND, ctypes.byref(owner_pid))
    dialogs = []
    proto = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

    def top(hwnd, _):
        pid = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        cls = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, cls, 256)
        if pid.value == owner_pid.value and cls.value == "#32770" and user32.IsWindowVisible(hwnd):
            dialogs.append(hwnd)
        return True

    user32.EnumWindows(proto(top), 0)
    assert len(dialogs) == 1, f"the app's dialogs: {dialogs}"
    edit = user32.GetDlgItem(dialogs[0], 1152)
    assert edit and user32.IsWindowVisible(edit), "no Folder box in the dialog"
    user32.SendMessageW(edit, 0x000C, 0, ctypes.c_wchar_p(path))  # WM_SETTEXT
    time.sleep(0.3)
    user32.SendMessageW(dialogs[0], 0x0111, 1, 0)  # WM_COMMAND IDOK
    print(f"picked {path}")


cmd, args = sys.argv[1], sys.argv[2:]
if cmd == "open":
    open_dialog(args[0])
elif cmd == "folder":
    folder_dialog(args[0])
elif cmd == "cancel":
    open_dialog(None)
elif cmd == "size":
    # The app's own window's outer size, its top-left kept (SWP_NOMOVE | SWP_NOZORDER).
    assert user32.SetWindowPos(HWND, None, 0, 0, int(args[0]), int(args[1]), 0x0002 | 0x0004)
    time.sleep(0.8)
    print(f"outer size {args[0]}x{args[1]}")
elif cmd == "maximize":
    user32.ShowWindow(HWND, 3)  # SW_MAXIMIZE
    time.sleep(1.0)
    print("maximized")
elif cmd == "close":
    # A normal close: a killed process makes the next start report an abnormal end.
    user32.PostMessageW(HWND, 0x0010, 0, 0)  # WM_CLOSE
    time.sleep(3.0)
    print("closed")
elif cmd == "info":
    rect = client()
    win = wt.RECT()
    user32.GetWindowRect(HWND, ctypes.byref(win))
    print(f"client {rect.right}x{rect.bottom}; window ({win.left},{win.top})-({win.right},{win.bottom})")
elif cmd == "shot":
    shot(args[0])
elif cmd == "wshot":
    shot(args[0], whole=True)
elif cmd == "click":
    foreground()
    for arg in args:
        move(arg)
        time.sleep(0.15)
        user32.mouse_event(2, 0, 0, 0, None)
        time.sleep(0.06)
        user32.mouse_event(4, 0, 0, 0, None)
        time.sleep(0.6)
        print(f"clicked {arg}")
elif cmd == "move":
    foreground()
    move(args[0])
    time.sleep(0.5)
    print(f"pointer at {args[0]}")
elif cmd == "drag":
    foreground()
    (x1, y1), (x2, y2) = ((float(v) for v in a.split(",")) for a in args[:2])
    to_screen(args[1])
    move(args[0])
    time.sleep(0.2)
    user32.mouse_event(2, 0, 0, 0, None)
    for step in range(1, 21):
        move(f"{x1 + (x2 - x1) * step / 20},{y1 + (y2 - y1) * step / 20}")
        time.sleep(0.03)
    time.sleep(0.2)
    user32.mouse_event(4, 0, 0, 0, None)
    time.sleep(0.6)
    print(f"dragged {args[0]} -> {args[1]}")
elif cmd == "wheel":
    foreground()
    move(args[0])
    time.sleep(0.2)
    user32.mouse_event(0x0800, 0, 0, int(args[1]) * 120, None)
    time.sleep(0.6)
    print(f"wheel {args[1]} at {args[0]}")
elif cmd == "keys":
    foreground()
    keys(args[0])
elif cmd == "burst":
    prefix, count, gap = args[1], int(args[2]), float(args[3]) / 1000
    foreground()
    move(args[0])
    for index in range(count):
        shot(f"{prefix}_{index:02d}.png")
        time.sleep(gap)
else:
    sys.exit(f"unknown command {cmd}")
