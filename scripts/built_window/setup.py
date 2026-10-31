"""Prepare the portable copy for a built-window check: back up its settings once
(`settings.before_ov1.toml`), copy the release exe built in the main tree (`utw.exe`) over the
copy's `UTW.exe`, and set the settings' window to the default client area (1280 x 780 at
(312, 130)) and the theme to the one asked, so every check starts from one window. Refuses while
the copy runs. The copy is the folder `UTW_APP` names, `.tmp/0_0_1s4t_v2/app` by default.

Run: python scripts/built_window/setup.py dark|light
"""
import os
import re
import shutil
import sys
from pathlib import Path

import psutil

ROOT = Path(__file__).resolve().parents[2]
APP = Path(os.environ.get("UTW_APP", ROOT / ".tmp" / "0_0_1s4t_v2" / "app")).resolve()
EXE = ROOT / "target" / "release" / "utw.exe"
SETTINGS = APP / "data" / "settings.toml"
BACKUP = APP / "data" / "settings.before_ov1.toml"

theme = sys.argv[1]
assert theme in ("dark", "light"), theme
running = [p.pid for p in psutil.process_iter(["exe"]) if str(APP) in (p.info["exe"] or "")]
assert not running, f"the portable copy runs: {running}"
assert EXE.is_file() and SETTINGS.is_file()

if not BACKUP.exists():
    shutil.copy2(SETTINGS, BACKUP)
temp = APP / "UTW.exe.tmp"
shutil.copy2(EXE, temp)
os.replace(temp, APP / "UTW.exe")

text = SETTINGS.read_text(encoding="utf-8")
window = re.compile(r"window_main = \[[^\]]*\]", re.S)
theme_line = re.compile(r'^theme = "[a-z]+"$', re.M)
assert len(window.findall(text)) == 1 and len(theme_line.findall(text)) == 1
text = window.sub("window_main = [312.0, 130.0, 1280.0, 780.0, 1.0, 0.0]", text)
text = theme_line.sub(f'theme = "{theme}"', text)
temp = SETTINGS.with_name("settings.toml.tmp")
with open(temp, "w", encoding="utf-8", newline="\n") as handle:
    handle.write(text)
os.replace(temp, SETTINGS)
print(f"exe copied ({EXE.stat().st_size} bytes), window 1280 x 780, theme {theme}")
