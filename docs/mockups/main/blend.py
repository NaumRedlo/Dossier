import json
from pathlib import Path

from make import LETTER, SEARCH, TICK, DOT, NONE, page
from directions import EXTRA
from more import MORE
from more2 import MORE2

HERE = Path(__file__).parent

BLEND = """
    .scene2 { position: absolute; inset: 0; background: #0a0507 url(frame.jpg) center/cover; }
    .scene2::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(7,3,4,0.82) 0%, rgba(7,3,4,0.14) 24%, rgba(7,3,4,0.14) 52%, rgba(7,3,4,0.88) 78%, rgba(7,3,4,0.97) 100%); }
    .crest { position: absolute; left: 0; right: 0; top: 24px; z-index: 3; display: flex; justify-content: center; }
    .crest .brand { padding: 0; gap: 14px; }
    .crest .brand img { width: 36px; height: 36px; padding: 2.5px; box-sizing: border-box; }
    .crest .brand i { height: 22px; }
    .crest .brand b { font-size: 20px; line-height: 26px; }
    .corner { position: absolute; top: 28px; z-index: 3; }
    .corner.l { left: 40px; }
    .corner.r { right: 40px; }
    .id { position: absolute; left: 40px; right: 40px; bottom: 156px; z-index: 2; display: flex; align-items: flex-end; gap: 24px; }
    .id .date { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; color: #a9a29b; letter-spacing: 0.04em; }
    .id h2 { margin: 2px 0 0; font-size: 28px; line-height: 32px; font-weight: 600; letter-spacing: -0.01em; }
    .id .map { margin-top: 2px; color: #ece7e2; font-size: 14px; }
    .id .meta { margin-top: 6px; display: flex; align-items: center; gap: 8px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .id .acts { display: flex; gap: 8px; margin-top: 14px; align-items: center; }
    .id .score { margin-left: auto; text-align: right; }
    .id .score .n { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 52px; line-height: 52px; font-weight: 700; letter-spacing: -0.03em; font-variant-numeric: tabular-nums; }
    .id .score .o { display: inline-flex; align-items: center; gap: 6px; margin-top: 8px; padding: 2px 8px; border-radius: 6px; background: rgba(10,5,7,0.6); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; font-weight: 700; color: #ece7e2; }
    .id .score .counts { display: flex; justify-content: flex-end; gap: 14px; margin-top: 10px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; color: #a9a29b; }
    .id .score .counts b { color: #ece7e2; font-weight: 700; }
    .id .score .counts i { display: inline-block; width: 7px; height: 7px; border-radius: 50%; margin-right: 5px; vertical-align: 1px; }
    .journal { position: absolute; left: 0; right: 0; bottom: 40px; z-index: 2; padding: 0 40px; }
    .journal .rail { position: relative; height: 1px; background: rgba(255,255,255,0.12); margin: 0 0 10px; }
    .journal .days { display: flex; gap: 22px; align-items: flex-end; overflow: hidden; }
    .day2 { display: flex; flex-direction: column; gap: 6px; flex: none; position: relative; }
    .day2 .lbl { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; color: #6b655f; letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; }
    .day2 .lbl.on { color: #ece7e2; }
    .day2 .lbl i { display: block; width: 5px; height: 5px; border-radius: 50%; background: #6b655f; }
    .day2 .lbl.on i { background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.16); }
    .day2 .set { display: flex; gap: 6px; align-items: flex-end; }
    .fr { width: 88px; height: 50px; border-radius: 6px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.1); opacity: 0.7; position: relative; }
    .fr.on { opacity: 1; border: 2px solid #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.18); width: 96px; height: 54px; }
    .fr.lift { opacity: 1; transform: translateY(-4px) scale(1.06); border-color: rgba(255,255,255,0.3); }
    .fr.dim { opacity: 0.22; }
    .fr.none { background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.04) 0 4px, transparent 4px 9px); }
    .fr .a { position: absolute; right: 4px; bottom: 3px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; font-weight: 700; color: #ece7e2; text-shadow: 0 1px 2px rgba(0,0,0,0.6); }
    .fr .busy { position: absolute; left: 5px; top: 5px; width: 7px; height: 7px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.2); }
    .tip { position: absolute; left: 96px; bottom: 74px; z-index: 5; padding: 8px 10px; border-radius: 8px; background: rgba(20,7,10,0.96); border: 1px solid rgba(255,255,255,0.12); box-shadow: 0 8px 24px rgba(0,0,0,0.45); white-space: nowrap; font-size: 12px; }
    .tip b { display: block; font-size: 13px; }
    .tip span { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .counter { position: absolute; right: 40px; bottom: 14px; z-index: 2; display: flex; gap: 14px; align-items: center; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .ledg { display: flex; flex-direction: column; gap: 2px; margin-top: 10px; }
    .ledg .line { height: 22px; font-size: 13px; }
    .ledg .line .g { width: 12px; height: 12px; }
    .story { position: absolute; left: 40px; right: 40px; top: 96px; display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; }
    .story .shot { border-radius: 10px; overflow: hidden; border: 1px solid rgba(255,255,255,0.1); width: 293px; height: 215px; position: relative; background: #070304; }
    .story .shot .inner { position: absolute; left: 0; top: 0; width: 980px; height: 720px; transform: scale(0.299); transform-origin: 0 0; }
    .story .cap2 { margin-top: 10px; font-size: 13px; color: #a9a29b; line-height: 18px; }
    .story .cap2 b { display: block; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; margin-bottom: 2px; }
"""

def with_blend(html):
    return html.replace("  </style>", EXTRA + MORE + MORE2 + BLEND + "  </style>", 1)

def frames(items, on=-1, lift=-1, dim=(), busy=-1):
    out = ""
    for i, it in enumerate(items):
        cls = "fr"
        if i == on: cls += " on"
        if i == lift: cls += " lift"
        if i in dim: cls += " dim"
        if it == "none": cls += " none"
        inner = "" if it == "none" else f'<span class="a">{it}</span>'
        if i == busy: inner += '<span class="busy"></span>'
        out += f'<span class="{cls}">{inner}</span>'
    return out

DAYS_EN = [("today", ["98,71%"]), ("Aug 14", ["99,65%", "93,03%"]), ("Aug 3", ["none"]), ("Jul 29", ["72,13%", "96,40%", "97,88%"]), ("May 10, 2025", ["none", "88,12%"])]
DAYS_RU = [("сегодня", ["98,71%"]), ("14 авг", ["99,65%", "93,03%"]), ("3 авг", ["none"]), ("29 июл", ["72,13%", "96,40%", "97,88%"]), ("10 мая 2025", ["none", "88,12%"])]

def strip(days, on_day=0, lift=None, dim_days=(), busy=False):
    out = ""
    for i, (label, items) in enumerate(days):
        dim = tuple(range(len(items))) if i in dim_days else ()
        l = lift[1] if lift and lift[0] == i else -1
        out += (f'<div class="day2"><span class="lbl{" on" if i == on_day else ""}"><i></i>{label}</span>'
                f'<span class="set">{frames(items, on=0 if i == on_day else -1, lift=l, dim=dim, busy=0 if (busy and i == 0) else -1)}</span></div>')
    return out

def chrome(en, search_text=None, focused=False):
    words = ("Replays", "Worker", "Settings") if en else ("Реплеи", "Воркер", "Настройки")
    ph = search_text if search_text is not None else ("Player, map, day" if en else "Игрок, карта, день")
    colour = "#ece7e2" if search_text else "#6b655f"
    edge = "rgba(255,255,255,0.3)" if focused else "rgba(255,255,255,0.08)"
    return (f'<div class="crest"><div class="brand">{LETTER}<i></i><b>Dossier</b></div></div>'
            f'<div class="corner l"><div class="field" style="width: 240px; background: rgba(10,5,7,0.55); border-color: {edge}; color: {colour};">{SEARCH}<span>{ph}</span></div></div>'
            f'<div class="corner r"><div class="words"><b>{words[0]}</b><span>{words[1]}</span><span>{words[2]}</span></div></div>')

def viewer(en, hover_acc=False, rendering=False, nomap=False):
    date = "today · 14:02 · lazer" if en else "сегодня · 14:02 · lazer"
    render = "Render" if en else "Отрендерить"
    counts = ""
    if hover_acc:
        counts = ('<div class="counts"><span><i style="background:#66ccff"></i>300 <b>1 184</b></span><span><i style="background:#88b300"></i>100 <b>77</b></span>'
                  '<span><i style="background:#f0c060"></i>50 <b>15</b></span><span><i style="background:#e24848"></i>✕ <b>27</b></span></div>')
    if rendering:
        steps = [("done", TICK, "Replay read" if en else "Реплей прочитан", ""), ("done", TICK, "Map on disk" if en else "Карта на месте", ""),
                 ("done", TICK, "Judged" if en else "Судейство сошлось", ""), ("cur", DOT, "Drawing" if en else "Рисую", " · 4,512 / 7,280 · 18 s" if en else " · 4 512 / 7 280 · 18 с"),
                 ("todo", NONE, "Encoding" if en else "Кодирую", ""), ("todo", NONE, "Saving" if en else "Сохраняю", "")]
        ledger = "".join(f'<div class="line {c}">{g}<span>{n}<span class="d">{d}</span></span></div>' for c, g, n, d in steps)
        left = (f'<div class="date">{date}</div><h2>NaumRedlo</h2><div class="map">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</div>'
                f'<div class="ledg">{ledger}</div><div class="acts" style="margin-top: 10px;"><span class="btn quiet" style="padding-left: 0;">{"Stop" if en else "Стоп"}</span></div>')
    elif nomap:
        left = (f'<div class="date">{"Aug 3 · 09:12 · stable" if en else "3 авг · 09:12 · stable"}</div><h2>Deeo_XD</h2><div class="map">Deeo_XD - Chocofan - LUCKY CAT [_]</div>'
                f'<div class="meta"><span class="mods"><span class="mod h">DT</span></span><span>312x</span></div>'
                f'<div class="acts"><span class="btn primary">{"Get the map" if en else "Скачать карту"}</span><span class="cap">{"Nothing to draw or judge without it." if en else "Без карты нечего ни рисовать, ни судить."}</span></div>')
    else:
        left = (f'<div class="date">{date}</div><h2>NaumRedlo</h2><div class="map">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</div>'
                f'<div class="meta"><span class="mods"><span class="mod h">HD</span><span class="mod h">DT</span><span class="mod h">HR</span></span><span>604x</span><span>·</span><span>3:51</span></div>'
                f'<div class="acts"><span class="btn primary">{render}</span></div>')
    score = ('<div class="score"><div class="n" style="color:#6b655f">—</div></div>' if nomap
             else f'<div class="score"><div class="n">98,71%</div><span class="o">FC</span>{counts}</div>')
    return f'<div class="id"><div>{left}</div>{score}</div>'

def blend(en, variant="rest"):
    days = DAYS_EN if en else DAYS_RU
    body = '<div class="scene2"></div>' + chrome(en)
    tip = ""
    if variant == "rest":
        body += viewer(en) + f'<div class="journal"><div class="rail"></div><div class="days">{strip(days)}</div></div>'
    elif variant == "hover":
        tip = (f'<div class="tip" style="left: 366px;"><b>Guest</b><span>xi — Blue Zenith [Asphyxia\'s Hard] · EZ · 401x</span></div>')
        body += viewer(en, hover_acc=True) + f'<div class="journal"><div class="rail"></div><div class="days">{strip(days, lift=(1, 0))}</div></div>' + tip
    elif variant == "rendering":
        body += viewer(en, rendering=True) + f'<div class="journal"><div class="rail"></div><div class="days">{strip(days, busy=True)}</div></div>'
    elif variant == "search":
        body = '<div class="scene2"></div>' + chrome(en, search_text="saki", focused=True)
        body += viewer(en) + f'<div class="journal"><div class="rail"></div><div class="days">{strip(days, on_day=-1, dim_days=(0, 1, 2, 4))}</div></div>'
        body = body.replace('<span class="set">' + frames(["72,13%", "96,40%", "97,88%"], on=-1) + '</span>',
                            '<span class="set">' + frames(["72,13%", "96,40%", "97,88%"], on=0) + '</span>')
    elif variant == "nomap":
        body = body.replace('url(frame.jpg) center/cover', 'repeating-linear-gradient(-45deg, rgba(255,255,255,0.025) 0 8px, transparent 8px 18px)')
        body += viewer(en, nomap=True) + f'<div class="journal"><div class="rail"></div><div class="days">{strip(days, on_day=2)}</div></div>'
    body += f'<div class="counter"><span>1 / 187</span><span>‹ ›</span></div>'
    return with_blend(page(body))

def storyboard(en):
    def shot(inner, cap_b, cap):
        return f'<div><div class="shot"><div class="inner">{inner}</div></div><div class="cap2"><b>{cap_b}</b>{cap}</div></div>'
    only_brand = f'<div class="win" style="background:#070304;"><div class="crest"><div class="brand">{LETTER}<i></i><b>Dossier</b></div></div></div>'
    faded_chrome = chrome(en).replace('class="corner l"', 'class="corner l" style="opacity:0.4"').replace('class="corner r"', 'class="corner r" style="opacity:0.4"')
    half_date = "today · 14" if en else "сегодня · 14"
    half_strip = strip(DAYS_EN if en else DAYS_RU)
    half = (f'<div class="win"><div class="scene2" style="opacity:0.45;"></div>{faded_chrome}'
            f'<div class="id" style="opacity:0.7;"><div><div class="date">{half_date}</div><h2>NaumRe</h2></div></div>'
            f'<div class="journal" style="bottom: 8px; opacity: 0.5;"><div class="rail"></div><div class="days">{half_strip}</div></div></div>')
    full = blend(en).split('<div class="win">')[1].rsplit('</div>\n</x-dc>', 1)[0]
    full = '<div class="win">' + full
    caps = [
        ("0 ms", "Only the crest, where the first run left it — nothing else has arrived. From a cold start this is the loading screen: the emblem alone, no spinner." if en else "Только знак — там, где его оставил первый запуск. При холодном старте это и есть экран загрузки: буква без спиннера."),
        ("~450 ms", "The scene fades up from black (640 ms), the journal rises from below the edge with its frames staggered 30 ms apart, the viewer's words start typing." if en else "Сцена проявляется из черноты (640 мс), журнал поднимается из-за края, кадры с шагом 30 мс, слова просмотра начинают печататься."),
        ("~1.2 s", "Everything at rest. The crest never moved: the first run's brand block and the main screen's are the same block in the same place." if en else "Всё на месте. Знак не двигался: блок первого запуска и главного экрана — один и тот же, на одном месте."),
    ]
    shots = shot(only_brand, *caps[0]) + shot(half, *caps[1]) + shot(full, *caps[2])
    title = "Entering the main screen" if en else "Вход на главный экран"
    body = (f'<div class="crest"><div class="brand">{LETTER}<i></i><b>Dossier</b></div></div>'
            f'<div style="position:absolute; left:40px; top:70px; font-size:16px; font-weight:600;">{title}</div><div class="story">{shots}</div>')
    return with_blend(page(body))

boards = {
    "Blend": blend(True), "BlendRu": blend(False),
    "BlendHover": blend(True, "hover"), "BlendRendering": blend(True, "rendering"),
    "BlendSearch": blend(True, "search"), "BlendNoMap": blend(True, "nomap"),
    "BlendEntrance": storyboard(True),
}
for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

manifest = json.loads((HERE / "canvas.json").read_text())
W, H, GX, GY = 980, 720, 80, 160
mine = {
    "Blend.dc.html": ("At rest", 0, 0), "BlendRu.dc.html": ("В покое", 1, 0),
    "BlendHover.dc.html": ("Hovering a frame, hovering the accuracy", 0, 1), "BlendRendering.dc.html": ("Render pressed", 1, 1), "BlendSearch.dc.html": ("Typing in search", 2, 1),
    "BlendNoMap.dc.html": ("A replay without its map", 0, 2), "BlendEntrance.dc.html": ("Entering", 1, 2),
}
if not any(p["id"] == "page-blend" for p in manifest["pages"]):
    manifest["pages"].insert(0, {"id": "page-blend", "name": "Main screen"})
manifest["pages"] = [{"id": "page-blend", "name": "Main screen"} if p["id"] == "page-blend" else p for p in manifest["pages"]]
manifest["artboards"] = [a for a in manifest["artboards"] if a["file"] not in mine] + [
    {"file": f, "x": c * (W + GX), "y": r * (H + GY), "w": W, "h": H, "page": "page-blend", "title": t} for f, (t, c, r) in mine.items()
]
manifest["annotations"] = [a for a in manifest["annotations"] if a["id"] != "blend"] + [
    {"id": "blend", "x": 0, "y": -210, "w": 560, "page": "page-blend",
     "text": "The scene is the window; the viewer is the lower third; the journal is the strip along the bottom, replays only, by day — today is the word alone, other days the date in each language's way, the year only when it is not this one. The crest is the first run's block, centred, unmoved. No state line: the worker gets its own screen and, later, an operations centre for downloads and notices. Second row: what pressing and hovering shows."},
]
manifest["launch"] = {"view": "canvas", "page": "page-blend"}
(HERE / "canvas.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards))
