"""Start the portable copy detached (its own folder as the working directory, so it reads its
own settings) and give its window time to appear. The copy is the folder `UTW_APP` names,
`.tmp/0_0_1s4t_v2/app` by default. Run: python scripts/built_window/launch.py"""
import os
import subprocess
import time
from pathlib import Path

APP = Path(os.environ.get(
    "UTW_APP", Path(__file__).resolve().parents[2] / ".tmp" / "0_0_1s4t_v2" / "app"))
# Every window start checks GitHub for a release, and a look session restarts the window often:
# the check reads a closed local port instead, unless the caller points it at a releases list.
env = {**os.environ,
       "UTW_RELEASES_URL": os.environ.get("UTW_RELEASES_URL") or "http://127.0.0.1:0/"}
subprocess.Popen([str(APP / "UTW.exe")], cwd=APP, creationflags=0x00000008 | 0x00000200, env=env,
                 stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(4)
print("started")
