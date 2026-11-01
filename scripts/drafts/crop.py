"""python scripts/drafts/crop.py SRC DST LEFT TOP RIGHT BOTTOM SCALE: crop a screenshot and enlarge it (nearest)."""
import sys
from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
left, top, right, bottom, scale = (int(v) for v in sys.argv[3:8])
image = Image.open(src).crop((left, top, right, bottom))
image.resize((image.width * scale, image.height * scale), Image.NEAREST).save(dst)
