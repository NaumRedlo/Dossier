import json
from pathlib import Path

from PIL import Image

HERE = Path(__file__).parent
GOLDEN = HERE.parent.parent.parent / "native" / "tests" / "golden"
FRAMES = HERE / "frames"

LANGS = [("en-US", "page-en", "First run · EN"), ("ru-RU", "page-ru", "First run · RU")]

STATES = [
    ("language", "1 · Language",
     "The system's language is preselected; the other one retypes every word on the screen — erased from the right, typed from the left, 640 ms."),
    ("folder-stable", "2 · osu! folder · stable found",
     "Found the way the game finds it: osu!.<user>.cfg, then BeatmapDirectory; Songs, Skins and Replays counted separately."),
    ("folder-lazer", "2 · osu! folder · lazer found",
     "lazer: storage.ini gives the root, client.realm and the content-addressed files store are read; replays counted from the store."),
    ("folder-both", "2 · osu! folder · both",
     "Both clients are read by default and appear as two sources with a switch each; the same list is Settings → Folders later."),
    ("folder-missing", "2 · osu! folder · nothing found",
     "Nothing found: Browse for the folder, or keep everything in the application's own folder — ~/.dossier with Songs, Skins, Replays."),
    ("folder-own", "2 · osu! folder · own folder",
     "The application's own folder as the only source, tagged dossier. Replays dropped on the window land here."),
    ("device", "3 · This device",
     "One field, the device's name as the bot will show it. Nothing else is asked about the machine."),
    ("bot-waiting", "4 · The bot · waiting",
     "Pairing: the QR opens the bot with the code; Open Telegram does the same on this machine. The code is shown to be recognised in the bot, never typed. The dot breathes while waiting; the application polls every 3 s."),
    ("bot-linked", "4 · The bot · linked",
     "Linked: the tick draws in, the bot's answer names the account. Later skips the step for a device that only watches its own replays."),
    ("checks-running", "5 · Checks · running",
     "The ledger runs on its own: osu! folder, ffmpeg, engine, bot. The current line's dot breathes; a settled line ticks with its detail."),
    ("checks-ffmpeg-missing", "5 · Checks · ffmpeg missing",
     "A failed line turns red with its reason and nothing under it but the fix: Download, where the explanation used to be. Check again and Continue anyway stay in the card's row."),
    ("checks-ffmpeg-downloading", "5 · Checks · ffmpeg downloading",
     "The line itself is the progress — downloading · 12.4 / 27.5 MB · martin-riedl.de — then unpacking; then the check runs again and the line ticks with the version."),
    ("checks-bot-skipped", "5 · Checks · bot skipped",
     "A bot skipped with Later is not a failure: its line stays quiet, not linked, and counts as done. The engine is still checked against the bot's build."),
    ("done", "5 · Everything works",
     "Four of four: the headline settles into a tick and one button remains, Open Dossier. From here the crest glides to the corner and the main screen enters."),
]

WIDE = [("folder-stable", "1280", (1280, 800)), ("bot-waiting", "1280", (1280, 800)),
        ("folder-stable", "1920", (1920, 1080)), ("bot-waiting", "1920", (1920, 1080))]

def frame(state, lang, size, box):
    src = GOLDEN / f"first-run-{state}-{lang}-{size}-wgpu.png"
    out = FRAMES / f"{state}-{lang}-{size}.jpg"
    image = Image.open(src).convert("RGB")
    if image.size != box:
        image = image.resize(box, Image.LANCZOS)
    image.save(out, quality=84, optimize=True, progressive=True)
    return out.name

def board(image, w, h):
    return f'''<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
  <style>
    body {{ margin: 0; background: #070304; }}
    img {{ display: block; width: {w}px; height: {h}px; }}
  </style>
</helmet>
<img src="{image}" alt="">
</x-dc>
</body>
</html>
'''

FRAMES.mkdir(exist_ok=True)
for old in HERE.glob("*.dc.html"):
    old.unlink()

W, H, GX, GY = 980, 720, 80, 200
PER_ROW = 5
artboards, annotations, images = [], [], []
for lang, page, _ in LANGS:
    for i, (state, title, note) in enumerate(STATES):
        image = frame(state, lang, "980", (W, H))
        images.append(image)
        name = "Main" if (state, lang) == ("language", "en-US") else f"{state}-{lang}"
        (HERE / f"{name}.dc.html").write_text(board(image, W, H))
        x, y = (i % PER_ROW) * (W + GX), (i // PER_ROW) * (H + GY)
        artboards.append({"file": f"{name}.dc.html", "x": x, "y": y, "w": W, "h": H, "page": page, "title": title})
        annotations.append({"id": f"{name}-note", "x": x, "y": y - 150, "w": 620, "page": page, "text": note})
    rows = (len(STATES) + PER_ROW - 1) // PER_ROW
    x = 0
    y = rows * (H + GY)
    for state, size, box in WIDE:
        image = frame(state, lang, size, box)
        images.append(image)
        name = f"{state}-{lang}-{size}"
        (HERE / f"{name}.dc.html").write_text(board(image, *box))
        artboards.append({"file": f"{name}.dc.html", "x": x, "y": y, "w": box[0], "h": box[1], "page": page, "title": f"{size} wide · {state}"})
        x += box[0] + GX
        if size == "1280" and state == "bot-waiting":
            x = 0
            y += 800 + GY
    annotations.append({"id": f"wide-{lang}", "x": 0, "y": rows * (H + GY) - 150, "w": 620, "page": page,
                        "text": "The same column at 1280×800 and 1920×1080: it stays 560 px wide and centred; only the ground around it grows."})

annotations.append({"id": "order", "x": 0, "y": -420, "w": 760, "page": "page-en",
                    "text": "These are the application's own frames, not drawings: native/tests/golden — the pictures every build is held to by the golden test. Order: Language → osu! folder → This device → The bot → Checks. Window 980×720, column 560, tokens from docs/design.md.\nTo refresh: cargo run --release -- --gallery <dir>, approve into tests/golden, then python make.py here and seed."})

canvas = {
    "pages": [{"id": page, "name": name} for _, page, name in LANGS],
    "artboards": artboards,
    "annotations": annotations,
    "launch": {"view": "canvas", "page": "page-en"},
}
(HERE / "canvas.json").write_text(json.dumps(canvas, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(artboards), "artboards,", len(set(images)), "frames")
