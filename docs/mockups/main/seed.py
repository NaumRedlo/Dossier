import base64
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).parent

def seed(page: Path, title: str, boards: list[Path], images: list[Path], canvas: Path):
    text = page.read_text()
    opening = re.search(r'<script type="application/json" id="appifact-doc">', text)
    start = opening.end()
    end = text.index("</script>", start)
    doc = json.loads(text[start:end].strip())
    files = {}
    for board in boards:
        files[board.name] = board.read_text()
    for image in images:
        files[image.name] = base64.b64encode(image.read_bytes()).decode("ascii")
    files["canvas.json"] = canvas.read_text()
    doc["title"] = title
    doc["content"]["files"] = files
    body = json.dumps(doc, ensure_ascii=False).replace("</", "<\\/")
    page.write_text(text[:start] + "\n" + body + "\n" + text[end:])
    print("seeded", page.name, "with", len(boards), "boards,", len(images), "images")

if __name__ == "__main__":
    title = sys.argv[1]
    boards = sorted(HERE.glob("*.dc.html"))
    images = [p for p in [HERE / "frame.jpg", HERE / "letter.png"] if p.exists()] + sorted(HERE.glob("bg-*.jpg")) + sorted((HERE / "frames").glob("*.jpg"))
    seed(HERE / "dossier-main-menu.html", title, boards, images, HERE / "canvas.json")
