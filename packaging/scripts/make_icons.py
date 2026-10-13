"""Renders the raster app icons from assets/icons/sevenplus.svg (DPI-005, PKG-007).

Writes the hicolor PNG set (16-512 px), sevenplus.png (256 px), sevenplus.ico and
sevenplus.icns. Needs ImageMagick (`magick`, with librsvg) and Pillow.

    python3 packaging/scripts/make_icons.py
"""

import subprocess
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ICONS = ROOT / "assets" / "icons"
SVG = ICONS / "sevenplus.svg"
SIZES = [16, 24, 32, 48, 64, 128, 256, 512]


def png(size: int) -> Path:
    return ICONS / "hicolor" / f"{size}x{size}" / "apps" / "sevenplus.png"


def render(size: int) -> None:
    out = png(size)
    out.parent.mkdir(parents=True, exist_ok=True)
    density = 96 * size / 256
    subprocess.run(
        ["magick", "-background", "none", "-density", f"{density}", str(SVG),
         "-resize", f"{size}x{size}", f"PNG32:{out}"],
        check=True,
    )


def main() -> int:
    for size in SIZES:
        render(size)
    Image.open(png(256)).save(ICONS / "sevenplus.png")
    frames = [Image.open(png(s)) for s in SIZES if s <= 256]
    frames[-1].save(ICONS / "sevenplus.ico", sizes=[f.size for f in frames])
    Image.open(png(512)).save(ICONS / "sevenplus.icns")
    print("icons written to", ICONS)
    return 0


if __name__ == "__main__":
    sys.exit(main())
