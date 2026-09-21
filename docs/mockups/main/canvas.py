import json
from pathlib import Path

import built
import centre
import errors
import island

HERE = Path(__file__).parent
W, H, GX, GY = 980, 720, 80, 190

def lay(names, notes, page, recommended=None, extra=None):
    artboards, annotations = [], []
    for i, name in enumerate(names):
        x, y = (i % 3) * (W + GX), (i // 3) * (H + GY)
        title, note = notes[name]
        artboards.append({"file": f"{name}.dc.html", "x": x, "y": y, "w": W, "h": H, "page": page, "title": title})
        annotations.append({"id": f"{name}-note", "x": x, "y": y - 140, "w": 640, "page": page, "text": note})
    rows = (len(names) + 2) // 3
    if recommended:
        annotations.append({"id": f"{page}-rec", "x": 0, "y": rows * (H + GY) - 60, "w": 980, "page": page, "text": recommended})
    if extra:
        annotations.append({"id": f"{page}-extra", "x": W + GX, "y": rows * (H + GY) - 60, "w": 980, "page": page, "text": extra})
    return artboards, annotations

for old in HERE.glob("Blend*.dc.html"):
    old.unlink()
for old in ["Main", "MainRu", "Empty", "Rendering", "RenderingRu"]:
    path = HERE / f"{old}.dc.html"
    if path.exists():
        path.unlink()

built_boards, built_notes = built.boards()
for name, html in built_boards.items():
    (HERE / f"{name}.dc.html").write_text(html)
error_boards = errors.boards()
for name, html in error_boards.items():
    (HERE / f"{name}.dc.html").write_text(html)
centre_boards = centre.boards()
for name, html in centre_boards.items():
    (HERE / f"{name}.dc.html").write_text(html)
island_boards = island.boards()
for name, html in island_boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

artboards = [a for a, _ in built_notes]
annotations = [n for _, n in built_notes if n]
annotations.append({"id": "built-head", "x": 0, "y": -330, "w": 900, "page": "page-built",
                    "text": "The main screen as built, 2026-09-21 — the application's own frames from native/tests/golden, the pictures every build is held to. Refresh: cargo run --release -- --gallery <dir>, approve into tests/golden, python canvas.py, seed. The live play cannot be staged, so the scene here is its blurred backdrop."})

names = ["ErrorInline", "ErrorButton", "ErrorToast", "ErrorCentre", "ErrorFatal", "ErrorOffline"]
a, n = lay(names, errors.NOTES, "page-errors", errors.RECOMMENDED)
artboards += a
annotations += n

names = ["IslandQuiet", "IslandOne", "IslandLine", "IslandTwo", "IslandDone", "IslandBad", "IslandWorker", "IslandOffline", "IslandOpen", "IslandMorph"]
a, n = lay(names, island.NOTES, "page-island", island.RECOMMENDED, centre.TO_BUILD)
artboards += a
annotations += n

names = ["CentreMark", "CentrePanel", "CentreDrawer", "CentreStripe", "CentreTelegram"]
a, n = lay(names, centre.NOTES, "page-centre", centre.RECOMMENDED)
artboards += a
annotations += n

old = json.loads((HERE / "canvas.json").read_text())
keep_pages = [p for p in old["pages"] if p["id"] == "page-directions"]
artboards += [b for b in old["artboards"] if b["page"] == "page-directions"]
annotations += [b for b in old["annotations"] if b["page"] == "page-directions"]

manifest = {
    "pages": [
        {"id": "page-built", "name": "Main screen · as built"},
        {"id": "page-island", "name": "Остров · состояния"},
        {"id": "page-errors", "name": "Errors · variations"},
        {"id": "page-centre", "name": "Operations centre · first variations"},
    ] + keep_pages,
    "artboards": artboards,
    "annotations": annotations,
    "launch": {"view": "canvas", "page": "page-built"},
}
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(artboards), "artboards on", len(manifest["pages"]), "pages")
