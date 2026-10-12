"""Generates the SevenPlus app icon (PNG + ICO) in assets/icons. Requires Pillow."""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

S = 256
out = Path(sys.argv[1] if len(sys.argv) > 1 else "assets/icons")
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)


def vgrad(box, top, bottom, radius):
    x0, y0, x1, y1 = box
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    for y in range(y0, y1):
        t = (y - y0) / max(1, y1 - y0 - 1)
        c = tuple(int(a + (b - a) * t) for a, b in zip(top, bottom))
        ld.line([(x0, y), (x1, y)], fill=c)
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius, fill=255)
    img.paste(layer, (0, 0), mask)


# Body
vgrad((28, 8, 228, 248), (138, 160, 192, 255), (62, 80, 112, 255), 26)
d.rounded_rectangle((28, 8, 228, 248), 26, outline=(40, 52, 76, 255), width=4)
# Display
vgrad((48, 28, 208, 80), (252, 253, 255, 255), (208, 224, 242, 255), 8)
d.rounded_rectangle((48, 28, 208, 80), 8, outline=(60, 80, 110, 255), width=3)
d.text((120, 36), "7+", fill=(20, 30, 45, 255), font_size=36)
# Keys: 4 x 4, last column orange like the Win7 hover glow
for r in range(4):
    for c in range(4):
        x0, y0 = 48 + c * 41, 98 + r * 36
        box = (x0, y0, x0 + 33, y0 + 28)
        if c == 3:
            vgrad(box, (255, 236, 170, 255), (246, 186, 76, 255), 6)
        else:
            vgrad(box, (246, 249, 253, 255), (204, 216, 233, 255), 6)
        d.rounded_rectangle(box, 6, outline=(70, 88, 120, 255), width=2)

out.mkdir(parents=True, exist_ok=True)
img.save(out / "sevenplus.png")
img.save(out / "sevenplus.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
img.save(out / "sevenplus.icns")
print("icons written to", out)
