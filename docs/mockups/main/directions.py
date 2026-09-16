import json
from pathlib import Path

from make import STYLE, LETTER, TICK, DOT, NONE, SEARCH, CARDS, page

HERE = Path(__file__).parent

EXTRA = """
    .top { display: flex; align-items: center; gap: 14px; padding: 0 0 0; }
    .top .brand { padding: 0; }
    .words { display: flex; gap: 22px; margin-left: auto; font-weight: 500; color: #a9a29b; }
    .words b { color: #ece7e2; font-weight: 600; }
    .col { position: absolute; left: 50%; top: 56px; width: 640px; margin-left: -320px; }
    .headline { display: flex; align-items: center; justify-content: center; gap: 8px; margin-top: 30px; font-size: 16px; line-height: 22px; font-weight: 600; }
    .headline .n { color: #a9a29b; font-weight: 400; }
    .menu { display: flex; justify-content: center; gap: 28px; margin-top: 8px; color: #a9a29b; font-weight: 500; }
    .menu b { color: #ece7e2; font-weight: 600; border-bottom: 2px solid #e24848; padding-bottom: 2px; }
    .rows { margin-top: 22px; border-top: 1px solid rgba(255,255,255,0.08); }
    .rw { display: grid; grid-template-columns: 56px minmax(0, 1fr) 70px 90px 60px; align-items: center; gap: 14px; height: 44px; border-bottom: 1px solid rgba(255,255,255,0.08); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 13px; }
    .rw .th { width: 56px; height: 32px; border-radius: 5px; background: #0a0507 url(frame.jpg) center/cover; }
    .rw .th.none { background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.03) 0 4px, transparent 4px 9px); }
    .rw .p { font-family: Commissioner, sans-serif; font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .rw .p span { display: block; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 400; font-size: 11px; color: #a9a29b; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .rw .a { text-align: right; font-weight: 700; font-variant-numeric: tabular-nums; }
    .rw .m { color: #a9a29b; font-size: 11px; }
    .rw .w { text-align: right; color: #6b655f; font-size: 11px; }
    .rw.lit { background: rgba(255,255,255,0.045); margin: 0 -12px; padding: 0 12px; }
    .foot { display: flex; align-items: center; gap: 8px; margin-top: 18px; color: #a9a29b; font-size: 12px; }
    .film { position: absolute; left: 0; right: 0; bottom: 88px; display: flex; gap: 10px; padding: 0 40px; overflow: hidden; }
    .film .f { flex: none; width: 128px; height: 72px; border-radius: 8px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.08); opacity: 0.75; }
    .film .f.on { opacity: 1; border: 2px solid #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); }
    .film .f.none { background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.03) 0 5px, transparent 5px 11px); }
    .big { position: absolute; left: 40px; top: 84px; width: 560px; height: 315px; border-radius: 12px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.08); }
    .beside { position: absolute; left: 632px; top: 84px; right: 40px; }
    .beside h2 { margin: 0; font-size: 22px; line-height: 28px; font-weight: 600; letter-spacing: -0.01em; }
    .beside .sub { margin-top: 4px; color: #a9a29b; }
    .beside .acc { margin-top: 22px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 40px; line-height: 44px; font-weight: 700; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
    .beside .k { color: #a9a29b; font-size: 12px; }
    .beside .row { display: flex; gap: 8px; margin-top: 22px; }
    .cmd { display: flex; align-items: center; gap: 12px; height: 44px; padding: 0 16px; border-radius: 10px; border: 1px solid rgba(255,255,255,0.16); background: rgba(0,0,0,0.26); color: #6b655f; font-size: 15px; }
    .cmd svg { width: 16px; height: 16px; stroke: #a9a29b; fill: none; stroke-width: 2; stroke-linecap: round; }
    .cmd kbd { margin-left: auto; padding: 1px 6px; border-radius: 4px; border: 1px solid rgba(255,255,255,0.16); color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .hint { display: flex; gap: 18px; margin-top: 10px; color: #6b655f; font-size: 12px; }
    .hint b { color: #a9a29b; font-weight: 500; }
    .bar { position: absolute; left: 40px; right: 40px; bottom: 24px; display: flex; align-items: center; gap: 8px; color: #a9a29b; font-size: 13px; }
    .bar .dot { width: 8px; height: 8px; }
    .bar b { color: #ece7e2; font-weight: 600; }
    .bar .r { margin-left: auto; display: flex; gap: 18px; }
"""

def with_extra(html):
    return html.replace("  </style>", EXTRA + "  </style>", 1)

def rows(n=6, hover=1):
    out = ""
    for i, c in enumerate(CARDS[:n]):
        client, outcome, good, player, mp, acc, mods, combo, when = c
        th = "th none" if client is None else "th"
        m = " ".join(x for _, x in mods)
        out += (f'<div class="rw{" lit" if i == hover else ""}"><span class="{th}"></span>'
                f'<span class="p">{player}<span>{mp}</span></span><span class="a">{acc}</span><span class="m">{m} · {combo}</span><span class="w">{when}</span></div>')
    return out

def direction_a():
    body = f'''
  <div class="col">
    <div class="brand" style="justify-content: center;">{LETTER}<i></i><b>Dossier</b></div>
    <div class="headline"><span class="dot"></span>Ready to take work <span class="n">· 187 replays</span></div>
    <div class="menu"><b>Replays</b><span>Farm</span><span>Settings</span></div>
    <div class="tools" style="margin-top: 22px;"><div class="field" style="flex: 1;">{SEARCH}<span>Player, map, file</span></div><div class="chips"><span class="chip on">All <span class="n">187</span></span><span class="chip">With a map <span class="n">185</span></span><span class="chip">Without <span class="n">2</span></span></div></div>
    <div class="rows">{rows()}</div>
    <div class="foot"><span>Newest first · 6 of 187 shown · scroll for more</span></div>
  </div>'''
    return with_extra(page(body))

def direction_b():
    film = "".join(f'<span class="f{" on" if i == 0 else ""}{" none" if c[0] is None else ""}"></span>' for i, c in enumerate(CARDS * 2))
    body = f'''
  <div class="top" style="position: absolute; left: 40px; right: 40px; top: 24px;">
    <div class="brand">{LETTER}<i></i><b>Dossier</b></div>
    <div class="words"><b>Replays</b><span>Farm</span><span>Settings</span></div>
  </div>
  <div class="big"><span class="tag l">lazer</span><span class="tag r good">FC</span></div>
  <div class="beside">
    <h2>NaumRedlo</h2>
    <div class="sub">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</div>
    <div class="acc">98,71%</div>
    <div class="k">604x · HD DT HR · 14 авг</div>
    <div class="row"><span class="btn primary">Render</span><span class="btn quiet">Judge</span></div>
  </div>
  <div class="film">{film}</div>
  <div class="bar"><span class="dot"></span><b>Ready to take work</b><span>· 341 frames/s · 3 waiting</span><span class="r"><span>1 of 187</span><span>{SEARCH.replace('viewBox', 'style="width:14px;height:14px;stroke:#a9a29b;fill:none;stroke-width:2" viewBox')}</span></span></div>'''
    return with_extra(page(body))

def direction_d():
    body = f'''
  <div class="col" style="top: 48px;">
    <div class="brand" style="justify-content: center;">{LETTER}<i></i><b>Dossier</b></div>
    <div class="cmd" style="margin-top: 26px;">{SEARCH.replace('viewBox', 'viewBox')}<span>Search a replay, or type farm, settings, render…</span><kbd>⌘K</kbd></div>
    <div class="hint"><b>Enter</b> render <span>·</span> <b>Space</b> judge <span>·</span> <b>↑↓</b> move <span>·</span> <b>Esc</b> clear</div>
    <div class="now" style="margin-top: 22px; padding: 14px 18px;"><div style="flex: 1;"><div class="head"><span class="dot"></span>Ready to take work <span>· 341 frames/s · 3 waiting at the bot</span></div></div></div>
    <div class="rows" style="margin-top: 18px; border-top: 0;">{rows(hover=0)}</div>
  </div>'''
    return with_extra(page(body))

boards = {
    "DirectionA": direction_a(),
    "DirectionB": direction_b(),
    "DirectionD": direction_d(),
}
for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

manifest = json.loads((HERE / "canvas.json").read_text())
W, H, GX = 980, 720, 80
manifest["pages"] = [{"id": "page-directions", "name": "Directions · pick one"}] + [p for p in manifest["pages"] if p["id"] != "page-directions"]
manifest["artboards"] = [a for a in manifest["artboards"] if not a["file"].startswith("Direction")] + [
    {"file": "DirectionA.dc.html", "x": 0, "y": 0, "w": W, "h": H, "page": "page-directions", "title": "A · The document"},
    {"file": "DirectionB.dc.html", "x": W + GX, "y": 0, "w": W, "h": H, "page": "page-directions", "title": "B · The viewer"},
    {"file": "DirectionD.dc.html", "x": 2 * (W + GX), "y": 0, "w": W, "h": H, "page": "page-directions", "title": "C · The command"},
]
manifest["annotations"] = [a for a in manifest["annotations"] if not a["id"].startswith("dir-")] + [
    {"id": "dir-a", "x": 0, "y": -190, "w": 460, "page": "page-directions",
     "text": "A · The document. No sidebar: the first run's column carries on. The emblem, one state line, three words as the menu, and replays as lines in a ledger — a dossier is a file of papers. Calm, typographic, the frames small stamps. Tradeoff: the frames the engine draws are not the hero."},
    {"id": "dir-b", "x": W + GX, "y": -190, "w": 460, "page": "page-directions",
     "text": "B · The viewer. One replay at a time, its frame large, the rest a filmstrip below; menu is three words in the corner, state in one bar at the bottom. The engine's picture is the hero. Tradeoff: finding one of 187 means scrolling the strip or searching."},
    {"id": "dir-d", "x": 2 * (W + GX), "y": -190, "w": 460, "page": "page-directions",
     "text": "C · The command. A single field is the menu and the search: type a player, a map, or farm / settings / render. Results as lines, the state as a ledger card, keys spelled out. Fastest for a keyboard; least discoverable for a newcomer."},
]
manifest["launch"] = {"view": "canvas", "page": "page-directions"}
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards), "directions")
