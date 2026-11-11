"""JPEG XL fixtures (0.0.2a): lossless and lossy encodes of existing PNG fixtures, by libjxl.

Run with a Python holding `imagecodecs` (2026.10.10, libjxl 0.12.0), `numpy` and Pillow:

    python scripts/provenance/fixtures/fixtures_jxl.py

Writes, each through a sibling temp file and `os.replace` after every check:

- `crates/libs/dds_convert/tests/fixtures/source.lossless.jxl`: `source.png` (32x16 RGBA, the
  alpha ramp) lossless, effort 7; libjxl's own decode asserted equal to the PNG's pixels.
- `crates/libs/dds_convert/tests/fixtures/source.lossy.jxl`: the same at distance 1.0, effort 7.
- `crates/libs/dds_convert/tests/fixtures/source.lossy.jxl.rgba`: libjxl's decode of the lossy
  file, raw RGBA8 rows (the reference a decoder is judged against, as `source.webp.rgba`).
- `crates/tools/team_compiler/tests/fixtures/textures/kit.jxl` and `portrait.jxl`: `kit.png` and
  `portrait.png` lossless, effort 7, each asserted to decode to its PNG's pixels.
"""
import os
import sys
from pathlib import Path

import imagecodecs
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
DDS = ROOT / "crates/libs/dds_convert/tests/fixtures"
TEXTURES = ROOT / "crates/tools/team_compiler/tests/fixtures/textures"


def rgba(path):
    """The PNG's pixels as an (h, w, 4) uint8 array."""
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)


def lossless(pixels):
    data = imagecodecs.jpegxl_encode(pixels, lossless=True, effort=7)
    assert np.array_equal(imagecodecs.jpegxl_decode(data), pixels), "lossless round trip"
    return data


def write(path, data):
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_bytes(data)
    os.replace(tmp, path)
    print(f"wrote {path.relative_to(ROOT)} ({len(data)} bytes)")


def main():
    assert imagecodecs.jpegxl_version() == "libjxl 0.12.0", imagecodecs.jpegxl_version()
    source = rgba(DDS / "source.png")
    assert source.shape == (16, 32, 4), source.shape
    lossy = imagecodecs.jpegxl_encode(source, distance=1.0, effort=7)
    reference = imagecodecs.jpegxl_decode(lossy)
    assert reference.shape == source.shape and reference.dtype == np.uint8, reference.shape
    kit = rgba(TEXTURES / "kit.png")
    portrait = rgba(TEXTURES / "portrait.png")
    outputs = [
        (DDS / "source.lossless.jxl", lossless(source)),
        (DDS / "source.lossy.jxl", lossy),
        (DDS / "source.lossy.jxl.rgba", reference.tobytes()),
        (TEXTURES / "kit.jxl", lossless(kit)),
        (TEXTURES / "portrait.jxl", lossless(portrait)),
    ]
    for path, data in outputs:
        write(path, data)


if __name__ == "__main__":
    sys.exit(main())
