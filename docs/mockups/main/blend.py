import json
from pathlib import Path

from make import LETTER, SEARCH, CARDS, page
from directions import EXTRA
from more import MORE
from more2 import MORE2

HERE = Path(__file__).parent

BLEND = """
    .scene2 { position: absolute; inset: 0; background: #0a0507 url(frame.jpg) center/cover; }
    .scene2::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(7,3,4,0.78) 0%, rgba(7,3,4,0.12) 26%, rgba(7,3,4,0.12) 52%, rgba(7,3,4,0.88) 78%, rgba(7,3,4,0.97) 100%); }
    .id { position: absolute; left: 40px; right: 40px; top: 360px; z-index: 2; display: flex; align-items: flex-end; gap: 24px; }
    .id .date { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; color: #a9a29b; letter-spacing: 0.04em; }
    .id h2 { margin: 2px 0 0; font-size: 28px; line-height: 32px; font-weight: 600; letter-spacing: -0.01em; }
    .id .map { margin-top: 2px; color: #ece7e2; font-size: 14px; }
    .id .meta { margin-top: 6px; display: flex; align-items: center; gap: 8px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .id .acts { display: flex; gap: 8px; margin-top: 14px; }
    .id .score { margin-left: auto; text-align: right; }
    .id .score .n { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 52px; line-height: 52px; font-weight: 700; letter-spacing: -0.03em; font-variant-numeric: tabular-nums; }
    .id .score .o { display: inline-flex; align-items: center; gap: 6px; margin-top: 8px; padding: 2px 8px; border-radius: 6px; background: rgba(10,5,7,0.6); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; font-weight: 700; color: #ece7e2; }
    .journal { position: absolute; left: 0; right: 0; bottom: 56px; z-index: 2; padding: 0 40px; }
    .journal .rail { position: relative; height: 1px; background: rgba(255,255,255,0.12); margin: 0 0 10px; }
    .journal .days { display: flex; gap: 22px; align-items: flex-end; overflow: hidden; }
    .day2 { display: flex; flex-direction: column; gap: 6px; flex: none; }
    .day2 .lbl { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; color: #6b655f; letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; }
    .day2 .lbl.today { color: #ece7e2; }
    .day2 .lbl i { display: block; width: 5px; height: 5px; border-radius: 50%; background: #6b655f; }
    .day2 .lbl.today i { background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.16); }
    .day2 .set { display: flex; gap: 6px; align-items: flex-end; }
    .fr { width: 88px; height: 50px; border-radius: 6px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.1); opacity: 0.7; position: relative; }
    .fr.on { opacity: 1; border: 2px solid #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.18); width: 96px; height: 54px; }
    .fr.none { background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.04) 0 4px, transparent 4px 9px); }
    .fr .a { position: absolute; right: 4px; bottom: 3px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; font-weight: 700; color: #ece7e2; text-shadow: 0 1px 2px rgba(0,0,0,0.6); }
    .bot { display: flex; align-items: center; gap: 6px; height: 50px; padding: 0 10px; border-radius: 6px; border: 1px dashed rgba(255,255,255,0.14); color: #a9a29b; font-size: 11px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; white-space: nowrap; }
    .bot i { display: block; width: 6px; height: 6px; border-radius: 50%; background: #e24848; }
    .bar2 { position: absolute; left: 40px; right: 40px; bottom: 20px; z-index: 2; display: flex; align-items: center; gap: 8px; color: #a9a29b; font-size: 13px; }
    .bar2 .dot { width: 8px; height: 8px; }
    .bar2 b { color: #ece7e2; font-weight: 600; }
    .bar2 .r { margin-left: auto; display: flex; gap: 16px; align-items: center; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .veil3 { position: absolute; left: 0; right: 0; top: 0; z-index: 3; display: flex; align-items: center; gap: 14px; padding: 22px 40px; }
"""

def with_blend(html):
    return html.replace("  </style>", EXTRA + MORE + MORE2 + BLEND + "  </style>", 1)

def frames(items, on=-1):
    out = ""
    for i, it in enumerate(items):
        if it == "bot":
            out += '<span class="bot"><i></i>for @tosha · 3:40</span>'
        elif it == "none":
            out += f'<span class="fr none{" on" if i == on else ""}"></span>'
        else:
            out += f'<span class="fr{" on" if i == on else ""}"><span class="a">{it}</span></span>'
    return out

def blend(lang):
    en = lang == "en"
    words = ("Replays", "Worker", "Settings") if en else ("Реплеи", "Воркер", "Настройки")
    days = [
        ("today · Sep 16" if en else "сегодня · 16 сен", True, ["98,71%"]),
        ("Aug 14" if en else "14 авг", False, ["99,65%", "93,03%"]),
        ("Aug 3" if en else "3 авг", False, ["none"]),
        ("Jul 29" if en else "29 июл", False, ["72,13%", "96,40%", "97,88%"]),
        ("May 10" if en else "10 мая", False, ["none", "88,12%"]),
    ]
    strip = ""
    for i, (label, is_today, items) in enumerate(days):
        strip += f'<div class="day2"><span class="lbl{" today" if is_today else ""}"><i></i>{label}</span><span class="set">{frames(items, on=0 if i == 0 else -1)}</span></div>'
    render = "Render" if en else "Отрендерить"
    worker = "Worker" if en else "Воркер"
    date = "Today · 14:02 · lazer" if en else "Сегодня · 14:02 · lazer"
    body = f'''
  <div class="scene2"></div>
  <div class="veil3">
    <div class="brand" style="padding: 0;">{LETTER}<i></i><b>Dossier</b></div>
    <div class="field" style="margin-left: 26px; width: 240px; background: rgba(10,5,7,0.55);">{SEARCH}<span>{"Player, map, day" if en else "Игрок, карта, день"}</span></div>
    <div class="words"><b>{words[0]}</b><span>{words[1]}</span><span>{words[2]}</span></div>
  </div>
  <div class="id">
    <div>
      <div class="date">{date}</div>
      <h2>NaumRedlo</h2>
      <div class="map">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</div>
      <div class="meta"><span class="mods"><span class="mod h">HD</span><span class="mod h">DT</span><span class="mod h">HR</span></span><span>604x</span><span>·</span><span>3:51</span></div>
      <div class="acts"><span class="btn primary">{render}</span></div>
    </div>
    <div class="score"><div class="n">98,71%</div><span class="o">FC</span></div>
  </div>
  <div class="journal"><div class="rail"></div><div class="days">{strip}</div></div>
  <div class="bar2"><span class="dot"></span><span>{worker}</span><span class="r"><span>1 / 187</span><span>‹ ›</span></span></div>'''
    return with_blend(page(body))

boards = {"Blend": blend("en"), "BlendRu": blend("ru")}
for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

manifest = json.loads((HERE / "canvas.json").read_text())
W, H, GX = 980, 720, 80
if not any(p["id"] == "page-blend" for p in manifest["pages"]):
    manifest["pages"].insert(0, {"id": "page-blend", "name": "Blend · scene + viewer + journal"})
manifest["artboards"] = [a for a in manifest["artboards"] if a["file"] not in ("Blend.dc.html", "BlendRu.dc.html")] + [
    {"file": "Blend.dc.html", "x": 0, "y": 0, "w": W, "h": H, "page": "page-blend", "title": "Scene · viewer · journal"},
    {"file": "BlendRu.dc.html", "x": W + GX, "y": 0, "w": W, "h": H, "page": "page-blend", "title": "Сцена · просмотр · журнал"},
]
manifest["annotations"] = [a for a in manifest["annotations"] if a["id"] != "blend"] + [
    {"id": "blend", "x": 0, "y": -210, "w": 560, "page": "page-blend",
     "text": "The scene is the window: the chosen play fills it, dimmed at the top and the bottom. The viewer is the lower third: date and client, the player, the map, mods and combo, one button, the accuracy large with its outcome. The journal is the strip along the bottom: replays only, grouped by day in the language's own date style, newest first, the chosen one outlined. Bottom-left: a dot and the word Worker — the dot is the state, the word is the way there. The brand block is the application's own: 36 px letter, a 22 px rule, the word at 20 px."},
]
manifest["launch"] = {"view": "canvas", "page": "page-blend"}
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards))
