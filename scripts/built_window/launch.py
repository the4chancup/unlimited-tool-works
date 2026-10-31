"""Start the portable copy detached (its own folder as the working directory, so it reads its
own settings) and give its window time to appear. The copy is the folder `UTW_APP` names,
`.tmp/0_0_1s4t_v2/app` by default. Run: python scripts/built_window/launch.py"""
import os
import subprocess
import time
from pathlib import Path

APP = Path(os.environ.get(
    "UTW_APP", Path(__file__).resolve().parents[2] / ".tmp" / "0_0_1s4t_v2" / "app"))
subprocess.Popen([str(APP / "UTW.exe")], cwd=APP, creationflags=0x00000008 | 0x00000200,
                 stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(4)
print("started")
