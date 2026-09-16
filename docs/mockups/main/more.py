import json
from pathlib import Path

from make import STYLE, LETTER, TICK, DOT, NONE, SEARCH, CARDS, page
from directions import EXTRA, rows

HERE = Path(__file__).parent

MORE = """
    .scene { position: absolute; inset: 0; background: #0a0507 url(frame.jpg) center/cover; }
    .scene::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(7,3,4,0.72) 0%, rgba(7,3,4,0.18) 30%, rgba(7,3,4,0.18) 60%, rgba(7,3,4,0.86) 100%); }
    .over { position: absolute; left: 40px; right: 40px; z-index: 2; }
    .over.top { top: 24px; display: flex; align-items: center; gap: 14px; }
    .over.bottom { bottom: 28px; display: flex; align-items: flex-end; gap: 24px; }
    .over h2 { margin: 0; font-size: 26px; line-height: 30px; font-weight: 600; letter-spacing: -0.01em; }
    .over .sub { margin-top: 4px; color: #a9a29b; }
    .over .acc { margin-left: auto; text-align: right; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 44px; line-height: 44px; font-weight: 700; letter-spacing: -0.02em; }
    .over .acc small { display: block; font-size: 12px; font-weight: 400; color: #a9a29b; letter-spacing: 0; margin-top: 4px; }
    .strip { position: absolute; left: 40px; right: 40px; bottom: 112px; z-index: 2; display: flex; gap: 6px; }
    .strip i { display: block; flex: 1; height: 3px; border-radius: 2px; background: rgba(255,255,255,0.16); }
    .strip i.on { background: #e24848; }
    .halves { position: absolute; inset: 0; display: grid; grid-template-columns: 1fr 1fr; }
    .half { padding: 28px 32px; box-sizing: border-box; min-width: 0; }
    .half + .half { border-left: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.18); }
    .half h1 { margin: 0; font-size: 22px; line-height: 28px; font-weight: 600; letter-spacing: -0.01em; }
    .half h1 span { color: #a9a29b; font-size: 16px; font-weight: 400; margin-left: 8px; }
    .hero { margin-top: 16px; aspect-ratio: 16 / 9; border-radius: 10px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.08); position: relative; }
    .hero .who { position: absolute; left: 0; right: 0; bottom: 0; padding: 22px 12px 10px; background: linear-gradient(transparent, rgba(7,3,4,0.85)); display: flex; align-items: flex-end; gap: 10px; }
    .hero .who b { font-size: 14px; }
    .hero .who span { color: #a9a29b; font-size: 12px; }
    .hero .who .a { margin-left: auto; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; font-size: 16px; }
    .lst { margin-top: 12px; }
    .lst .rw { grid-template-columns: minmax(0, 1fr) 62px; height: 36px; }
    .kv { display: flex; justify-content: space-between; gap: 12px; padding: 9px 0; border-top: 1px solid rgba(255,255,255,0.08); font-size: 13px; }
    .kv:first-child { border-top: 0; }
    .kv .k { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .gear { position: absolute; right: 24px; top: 24px; width: 32px; height: 32px; border-radius: 8px; display: flex; align-items: center; justify-content: center; color: #a9a29b; }
    .gear svg { width: 18px; height: 18px; stroke: currentColor; fill: none; stroke-width: 1.7; stroke-linecap: round; }
    .time { position: absolute; left: 50%; top: 96px; bottom: 0; width: 720px; margin-left: -360px; }
    .day { display: flex; gap: 20px; margin-bottom: 6px; }
    .day .d { width: 92px; flex: none; text-align: right; padding-top: 10px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .day .d b { display: block; color: #ece7e2; font-size: 14px; font-weight: 700; }
    .day .axis { width: 14px; flex: none; position: relative; }
    .day .axis::before { content: ""; position: absolute; left: 6px; top: 0; bottom: -6px; width: 1px; background: rgba(255,255,255,0.08); }
    .day .axis i { position: absolute; left: 3px; top: 13px; width: 7px; height: 7px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.16); }
    .day .axis i.farm { background: #a9a29b; box-shadow: none; }
    .day .ev { flex: 1; min-width: 0; display: flex; align-items: center; gap: 14px; padding: 8px 12px; border-radius: 10px; }
    .day .ev.play { border: 1px solid rgba(255,255,255,0.08); background: rgba(255,255,255,0.045); }
    .day .ev .th { width: 72px; height: 40px; border-radius: 6px; background: #0a0507 url(frame.jpg) center/cover; flex: none; }
    .day .ev .t { min-width: 0; }
    .day .ev .t b { display: block; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .day .ev .t span { color: #a9a29b; font-size: 12px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .day .ev .a { margin-left: auto; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .day .ev.farm { color: #a9a29b; font-size: 13px; padding: 6px 12px; }
    .day .ev.farm b { color: #ece7e2; font-weight: 600; }
"""

def with_more(html):
    return html.replace("  </style>", EXTRA + MORE + "  </style>", 1)

def direction_scene():
    body = f'''
  <div class="scene"></div>
  <div class="over top">
    <div class="brand" style="padding: 0;">{LETTER}<i></i><b>Dossier</b></div>
    <div class="words"><b>Replays</b><span>Farm</span><span>Settings</span></div>
  </div>
  <div class="strip">{"".join('<i class="on"></i>' if i == 0 else "<i></i>" for i in range(12))}</div>
  <div class="over bottom">
    <div><h2>NaumRedlo</h2><div class="sub">Dj Grimoire — Astral Quantization [Nattu VN0TH3R] · HD DT HR · 604x · lazer · 14 авг</div>
      <div style="display: flex; gap: 8px; margin-top: 14px;"><span class="btn primary">Render</span><span class="btn quiet">Judge</span><span class="btn quiet">Next →</span></div></div>
    <div class="acc">98,71%<small>1 of 187 · ● ready to take work</small></div>
  </div>'''
    return with_more(page(body))

def direction_halves():
    lst = ""
    for c in CARDS[1:5]:
        client, outcome, good, player, mp, acc, mods, combo, when = c
        lst += f'<div class="rw"><span class="p">{player}<span>{mp}</span></span><span class="a">{acc}</span></div>'
    body = f'''
  <div class="halves">
    <div class="half">
      <h1>Yours<span>187</span></h1>
      <div class="hero"><span class="tag l">lazer</span><span class="tag r good">FC</span><div class="who"><b>NaumRedlo</b><span>Dj Grimoire — Astral Quantization</span><span class="a">98,71%</span></div></div>
      <div class="lst rows">{lst}</div>
      <div class="foot"><span>Newest first · scroll for more</span></div>
    </div>
    <div class="half">
      <h1>The farm</h1>
      <div class="now" style="margin-top: 16px;"><div style="flex: 1;"><div class="head"><span class="dot"></span>Rendering for the bot <span>· 4 of 6</span></div><div class="cap" style="margin-top: 2px;">Camellia — Exit This Earth's Atomosphere [Collab]</div>
        <div class="ledger" style="margin-top: 12px;">{"".join(f'<div class="line {c}">{g}<span>{n}{d}</span></div>' for c, g, n, d in [("done", TICK, "Replay received", ""), ("done", TICK, "Map on disk", ""), ("cur", DOT, "Drawing", ' <span class="d">· 4,512 of 7,280</span>'), ("todo", NONE, "Encoding", ""), ("todo", NONE, "Delivering", "")])}</div></div></div>
      <div style="margin-top: 18px;">
        <div class="kv"><span class="k">this device</span><span>341 frames/s · 10 threads</span></div>
        <div class="kv"><span class="k">delivered</span><span>41 · 2 handed back</span></div>
        <div class="kv"><span class="k">online</span><span>3 devices · 3 waiting</span></div>
      </div>
    </div>
  </div>
  <div class="brand" style="position: absolute; left: 32px; bottom: 24px; padding: 0; opacity: 0.85;">{LETTER}<i></i><b>Dossier</b></div>
  <div class="gear"><svg viewBox="0 0 24 24"><path d="M4 7h10M18 7h2M4 17h4M12 17h8"></path><circle cx="15" cy="7" r="2.2"></circle><circle cx="9" cy="17" r="2.2"></circle></svg></div>'''
    return with_more(page(body))

def direction_time():
    def play(c):
        client, outcome, good, player, mp, acc, mods, combo, when = c
        m = " ".join(x for _, x in mods)
        return f'<div class="ev play"><span class="th"></span><span class="t"><b>{player} · {mp}</b><span>{m} · {combo} · {outcome or "no map"}</span></span><span class="a">{acc}</span></div>'
    days = [
        ("today", "16 сен", [("play", CARDS[0]), ("farm", "Rendered for <b>@tosha</b> · Camellia — Exit This Earth's Atomosphere · 3 min 40 s")]),
        ("", "14 авг", [("play", CARDS[1]), ("play", CARDS[2])]),
        ("", "3 авг", [("play", CARDS[3]), ("farm", "Delivered 6 to the bot · 41 in all")]),
        ("", "29 июл", [("play", CARDS[4])]),
    ]
    out = ""
    for label, date, events in days:
        for i, (kind, ev) in enumerate(events):
            d = f'<span class="d">{"<b>" + label + "</b>" if label and i == 0 else ""}{date if i == 0 else ""}</span>'
            axis = f'<span class="axis"><i class="{"farm" if kind == "farm" else ""}"></i></span>'
            body = play(ev) if kind == "play" else f'<div class="ev farm">{ev}</div>'
            out += f'<div class="day">{d}{axis}{body}</div>'
    body = f'''
  <div class="top" style="position: absolute; left: 40px; right: 40px; top: 24px;">
    <div class="brand">{LETTER}<i></i><b>Dossier</b></div>
    <div class="field" style="margin-left: 24px; width: 240px;">{SEARCH}<span>Player, map, day</span></div>
    <div class="words"><b>All</b><span>Yours</span><span>The farm</span><span>Settings</span></div>
  </div>
  <div class="time">{out}</div>
  <div class="bar"><span class="dot"></span><b>Ready to take work</b><span>· 341 frames/s · 3 waiting</span></div>'''
    return with_more(page(body))

boards = {"DirectionScene": direction_scene(), "DirectionHalves": direction_halves(), "DirectionTime": direction_time()}
for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

manifest = json.loads((HERE / "canvas.json").read_text())
W, H, GX, GY = 980, 720, 80, 160
manifest["artboards"] = [a for a in manifest["artboards"] if a["file"] not in ("DirectionScene.dc.html", "DirectionHalves.dc.html", "DirectionTime.dc.html")] + [
    {"file": "DirectionScene.dc.html", "x": 0, "y": H + GY, "w": W, "h": H, "page": "page-directions", "title": "D · The scene"},
    {"file": "DirectionHalves.dc.html", "x": W + GX, "y": H + GY, "w": W, "h": H, "page": "page-directions", "title": "E · Two halves"},
    {"file": "DirectionTime.dc.html", "x": 2 * (W + GX), "y": H + GY, "w": W, "h": H, "page": "page-directions", "title": "F · The journal"},
]
manifest["annotations"] = [a for a in manifest["annotations"] if a["id"] not in ("dir-scene", "dir-halves", "dir-time")] + [
    {"id": "dir-scene", "x": 0, "y": H + GY - 150, "w": 460, "page": "page-directions",
     "text": "D · The scene. The play itself fills the window — the engine draws it live, dimmed — and everything else is a thin overlay: name and map bottom-left, accuracy bottom-right, the strip of dashes is the whole library. Immersive. Tradeoff: continuous drawing; reads best on 'still' as a poster."},
    {"id": "dir-halves", "x": W + GX, "y": H + GY - 150, "w": 460, "page": "page-directions",
     "text": "E · Two halves. No menu at all: the window is the two things this application is — your replays on the left, the farm on the right — both visible at once, settings behind one gear. Tradeoff: narrow halves at 980 px; both halves must earn their space every day."},
    {"id": "dir-time", "x": 2 * (W + GX), "y": H + GY - 150, "w": 460, "page": "page-directions",
     "text": "F · The journal. One stream in time: your plays and the device's work for the bot as entries on the same line, newest first — a dossier as a case file. Filters are words at the top. Tradeoff: two kinds of things share a stream; a big library is a long scroll, search matters."},
]
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards))
