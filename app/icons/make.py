"""Draw the application's icon, and the mark the window wears in its corner.

Kept as the thing that *makes* the picture rather than only the picture: an
icon nobody can regenerate cannot be adjusted, and the colours in it belong to
the bot — `services/image/colors.py` in the 1984 repository — so they have to
be able to follow it if it ever moves.

    python3 app/icons/make.py

The mark is a `D` knocked out of the accent, inside the two rings osu! draws
around a note as it approaches, with a strip of film cut back through the
letter. Three decisions in it are worth writing down:

* **The tile carries the accent and the letter is the hole.** The other way
  round — a red letter on near-black — disappears at thirty-two pixels, which
  is the size an icon is actually looked at.
* **The slots are measured from the letter, not from the tile.** A `D` does not
  sit centred in its own box (the bowl is right of the stem), so slots placed
  by the tile's midline land visibly off. The first version did exactly that.
* **There is a dark rim.** Without one the tile dissolves at its edges against
  a white background; the dock is dark and the file manager is not.
"""

import pathlib

from PIL import Image, ImageDraw, ImageFont

HERE = pathlib.Path(__file__).resolve().parent
UI = HERE.parent / "ui"
FONT = pathlib.Path.home() / "Documents/1984/assets/fonts/ProximaSoft-Bold.ttf"

ACCENT = (226, 72, 72)       # the bot's ACCENT, its dark theme's
ACCENT_DEEP = (201, 52, 47)  # and its light theme's — the same red twice
RIM = (120, 28, 26)
DARK = (14, 12, 16)          # the bot's BG, which the letter is cut down to

LETTER_SHARE = 0.46
SLOTS = 4


def _rounded(size: int, share: float = 0.225) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=round(size * share), fill=255
    )
    return mask


def _ramp(size: int) -> Image.Image:
    """The accent, top to bottom."""
    column = Image.new("RGB", (1, size))
    for y in range(size):
        share = y / max(1, size - 1)
        column.putpixel(
            (0, y),
            tuple(round(a + (b - a) * share) for a, b in zip(ACCENT, ACCENT_DEEP)),
        )
    return column.resize((size, size))


def _letter(size: int) -> tuple[Image.Image, tuple[float, float, float, float]]:
    """The letter's mask, and where it actually sits — the slots need the second."""
    font = ImageFont.truetype(str(FONT), round(size * LETTER_SHARE))
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    box = draw.textbbox((0, 0), "D", font=font)
    x = (size - (box[2] - box[0])) / 2 - box[0]
    y = (size - (box[3] - box[1])) / 2 - box[1]
    draw.text((x, y), "D", font=font, fill=255)
    return mask, (x + box[0], y + box[1], x + box[2], y + box[3])


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

    # The approach circles, in the ground colour rather than in a second red:
    # two reds at this size read as a smudge.
    draw = ImageDraw.Draw(art)
    pad = int(size * 0.075)
    draw.ellipse([pad, pad, size - pad, size - pad], outline=(*DARK, 70), width=int(size * 0.020))
    pad = int(size * 0.165)
    draw.ellipse([pad, pad, size - pad, size - pad], outline=(*DARK, 190), width=int(size * 0.046))

    mask, (x0, y0, x1, y1) = _letter(size)
    art.paste(Image.new("RGBA", (size, size), (*DARK, 255)), (0, 0), mask)

    # The film, measured from the letter: its width plus a margin, and the run
    # of them centred on the letter's own middle.
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
        # Drawn at its own size rather than scaled down from the big one: rings
        # this thin close up when they are resampled, and thirty-two pixels is
        # where an icon is actually looked at.
        tile(size).save(HERE / f"{size}x{size}.png")
    tile(256).save(HERE / "icon.ico")
    # And the same drawing for the window's own corner, so the two cannot drift.
    tile(128).save(UI / "mark.png")
    print("нарисовано:", ", ".join(sorted(p.name for p in HERE.glob("*.png"))), "+ ui/mark.png")


if __name__ == "__main__":
    main()
