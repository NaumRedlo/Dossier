"""Draw the application's icon.

Kept as the thing that *makes* the picture rather than only the picture: an
icon nobody can regenerate is one that cannot be adjusted, and the colours in
it belong to the bot — `services/image/colors.py` in the 1984 repository — so
they have to be able to follow it if it ever moves.

    python3 app/icons/make.py

A `D` knocked out of the accent, on a rounded tile. Knocked out rather than
drawn in red on dark: at thirty-two pixels in a dock, a light shape on a solid
ground reads and a red shape on near-black does not.
"""

import pathlib
from PIL import Image, ImageDraw, ImageFont

HERE = pathlib.Path(__file__).resolve().parent
FONT = pathlib.Path.home() / "Documents/1984/assets/fonts/ProximaSoft-Bold.ttf"

# The bot's own two accents: the light theme's and the dark theme's, top to
# bottom. Not a decorative gradient — it is the same red twice.
TOP = (226, 72, 72)      # ACCENT
BOTTOM = (201, 52, 47)   # the guide's light-theme accent
GROUND = (14, 12, 16)    # BG


def tile(size: int) -> Image.Image:
    art = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    # The gradient, drawn full and then masked to a rounded square.
    ramp = Image.new("RGB", (1, size))
    for y in range(size):
        share = y / max(1, size - 1)
        ramp.putpixel(
            (0, y),
            tuple(round(a + (b - a) * share) for a, b in zip(TOP, BOTTOM)),
        )
    ramp = ramp.resize((size, size))

    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=round(size * 0.225), fill=255
    )
    art.paste(ramp, (0, 0), mask)

    # The letter, cut out of the tile so the ground shows through.
    letter = Image.new("L", (size, size), 0)
    font = ImageFont.truetype(str(FONT), round(size * 0.62))
    draw = ImageDraw.Draw(letter)
    box = draw.textbbox((0, 0), "D", font=font)
    draw.text(
        ((size - (box[2] - box[0])) / 2 - box[0],
         (size - (box[3] - box[1])) / 2 - box[1]),
        "D",
        font=font,
        fill=255,
    )
    cut = Image.new("RGBA", (size, size), (*GROUND, 255))
    art.paste(cut, (0, 0), letter)
    return art


def main() -> None:
    if not FONT.is_file():
        raise SystemExit(f"no font at {FONT}")
    full = tile(1024)
    full.save(HERE / "icon.png")
    for size in (32, 128, 256, 512):
        # Drawn at its own size rather than scaled down from the big one: a
        # letter this weight closes up when it is resampled, and thirty-two
        # pixels is where an icon is actually looked at.
        tile(size).save(HERE / f"{size}x{size}.png")
    tile(256).save(HERE / "icon.ico")
    print("нарисовано:", ", ".join(p.name for p in sorted(HERE.glob("*.png"))))


if __name__ == "__main__":
    main()
