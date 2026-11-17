"""Input and captures for the built app's own window only: the portable copy `setup.py` fills
(`UTW.exe` in the folder `UTW_APP` names, `.tmp/0_0_1s4t_v2/app` by default). Every point is in
client coordinates (on a 96 dpi monitor points = pixels) and asserted inside the client rect.
Captures use PrintWindow, never a screen grab, so a window over the app does not end up in the
picture.

Clicks, drags, the wheel and keys are posted to the app's window as window messages: the real
pointer does not move, no key reaches another program, and the window need not be in front, so
the maintainer can keep working beside a check. Hover cannot be posted (Windows tells the app
the pointer left as soon as a posted move enters, since the real pointer is elsewhere), so
`move`, `burst` and a drag's capture while held move the real pointer: they bring the app to
the front first and refuse a point another window covers.

Usage:
  gui.py info                         client rect and window rect
  gui.py close                        WM_CLOSE to the app's window (never kill it)
  gui.py shot OUT.png                 one PrintWindow capture of the client area
  gui.py wshot OUT.png                the whole window, its title bar included
  gui.py click X,Y [X,Y ...]          left clicks (posted)
  gui.py move X,Y                     the real pointer to a point (hover; takes the foreground)
  gui.py drag X1,Y1 X2,Y2 [OUT.png]   press, move in 20 steps, release (posted); with OUT.png,
                                      a capture while still held (what shows before a drop),
                                      by the real pointer (takes the foreground)
  gui.py wheel X,Y NOTCHES            wheel at a point, negative scrolls down (posted)
  gui.py keys ctrl+z | esc | ...      one chord (posted)
  gui.py burst X,Y OUT_PREFIX N MS    the real pointer to X,Y, then N captures MS apart (takes
                                      the foreground)
  gui.py open PATH | folder PATH      fill the app's own Open / folder dialog and confirm it
  gui.py cancel                       cancel the app's open dialog
  gui.py size W H | maximize          resize (outer size) or maximize the window

With two windows of the copy open, UTW_PID=<pid> picks the one each command acts on.
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
kernel32 = ctypes.windll.kernel32
user32.PostMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
WM_KEYDOWN, WM_KEYUP = 0x0100, 0x0101
WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL = 0x0200, 0x0201, 0x0202, 0x020A
MK_LBUTTON = 0x0001
user32.WindowFromPoint.argtypes = [wt.POINT]
user32.WindowFromPoint.restype = wt.HWND
user32.GetAncestor.argtypes = [wt.HWND, wt.UINT]
user32.GetAncestor.restype = wt.HWND
# The portable copy's folder, matched in its exe's path; another copy through UTW_APP.
DEFAULT_APP = os.path.join(os.path.dirname(__file__), "..", "..", ".tmp", "0_0_1s4t_v2", "app")
APP = os.path.normcase(os.path.abspath(os.environ.get("UTW_APP", DEFAULT_APP))) + "\\"


def app_window():
    pids = {p.pid for p in psutil.process_iter(["exe"])
            if os.path.normcase(p.info["exe"] or "").startswith(APP)}
    assert pids, "the portable copy is not running"
    # Two windows of the copy at once (a check across instances): UTW_PID names the one meant.
    if os.environ.get("UTW_PID"):
        pids &= {int(os.environ["UTW_PID"])}
        assert pids, "UTW_PID is not a running process of the portable copy"
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
    # Inside the client rect is not enough: a topmost window over the app (a notification)
    # takes whatever lands on it.
    under = user32.GetAncestor(user32.WindowFromPoint(pt), 2)  # GA_ROOT
    assert under == HWND, f"{arg} is covered by another window: no input"
    return pt


def move(arg):
    pt = to_screen(arg)
    user32.SetCursorPos(pt.x, pt.y)


def client_point(arg):
    """`X,Y` as a mouse message's lParam, asserted inside the client rect."""
    x, y = (int(float(v)) for v in arg.split(","))
    rect = client()
    assert 0 < x < rect.right and 0 < y < rect.bottom, f"{arg} is outside the client rect"
    return (y << 16) | x


def post_pointer_at(lparam, buttons=0):
    """Post a move to the point. winit reports a move only when the position changed, and the
    leave notice Windows sends after a posted move (the real pointer is elsewhere) clears
    egui's pointer: so a move to the next pixel first, and whatever must happen at the point
    is posted right after this, ahead of that notice."""
    user32.PostMessageW(HWND, WM_MOUSEMOVE, buttons, lparam + 1)
    user32.PostMessageW(HWND, WM_MOUSEMOVE, buttons, lparam)


def post_click(arg):
    lparam = client_point(arg)
    post_pointer_at(lparam)
    user32.PostMessageW(HWND, WM_LBUTTONDOWN, MK_LBUTTON, lparam)
    user32.PostMessageW(HWND, WM_LBUTTONUP, 0, lparam)
    time.sleep(0.6)


def post_wheel(arg, notches):
    lparam = client_point(arg)
    pt = wt.POINT(lparam & 0xFFFF, lparam >> 16)
    user32.ClientToScreen(HWND, ctypes.byref(pt))  # the wheel's point is in screen coordinates
    post_pointer_at(lparam)
    delta = (notches * 120) & 0xFFFF
    user32.PostMessageW(HWND, WM_MOUSEWHEEL, delta << 16, ((pt.y & 0xFFFF) << 16) | (pt.x & 0xFFFF))
    time.sleep(0.6)


def post_drag(start, end):
    (x1, y1), (x2, y2) = ((float(v) for v in a.split(",")) for a in (start, end))
    client_point(end)
    first = client_point(start)
    post_pointer_at(first)
    user32.PostMessageW(HWND, WM_LBUTTONDOWN, MK_LBUTTON, first)
    time.sleep(0.2)
    for step in range(1, 21):
        point = client_point(f"{x1 + (x2 - x1) * step / 20},{y1 + (y2 - y1) * step / 20}")
        user32.PostMessageW(HWND, WM_MOUSEMOVE, MK_LBUTTON, point)
        time.sleep(0.03)
    # The release with a fresh move at the drop point, so egui has the point it drops on.
    last = client_point(end)
    post_pointer_at(last, MK_LBUTTON)
    user32.PostMessageW(HWND, WM_LBUTTONUP, 0, last)
    time.sleep(0.6)


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


KEYS = {
    "ctrl": 0x11, "shift": 0x10, "alt": 0x12, "esc": 0x1B, "enter": 0x0D, "tab": 0x09,
    "space": 0x20, "backspace": 0x08, "delete": 0x2E, "home": 0x24, "end": 0x23,
    "pgup": 0x21, "pgdn": 0x22, "left": 0x25, "up": 0x26, "right": 0x27, "down": 0x28,
}
# The navigation keys sit on the extended keyboard: without the flag their scancode is the
# numeric keypad's, which the app's keyboard layer reads as another key.
EXTENDED = {0x2E, 0x24, 0x23, 0x21, 0x22, 0x25, 0x26, 0x27, 0x28}


# The modifiers and their left-hand keys: winit reads them from the keyboard state, not from the
# key messages.
MODIFIERS = {0x11: 0xA2, 0x10: 0xA0, 0x12: 0xA4}


def keys(chord):
    """Post the chord's key to the app's window. Its modifiers are marked held in the app's
    keyboard state alone: this thread joins the app's input thread (the two then share one
    keyboard state), sets the state, and puts it back once the app has read the keys."""
    codes = [KEYS.get(k) or ord(k.upper()) for k in chord.split("+")]
    app_thread = user32.GetWindowThreadProcessId(HWND, None)
    own = kernel32.GetCurrentThreadId()
    assert user32.AttachThreadInput(own, app_thread, True), "AttachThreadInput refused"
    saved = (ctypes.c_ubyte * 256)()
    user32.GetKeyboardState(saved)
    try:
        held = (ctypes.c_ubyte * 256)(*saved)
        for code in codes:
            if code in MODIFIERS:
                held[code] = held[MODIFIERS[code]] = 0x80
        user32.SetKeyboardState(held)
        plain = [code for code in codes if code not in MODIFIERS]
        for message, order, up in ((WM_KEYDOWN, plain, 0), (WM_KEYUP, plain[::-1], 3 << 30)):
            for code in order:
                scan = user32.MapVirtualKeyW(code, 0)  # MAPVK_VK_TO_VSC
                extended = (1 << 24) if code in EXTENDED else 0
                user32.PostMessageW(HWND, message, code, (scan << 16) | extended | up | 1)
        time.sleep(0.5)
    finally:
        user32.SetKeyboardState(saved)
        user32.AttachThreadInput(own, app_thread, False)
    time.sleep(0.2)
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
    user32.PostMessageW(dialogs[0], 0x0111, 1, 0)  # WM_COMMAND IDOK
    confirm_closed(dialogs[0], path)
    print(f"opened {path}")


def windows_path(path):
    """The path as a member types it: a forward slash is refused, since the dialog may take a
    path a member would never type, or leave itself open on one."""
    assert "/" not in path, f"type the path with backslashes: {path}"
    return path


def confirm_closed(dialog, path):
    """OK only proves the path was accepted once the dialog has gone: a path it rejects leaves
    it open behind an error box, which the next step would otherwise type into. OK is posted,
    not sent, because a sent OK returns only once someone closes that error box."""
    for _ in range(20):
        if not user32.IsWindow(dialog) or not user32.IsWindowVisible(dialog):
            return
        time.sleep(0.25)
    sys.exit(f"the dialog is still open after OK with {path}: look at it, then `gui.py cancel`")


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
    user32.PostMessageW(dialogs[0], 0x0111, 1, 0)  # WM_COMMAND IDOK
    confirm_closed(dialogs[0], path)
    print(f"picked {path}")


cmd, args = sys.argv[1], sys.argv[2:]
if cmd == "open":
    open_dialog(windows_path(args[0]))
elif cmd == "folder":
    folder_dialog(windows_path(args[0]))
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
    for arg in args:
        post_click(arg)
        print(f"clicked {arg}")
elif cmd == "move":
    foreground()
    move(args[0])
    time.sleep(0.5)
    print(f"pointer at {args[0]}")
elif cmd == "drag" and len(args) == 2:
    post_drag(args[0], args[1])
    print(f"dragged {args[0]} -> {args[1]}")
elif cmd == "drag":
    # A capture while held needs a pointer that stays on the window: the real one.
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
    if len(args) > 2:
        time.sleep(0.3)
        shot(args[2])
    user32.mouse_event(4, 0, 0, 0, None)
    time.sleep(0.6)
    print(f"dragged {args[0]} -> {args[1]}")
elif cmd == "wheel":
    post_wheel(args[0], int(args[1]))
    print(f"wheel {args[1]} at {args[0]}")
elif cmd == "keys":
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
