"""Builds Kaze's Android launcher icons (adaptive + legacy) from the desktop icon art.
Usage: python3 scripts/android_icon.py src-tauri/icons/icon.png src-tauri/gen/android/app/src/main/res preview.png"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter, ImageStat

src, res, preview = Image.open(sys.argv[1]).convert("RGB"), Path(sys.argv[2]), sys.argv[3]
art = src.crop((48, 48, 464, 464))  # inside the rounded-square border
edge = Image.new("L", art.size, 0)
ImageDraw.Draw(edge).rectangle((0, 0, art.width - 1, art.height - 1), outline=255, width=6)
bg = tuple(int(v) for v in ImageStat.Stat(art, mask=edge).mean)  # the art's own edge colour

def layer(size: int, motif: float) -> Image.Image:
    """An opaque square: the background colour with the art centred, edges feathered into it."""
    out = Image.new("RGB", (size, size), bg)
    s = round(size * motif)
    piece = art.resize((s, s), Image.LANCZOS)
    mask = Image.new("L", (s, s), 0)
    inset = max(2, s // 14)
    ImageDraw.Draw(mask).rectangle((inset, inset, s - inset, s - inset), fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(inset / 2))
    out.paste(piece, ((size - s) // 2, (size - s) // 2), mask)
    return out

def circle(img: Image.Image) -> Image.Image:
    m = Image.new("L", img.size, 0)
    ImageDraw.Draw(m).ellipse((0, 0, img.width - 1, img.height - 1), fill=255)
    out = Image.new("RGBA", img.size, (0, 0, 0, 0))
    out.paste(img, (0, 0), m)
    return out

DENSITY = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}
for name, k in DENSITY.items():
    d = res / f"mipmap-{name}"
    d.mkdir(parents=True, exist_ok=True)
    layer(round(108 * k), 0.74).save(d / "ic_launcher_foreground.png")  # adaptive: 108dp, motif in the 72dp safe zone
    legacy = layer(round(48 * k), 1.0)
    legacy.save(d / "ic_launcher.png")
    circle(legacy).save(d / "ic_launcher_round.png")

(res / "mipmap-anydpi-v26").mkdir(exist_ok=True)
(res / "mipmap-anydpi-v26" / "ic_launcher.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
    '  <background android:drawable="@color/ic_launcher_background"/>\n'
    '  <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n</adaptive-icon>\n')
(res / "values").mkdir(exist_ok=True)
(res / "values" / "ic_launcher_background.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n<resources>\n'
    f'  <color name="ic_launcher_background">#{bg[0]:02X}{bg[1]:02X}{bg[2]:02X}</color>\n</resources>\n')

# Preview: the adaptive icon under a circle and a rounded-square mask, plus the legacy icon.
fg = Image.open(res / "mipmap-xxxhdpi" / "ic_launcher_foreground.png").convert("RGB")
sheet = Image.new("RGB", (3 * 300, 300), (90, 90, 90))
sheet.paste(circle(fg).resize((260, 260)), (20, 20), circle(fg).resize((260, 260)))
sq = Image.new("L", fg.size, 0); ImageDraw.Draw(sq).rounded_rectangle((0, 0, fg.width - 1, fg.height - 1), radius=fg.width // 4, fill=255)
sheet.paste(fg.resize((260, 260)), (320, 20), sq.resize((260, 260)))
sheet.paste(Image.open(res / "mipmap-xxxhdpi" / "ic_launcher.png").resize((260, 260)), (620, 20))
sheet.save(preview)
print("background", bg)
