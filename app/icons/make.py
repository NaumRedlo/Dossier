import pathlib

from PIL import Image, ImageDraw, ImageFont

HERE = pathlib.Path(__file__).resolve().parent
UI = HERE.parent / "ui"
FONT = pathlib.Path.home() / "Documents/1984/assets/fonts/ProximaSoft-Bold.ttf"

ACCENT = (226, 72, 72)
ACCENT_DEEP = (201, 52, 47)
RIM = (120, 28, 26)
DARK = (14, 12, 16)

LETTER_SHARE = 0.46
SLOTS = 4

def _rounded(size: int, share: float = 0.225) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=round(size * share), fill=255
    )
    return mask

def _ramp(size: int) -> Image.Image:
    column = Image.new("RGB", (1, size))
    for y in range(size):
        share = y / max(1, size - 1)
        column.putpixel(
            (0, y),
            tuple(round(a + (b - a) * share) for a, b in zip(ACCENT, ACCENT_DEEP)),
        )
    return column.resize((size, size))

def _letter(size: int) -> tuple[Image.Image, tuple[float, float, float, float]]:
    font = ImageFont.truetype(str(FONT), round(size * LETTER_SHARE))
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    box = draw.textbbox((0, 0), "D", font=font)
    x = (size - (box[2] - box[0])) / 2 - box[0]
    y = (size - (box[3] - box[1])) / 2 - box[1]
    draw.text((x, y), "D", font=font, fill=255)
    return mask, (x + box[0], y + box[1], x + box[2], y + box[3])

def glyph(size: int) -> Image.Image:
    art = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask, (x0, y0, x1, y1) = _letter(size)
    art.paste(_ramp(size), (0, 0), mask)

    height = size * 0.028
    step = height * 2
    margin = (x1 - x0) * 0.05
    top = (y0 + y1) / 2 - (SLOTS * step - height) / 2
    slots = Image.new("L", (size, size), 0)
    cut = ImageDraw.Draw(slots)
    for slot in range(SLOTS):
        y = top + slot * step
        cut.rectangle([x0 - margin, y, x1 + margin, y + height], fill=255)
    alpha = art.getchannel("A")
    alpha.paste(0, (0, 0), slots)
    art.putalpha(alpha)
    return art

def tile(size: int) -> Image.Image:
    art = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    art.paste(_ramp(size), (0, 0), _rounded(size))

    rim = Image.new("L", (size, size), 0)
    ImageDraw.Draw(rim).rounded_rectangle(
        [0, 0, size - 1, size - 1],
        radius=round(size * 0.225),
        outline=255,
        width=max(1, round(size * 0.014)),
    )
    art.paste(Image.new("RGBA", (size, size), (*RIM, 255)), (0, 0), rim)

    draw = ImageDraw.Draw(art)
    pad = int(size * 0.075)
    draw.ellipse([pad, pad, size - pad, size - pad], outline=(*DARK, 70), width=int(size * 0.020))
    pad = int(size * 0.165)
    draw.ellipse([pad, pad, size - pad, size - pad], outline=(*DARK, 190), width=int(size * 0.046))

    mask, (x0, y0, x1, y1) = _letter(size)
    art.paste(Image.new("RGBA", (size, size), (*DARK, 255)), (0, 0), mask)

    height = size * 0.028
    step = height * 2
    margin = (x1 - x0) * 0.05
    top = (y0 + y1) / 2 - (SLOTS * step - height) / 2
    cut = ImageDraw.Draw(mask := Image.new("L", (size, size), 0))
    for slot in range(SLOTS):
        y = top + slot * step
        cut.rectangle([x0 - margin, y, x1 + margin, y + height], fill=255)
    art.paste(_ramp(size), (0, 0), mask)
    return art

def main() -> None:
    if not FONT.is_file():
        raise SystemExit(f"no font at {FONT}")
    tile(1024).save(HERE / "icon.png")
    for size in (32, 128, 256, 512):

        tile(size).save(HERE / f"{size}x{size}.png")
    tile(256).save(HERE / "icon.ico")

    tile(128).save(UI / "mark.png")
    glyph(512).save(UI / "letter.png")
    print(
        "нарисовано:",
        ", ".join(sorted(p.name for p in HERE.glob("*.png"))),
        "+ ui/mark.png, ui/letter.png",
    )

if __name__ == "__main__":
    main()
