"""Makes the icon files under `resources/icon/` from the maintainer's picture of Archer, the
caster of the reality marble Unlimited Tool Works is named after. Run once when the picture
changes; the outputs are committed, so a build needs no image tool. Needs Pillow.

The maintainer's split (2026-10-27): the whole picture, padded square, at 256, 64, 48 and 32
pixels; at 24 and 16 a tighter square on the head, which ends above the outfit's black collar
(row 1337 of the picture) and its red (row 1344), so the smallest sizes show the face alone.

- `icon.ico`: the executable's file icon, all six sizes (`studio`'s build script compiles it in
  on Windows).
- `small.png`: the window's small icon on Windows (the title bar's, 16 pixels at 100% scaling,
  24 at 150%), the tight crop at 64 pixels for Windows to scale down.
- `window.png`: the window's icon elsewhere, where one image serves every size: the whole
  picture at 256.

Run: python scripts/make_icon.py <picture.png>
"""
import os
import sys
from pathlib import Path

from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "resources" / "icon"
# The head, in the picture's pixels: 17 rows above the collar, the hair to the cheek.
TIGHT = (200, 0, 1520, 1320)
WHOLE_SIZES = [256, 64, 48, 32]
TIGHT_SIZES = [24, 16]


def save_replacing(image: Image.Image, path: Path, **options) -> None:
    """Writes beside `path` first, then replaces it, so a failed save leaves the old file."""
    temp = path.with_name(path.name + ".tmp")
    image.save(temp, format=path.suffix[1:].upper(), **options)
    os.replace(temp, path)


def main() -> None:
    src = Image.open(sys.argv[1]).convert("RGBA")
    assert src.size == (1668, 1636), f"not the picture the crop was measured on: {src.size}"
    side = max(src.size)
    whole = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    whole.paste(src, ((side - src.width) // 2, (side - src.height) // 2))
    tight = src.crop(TIGHT)

    by_size = {s: whole.resize((s, s), Image.LANCZOS) for s in WHOLE_SIZES}
    by_size.update({s: tight.resize((s, s), Image.LANCZOS) for s in TIGHT_SIZES})
    sizes = sorted(by_size, reverse=True)
    OUT.mkdir(parents=True, exist_ok=True)
    # The ICO writer takes each listed size from the image of that size among the first
    # and `append_images`, so every size keeps its own crop rather than one resized image.
    save_replacing(by_size[sizes[0]], OUT / "icon.ico", sizes=[(s, s) for s in sizes],
                   append_images=[by_size[s] for s in sizes[1:]])
    save_replacing(tight.resize((64, 64), Image.LANCZOS), OUT / "small.png")
    save_replacing(by_size[256], OUT / "window.png")

    # Read the ICO back: every size present, and each the image made for it.
    ico = Image.open(OUT / "icon.ico")
    stored = sorted(s[0] for s in ico.ico.sizes())
    assert stored == sorted(sizes), f"the ICO holds {stored}"
    for s in sizes:
        got = ico.ico.getimage((s, s)).convert("RGBA")
        assert got.tobytes() == by_size[s].tobytes(), f"the ICO's {s}-pixel image differs"
    print("written", OUT, "ICO sizes", stored)


if __name__ == "__main__":
    main()
