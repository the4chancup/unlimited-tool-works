"""A burst of screenshots of a running game, tiled into contact sheets.

  python scripts/ingame/burst.py GAME COUNT NAME [INTERVAL]

Takes COUNT screenshots of GAME's window (GAME as in game.py) INTERVAL seconds apart (default 2)
as NAME_NN.png, then tiles them 16 to a sheet, two columns of 512x288 each, as NAME_sheetN.png.
A bare NAME (`ref04`) writes into `.tmp/ingame/` of the repository, the gitignored scratch
folder, created if missing; a NAME with a folder in it (`.tmp/phase_04/4_19/ingame/ref04`) is used as given.
Needs what game.py's `shot` needs (`mss`, `Pillow`).
"""

import sys
import time
from pathlib import Path

from PIL import Image

from game import GAMES, cmd_shot

DEFAULT_FOLDER = Path(__file__).resolve().parents[2] / ".tmp" / "ingame"
PER_SHEET = 16


def output_prefix(name: str) -> Path:
    prefix = Path(name)
    if prefix.parent != Path("."):
        return prefix
    DEFAULT_FOLDER.mkdir(parents=True, exist_ok=True)
    return DEFAULT_FOLDER / name


def main(args: list[str]) -> int:
    if len(args) not in (3, 4) or args[0] not in GAMES:
        print(__doc__, file=sys.stderr)
        return 2
    game = GAMES[args[0]]
    count = int(args[1])
    prefix = output_prefix(args[2])
    interval = float(args[3]) if len(args) > 3 else 2.0
    names = []
    for i in range(1, count + 1):
        name = f"{prefix}_{i:02}.png"
        cmd_shot(game, name, 1.0)
        names.append(name)
        time.sleep(interval)
    for s, start in enumerate(range(0, count, PER_SHEET)):
        chunk = names[start:start + PER_SHEET]
        sheet = Image.new("RGB", (1024, 288 * ((len(chunk) + 1) // 2)))
        for i, name in enumerate(chunk):
            sheet.paste(Image.open(name).resize((512, 288)), ((i % 2) * 512, (i // 2) * 288))
        sheet.save(f"{prefix}_sheet{s + 1}.png")
        print(f"{prefix}_sheet{s + 1}.png: {chunk[0]} .. {chunk[-1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
