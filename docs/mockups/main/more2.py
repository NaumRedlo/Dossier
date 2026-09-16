import json
from pathlib import Path

from make import STYLE, LETTER, TICK, DOT, NONE, SEARCH, CARDS, page
from directions import EXTRA
from more import MORE

HERE = Path(__file__).parent

MORE2 = """
    .shelf { position: absolute; left: 40px; right: 40px; top: 84px; display: flex; flex-direction: column; gap: 14px; }
    .mapb { position: relative; height: 96px; border-radius: 12px; overflow: hidden; border: 1px solid rgba(255,255,255,0.08); background: #0a0507 url(frame.jpg) center/cover; }
    .mapb::after { content: ""; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(7,3,4,0.92) 0%, rgba(7,3,4,0.72) 45%, rgba(7,3,4,0.35) 100%); }
    .mapb .in { position: absolute; inset: 0; z-index: 1; display: flex; align-items: center; gap: 18px; padding: 0 20px; }
    .mapb .t { width: 300px; min-width: 0; }
    .mapb .t b { display: block; font-size: 15px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .mapb .t span { color: #a9a29b; font-size: 12px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .mapb .pl { display: flex; gap: 6px; flex-wrap: wrap; margin-left: auto; justify-content: flex-end; }
    .pill { display: inline-flex; align-items: center; gap: 8px; height: 28px; padding: 0 10px; border-radius: 14px; background: rgba(10,5,7,0.72); border: 1px solid rgba(255,255,255,0.08); font-size: 12px; }
    .pill b { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .pill.best { border-color: rgba(226,72,72,0.6); }
    .player { position: absolute; inset: 0; display: grid; grid-template-columns: minmax(0, 1fr) 300px; }
    .stagebox { padding: 56px 32px 72px 40px; display: flex; flex-direction: column; }
    .stagebox .frame { flex: 1; border-radius: 12px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.08); position: relative; }
    .transport { display: flex; align-items: center; gap: 16px; margin-top: 16px; }
    .transport .ctl { display: flex; gap: 10px; }
    .transport .ctl span { width: 36px; height: 36px; border-radius: 18px; display: flex; align-items: center; justify-content: center; border: 1px solid rgba(255,255,255,0.08); color: #ece7e2; }
    .transport .ctl span.go { background: #e24848; border-color: transparent; }
    .transport .ctl svg { width: 16px; height: 16px; fill: currentColor; }
    .transport .seek { flex: 1; height: 4px; border-radius: 2px; background: rgba(255,255,255,0.16); position: relative; }
    .transport .seek i { position: absolute; left: 0; top: 0; bottom: 0; width: 38%; border-radius: 2px; background: #e24848; }
    .transport .tm { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; color: #a9a29b; }
    .queue { padding: 56px 32px 24px 0; display: flex; flex-direction: column; min-width: 0; }
    .queue .np b { display: block; font-size: 18px; line-height: 24px; }
    .queue .np span { color: #a9a29b; font-size: 13px; }
    .queue .np .acc { margin-top: 10px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 30px; line-height: 34px; font-weight: 700; }
    .queue .cap { margin-top: 26px; text-transform: uppercase; letter-spacing: 0.08em; font-size: 11px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .queue .q { display: flex; align-items: center; gap: 10px; height: 40px; border-bottom: 1px solid rgba(255,255,255,0.08); font-size: 13px; }
    .queue .q .th { width: 44px; height: 26px; border-radius: 4px; background: #0a0507 url(frame.jpg) center/cover; flex: none; }
    .queue .q .t { min-width: 0; flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .queue .q .t span { color: #a9a29b; font-size: 11px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; display: block; }
    .queue .q .a { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; font-size: 12px; }
    .mosaic { position: absolute; inset: 0; display: grid; grid-template-columns: repeat(4, 1fr); grid-auto-rows: 180px; gap: 2px; background: #070304; }
    .mosaic .m { position: relative; background: #0a0507 url(frame.jpg) center/cover; }
    .mosaic .m.wide { grid-column: span 2; }
    .mosaic .m.none { background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.03) 0 6px, transparent 6px 14px); }
    .mosaic .m .lbl { position: absolute; left: 0; right: 0; bottom: 0; padding: 30px 12px 10px; background: linear-gradient(transparent, rgba(7,3,4,0.85)); display: flex; align-items: flex-end; gap: 8px; }
    .mosaic .m .lbl b { font-size: 13px; }
    .mosaic .m .lbl span { color: #a9a29b; font-size: 11px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .mosaic .m .lbl .a { margin-left: auto; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; font-size: 14px; }
    .veil { position: absolute; left: 0; right: 0; top: 0; z-index: 3; display: flex; align-items: center; gap: 14px; padding: 16px 40px; background: linear-gradient(rgba(7,3,4,0.9), rgba(7,3,4,0)); }
    .veil2 { position: absolute; left: 0; right: 0; bottom: 0; z-index: 3; display: flex; align-items: center; gap: 8px; padding: 30px 40px 16px; background: linear-gradient(rgba(7,3,4,0), rgba(7,3,4,0.9)); color: #a9a29b; font-size: 13px; }
    .veil2 b { color: #ece7e2; font-weight: 600; }
"""

def with_all(html):
    return html.replace("  </style>", EXTRA + MORE + MORE2 + "  </style>", 1)

def direction_maps():
    maps = [
        ("xi — FREEDOM DiVE", "Extra · 4:22 · 2 006 max", [("-legusshhka-", "93,03%", True), ("Guest", "88,12%", False)]),
        ("Dj Grimoire — Astral Quantization", "Nattu VN0TH3R · 3:51 · 1 214 max", [("NaumRedlo", "98,71%", True)]),
        ("xi — Blue Zenith", "Asphyxia's Hard · 4:40 · 1 802 max", [("Guest", "99,65%", True), ("NaumRedlo", "97,20%", False), ("Saki_chan", "91,00%", False)]),
        ("Chroma — sink to the deep sea world", "HP0 · 2:20 · 1 402 max", [("Saki_chan", "72,13%", True)]),
        ("Phoneboy — Nevermind", "Insane · 3:05 · 988 max", [("ItarinKamiSama", "96,40%", True)]),
    ]
    out = ""
    for title, meta, plays in maps:
        pills = "".join(f'<span class="pill{" best" if best else ""}">{p} <b>{a}</b></span>' for p, a, best in plays)
        out += f'<div class="mapb"><div class="in"><span class="t"><b>{title}</b><span>{meta}</span></span><span class="pl">{pills}</span></div></div>'
    body = f'''
  <div class="top" style="position: absolute; left: 40px; right: 40px; top: 24px;">
    <div class="brand">{LETTER}<i></i><b>Dossier</b></div>
    <div class="field" style="margin-left: 24px; width: 240px;">{SEARCH}<span>Map or player</span></div>
    <div class="words"><b>Maps</b><span>Farm</span><span>Settings</span></div>
  </div>
  <div class="shelf">{out}</div>
  <div class="bar"><span class="dot"></span><b>Ready to take work</b><span>· 341 frames/s · 3 waiting</span><span class="r"><span>42 maps · 187 replays</span></span></div>'''
    return with_all(page(body))

def direction_player():
    q = ""
    for c in CARDS[1:6]:
        client, outcome, good, player, mp, acc, mods, combo, when = c
        q += f'<div class="q"><span class="th"></span><span class="t">{player}<span>{mp}</span></span><span class="a">{acc}</span></div>'
    body = f'''
  <div class="top" style="position: absolute; left: 40px; right: 32px; top: 20px; z-index: 2;">
    <div class="brand">{LETTER}<i></i><b>Dossier</b></div>
    <div class="words"><b>Replays</b><span>Farm</span><span>Settings</span></div>
  </div>
  <div class="player">
    <div class="stagebox">
      <div class="frame"><span class="tag l">lazer</span><span class="tag r good">FC</span></div>
      <div class="transport">
        <div class="ctl"><span><svg viewBox="0 0 24 24"><path d="M6 6h2v12H6zM18 6v12L9 12z"></path></svg></span><span class="go"><svg viewBox="0 0 24 24"><path d="M8 6v12l10-6z"></path></svg></span><span><svg viewBox="0 0 24 24"><path d="M16 6h2v12h-2zM6 6v12l9-6z"></path></svg></span></div>
        <div class="seek"><i></i></div><span class="tm">1:28 / 3:51</span>
      </div>
    </div>
    <div class="queue">
      <div class="np"><b>NaumRedlo</b><span>Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</span><div class="acc">98,71%</div><span>604x · HD DT HR · 14 авг</span>
        <div style="display: flex; gap: 8px; margin-top: 16px;"><span class="btn primary">Render</span><span class="btn quiet">Judge</span></div></div>
      <div class="cap">Up next · 186</div>
      {q}
    </div>
  </div>
  <div class="bar" style="right: 340px;"><span class="dot"></span><b>Ready to take work</b><span>· 341 frames/s · 3 waiting</span></div>'''
    return with_all(page(body))

def direction_mosaic():
    tiles = ""
    layout = ["wide", "", "", "", "", "wide", "none", "", "", "wide", "", ""]
    for i, kind in enumerate(layout):
        c = CARDS[i % len(CARDS)]
        client, outcome, good, player, mp, acc, mods, combo, when = c
        none = " none" if (kind == "none" or client is None) else ""
        a = "—" if none else acc
        tiles += f'<div class="m {kind}{none}"><div class="lbl"><b>{player}</b><span>{mp if kind == "wide" else ""}</span><span class="a">{a}</span></div></div>'
    body = f'''
  <div class="mosaic">{tiles}</div>
  <div class="veil"><div class="brand" style="padding: 0;">{LETTER}<i></i><b>Dossier</b></div><div class="field" style="margin-left: 24px; width: 240px; background: rgba(0,0,0,0.5);">{SEARCH}<span>Player, map, file</span></div><div class="words"><b>Replays</b><span>Farm</span><span>Settings</span></div></div>
  <div class="veil2"><span class="dot"></span><b>Ready to take work</b><span>· 341 frames/s · 3 waiting</span><span style="margin-left: auto;">187 · newest first</span></div>'''
    return with_all(page(body))

boards = {"DirectionMaps": direction_maps(), "DirectionPlayer": direction_player(), "DirectionMosaic": direction_mosaic()}
for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

manifest = json.loads((HERE / "canvas.json").read_text())
W, H, GX, GY = 980, 720, 80, 160
names = {"DirectionMaps.dc.html": "G · By map", "DirectionPlayer.dc.html": "H · The player", "DirectionMosaic.dc.html": "I · The mosaic"}
manifest["artboards"] = [a for a in manifest["artboards"] if a["file"] not in names] + [
    {"file": f, "x": i * (W + GX), "y": 2 * (H + GY), "w": W, "h": H, "page": "page-directions", "title": t}
    for i, (f, t) in enumerate(names.items())
]
manifest["annotations"] = [a for a in manifest["annotations"] if a["id"] not in ("dir-maps", "dir-player", "dir-mosaic")] + [
    {"id": "dir-maps", "x": 0, "y": 2 * (H + GY) - 150, "w": 460, "page": "page-directions",
     "text": "G · By map. The library is organised by beatmap, not by replay: each map is a banner with everyone's plays on it as pills, the best outlined. A replay is chosen through its map. Tradeoff: one play per map makes the banner heavy; many plays per map make it shine."},
    {"id": "dir-player", "x": W + GX, "y": 2 * (H + GY) - 150, "w": 460, "page": "page-directions",
     "text": "H · The player. A media player everyone already knows: now playing large with transport and a seek bar, the queue on the right, Render and Judge where a like button would be. A bot job appears in the same transport as a recording. Tradeoff: a queue implies order; searching 187 is scrolling a list."},
    {"id": "dir-mosaic", "x": 2 * (W + GX), "y": 2 * (H + GY) - 150, "w": 460, "page": "page-directions",
     "text": "I · The mosaic. Frames edge to edge with no boxes, no borders, no text blocks — names live on the frames; the menu and the state are veils that fade in at the top and bottom. Visual density without the old card grid. Tradeoff: the eye has no rows to rest on; the frames must be good."},
]
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards))
