import json
from pathlib import Path

HERE = Path(__file__).parent

STYLE = """
    body { margin: 0; background: #070304; color: #ece7e2; font-family: Commissioner, "Helvetica Neue", Arial, sans-serif; font-size: 14px; line-height: 20px; -webkit-font-smoothing: antialiased; }
    a { color: #e24848; } a:hover { color: #f06060; }
    .win { position: relative; width: 980px; height: 720px; overflow: hidden; background: radial-gradient(115% 95% at 50% -18%, #3a1015 0%, #26090f 30%, #17070b 55%, #0d0508 78%, #070304 100%); }
    .mono { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .side { position: absolute; left: 0; top: 0; bottom: 0; width: 220px; box-sizing: border-box; padding: 20px 12px 16px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.26); }
    .brand { display: flex; align-items: center; gap: 14px; padding: 0 8px 18px; }
    .brand img { width: 36px; height: 36px; padding: 2.5px; box-sizing: border-box; display: block; }
    .brand i { display: block; width: 1px; height: 22px; background: rgba(255,255,255,0.16); }
    .brand b { font-size: 20px; line-height: 26px; font-weight: 600; letter-spacing: -0.01em; }
    .nav { display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 10px; border-radius: 8px; color: #a9a29b; font-weight: 500; }
    .nav.on { background: rgba(226,72,72,0.16); color: #ece7e2; }
    .nav svg { width: 17px; height: 17px; stroke: currentColor; fill: none; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; flex: none; }
    .state { margin-top: auto; padding: 12px 10px; border-radius: 8px; background: rgba(0,0,0,0.26); }
    .state .l { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 13px; line-height: 18px; }
    .state .n { margin-top: 3px; padding-left: 16px; color: #a9a29b; font-size: 12px; line-height: 16px; }
    .dot { display: block; width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); flex: none; }
    .dot.off { background: #6b655f; box-shadow: none; }
    .main { position: absolute; left: 220px; top: 0; right: 0; bottom: 0; padding: 28px 40px 0; box-sizing: border-box; overflow: hidden; }
    .title { display: flex; align-items: baseline; gap: 10px; }
    .title h1 { margin: 0; font-size: 22px; line-height: 28px; font-weight: 600; letter-spacing: -0.01em; }
    .title span { color: #a9a29b; font-size: 16px; }
    .tools { display: flex; align-items: center; gap: 12px; margin-top: 18px; }
    .field { display: flex; align-items: center; gap: 8px; width: 260px; height: 32px; padding: 0 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.26); color: #6b655f; box-sizing: border-box; }
    .field svg { width: 14px; height: 14px; stroke: #a9a29b; fill: none; stroke-width: 2; stroke-linecap: round; }
    .chips { display: flex; gap: 6px; }
    .chip { display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 11px; border-radius: 14px; border: 1px solid rgba(255,255,255,0.08); color: #a9a29b; font-size: 13px; font-weight: 500; box-sizing: border-box; }
    .chip.on { background: #ece7e2; border-color: #ece7e2; color: #14070a; font-weight: 600; }
    .chip .n { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; color: #6b655f; }
    .chip.on .n { color: #6b5c60; }
    .grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; margin-top: 20px; }
    .card { display: flex; flex-direction: column; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; background: rgba(255,255,255,0.045); overflow: hidden; }
    .stage { position: relative; aspect-ratio: 16 / 9; background: #0a0507; }
    .stage img { display: block; width: 100%; height: 100%; object-fit: cover; }
    .tag { position: absolute; top: 8px; padding: 2px 7px; border-radius: 6px; background: rgba(10,5,7,0.72); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; line-height: 16px; font-weight: 700; }
    .tag.l { left: 8px; color: #a9a29b; font-weight: 500; }
    .tag.r { right: 8px; color: #e24848; }
    .tag.r.good { color: #ece7e2; }
    .who { display: grid; grid-template-columns: minmax(0, 1fr) auto; grid-template-areas: "name acc" "map acc" "meta meta"; column-gap: 10px; padding: 10px 12px 12px; }
    .who b { grid-area: name; font-size: 14px; line-height: 20px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .who .map { grid-area: map; margin: 0; color: #a9a29b; font-size: 12px; line-height: 16px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .who .acc { grid-area: acc; align-self: start; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 16px; line-height: 20px; font-weight: 700; font-variant-numeric: tabular-nums; }
    .who .meta { grid-area: meta; display: flex; align-items: center; gap: 6px; margin-top: 8px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; line-height: 16px; white-space: nowrap; }
    .who .meta .when { margin-left: auto; color: #6b655f; }
    .mods { display: inline-flex; gap: 3px; }
    .mod { display: inline-flex; align-items: center; justify-content: center; height: 16px; min-width: 26px; padding: 0 4px; border-radius: 4px; color: #fff; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 800; font-size: 9px; letter-spacing: 0.02em; }
    .mod.e { background: #7ac65c; } .mod.h { background: #d64e48; } .mod.a { background: #5694d6; } .mod.c { background: #9668ce; } .mod.o { background: #706064; }
    .hover { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; background: rgba(10,5,7,0.35); }
    .btn { display: inline-flex; align-items: center; justify-content: center; height: 32px; padding: 0 12px; border-radius: 8px; font-weight: 600; font-size: 14px; white-space: nowrap; box-sizing: border-box; }
    .btn.primary { background: #e24848; color: #fff; }
    .btn.quiet { color: #a9a29b; }
    .ledger { display: flex; flex-direction: column; gap: 4px; }
    .line { display: flex; align-items: center; gap: 12px; height: 24px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 14px; color: #6b655f; }
    .line.done { color: #a9a29b; } .line.cur { color: #ece7e2; font-weight: 700; }
    .line .g { display: flex; align-items: center; justify-content: center; width: 14px; height: 14px; flex: none; }
    .line .d { color: #6b655f; font-weight: 400; } .line.cur .d { color: #a9a29b; }
    .now { display: flex; align-items: flex-start; gap: 20px; margin-top: 20px; padding: 16px 20px; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; background: rgba(255,255,255,0.045); }
    .now .head { display: flex; align-items: center; gap: 8px; font-size: 16px; line-height: 22px; font-weight: 600; }
    .now .head span { color: #a9a29b; font-weight: 400; }
    .empty { margin-top: 20px; padding: 40px 24px; border: 1px dashed rgba(255,255,255,0.16); border-radius: 12px; text-align: center; }
    .empty b { display: block; font-size: 16px; line-height: 22px; font-weight: 600; }
    .empty p { margin: 6px 0 16px; color: #a9a29b; }
    .cap { font-size: 12px; line-height: 16px; color: #a9a29b; }
"""

LETTER = '<img src="letter.png">'
TICK = '<span class="g"><svg viewBox="0 0 16 16" width="14" height="14"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="#e24848" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path></svg></span>'
DOT = '<span class="g"><span class="dot"></span></span>'
NONE = '<span class="g"></span>'
ICONS = {
    "replays": '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="3"></rect><path d="M10 9.5v5l4-2.5z"></path></svg>',
    "farm": '<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="6" rx="2"></rect><rect x="3" y="14" width="18" height="6" rx="2"></rect><circle cx="7" cy="7" r="0.9"></circle><circle cx="7" cy="17" r="0.9"></circle></svg>',
    "settings": '<svg viewBox="0 0 24 24"><path d="M4 7h10M18 7h2M4 17h4M12 17h8"></path><circle cx="15" cy="7" r="2.2"></circle><circle cx="9" cy="17" r="2.2"></circle></svg>',
}
SEARCH = '<svg viewBox="0 0 24 24"><circle cx="10.5" cy="10.5" r="6.2"></circle><path d="M15.2 15.2 20 20"></path></svg>'

EN = {
    "items": ["Replays", "Worker", "Settings"],
    "title": "Replays",
    "search": "Player, map, file",
    "chips": ["All", "With a map", "Without a map"],
    "ready": "Ready to take work",
    "ready_n": "341 frames/s · 3 waiting",
    "rendering": "Rendering for the bot",
    "rendering_n": "4 of 6 · 62%",
    "notlinked": "Not linked to the bot",
    "notlinked_n": "Worker · pair",
    "map_none": "No map",
    "get": "Get the map",
    "render": "Render",
    "empty_b": "No replays yet",
    "empty_p": "Put .osr files into ~/.dossier/Replays, or add osu!'s folder in Settings.",
    "empty_btn": "Add a folder…",
    "now_head": "Rendering", "now_for": "NaumRedlo · Dj Grimoire — Astral Quantization",
    "steps": [("done", "Replay read", ""), ("done", "Map on disk", ""), ("done", "Judgement agrees", ""), ("now", "Drawing", "4,512 of 7,280 · 18 s left"), ("todo", "Encoding", ""), ("todo", "Saving", "")],
    "now_n": "4 of 6",
}
RU = {
    "items": ["Реплеи", "Воркер", "Настройки"],
    "title": "Реплеи",
    "search": "Игрок, карта, файл",
    "chips": ["Все", "С картой", "Без карты"],
    "ready": "Готов брать работу",
    "ready_n": "341 кадров/с · ждут 3",
    "rendering": "Рисую для бота",
    "rendering_n": "4 из 6 · 62%",
    "notlinked": "Не привязано к боту",
    "notlinked_n": "Воркер · привязать",
    "map_none": "Карты нет",
    "get": "Скачать карту",
    "render": "Отрендерить",
    "empty_b": "Реплеев пока нет",
    "empty_p": "Положите .osr в ~/.dossier/Replays или добавьте папку osu! в настройках.",
    "empty_btn": "Добавить папку…",
    "now_head": "Рисую", "now_for": "NaumRedlo · Dj Grimoire — Astral Quantization",
    "steps": [("done", "Реплей прочитан", ""), ("done", "Карта на месте", ""), ("done", "Судейство сошлось", ""), ("now", "Рисую", "4 512 из 7 280 · осталось 18 с"), ("todo", "Кодирую", ""), ("todo", "Сохраняю", "")],
    "now_n": "4 из 6",
}

CARDS = [
    ("lazer", "FC", True, "NaumRedlo", "Dj Grimoire — Astral Quantization [Nattu VN0TH3R]", "98,71%", [("h", "HD"), ("h", "DT"), ("h", "HR")], "604x", "14 авг"),
    ("stable", "SB", True, "Guest", "xi — Blue Zenith [Asphyxia's Hard]", "99,65%", [("e", "EZ")], "401x", "7 авг"),
    ("stable", "×27", False, "-legusshhka-", "xi — FREEDOM DiVE [Extra]", "93,03%", [("o", "NM")], "1137x", "2 авг"),
    (None, None, None, "Deeo_XD", "Deeo_XD - Chocofan - LUCKY CAT [_]", "—", [("h", "DT")], "312x", "3 авг"),
    ("stable", "Fail · 61,2%", False, "Saki_chan", "Chroma — sink to the deep sea world [HP0]", "72,13%", [("h", "HD"), ("h", "HR")], "185x", "29 июл"),
    ("stable", "×3", False, "ItarinKamiSama", "Phoneboy — Nevermind (feat. Justin Magnaye)", "96,40%", [("h", "NC")], "522x", "26 июл"),
]

def page(body):
    return f'''<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Commissioner:wght@400;500;600&family=JetBrains+Mono:wght@400;700;800&display=swap">
  <style>{STYLE}  </style>
</helmet>
<div class="win">
{body}
</div>
</x-dc>
</body>
</html>
'''

def side(t, on, state):
    items = "".join(
        f'<div class="nav{" on" if i == on else ""}">{ICONS[k]}<span>{name}</span></div>'
        for i, (k, name) in enumerate(zip(["replays", "farm", "settings"], t["items"]))
    )
    if state == "ready":
        block = f'<div class="state"><div class="l"><span class="dot"></span>{t["ready"]}</div><div class="n">{t["ready_n"]}</div></div>'
    elif state == "rendering":
        block = f'<div class="state"><div class="l"><span class="dot"></span>{t["rendering"]}</div><div class="n">{t["rendering_n"]}</div></div>'
    else:
        block = f'<div class="state"><div class="l"><span class="dot off"></span>{t["notlinked"]}</div><div class="n">{t["notlinked_n"]}</div></div>'
    return f'''  <div class="side">
    <div class="brand">{LETTER}<i></i><b>Dossier</b></div>
{items}
    {block}
  </div>'''

def card(t, c, hover=False):
    client, outcome, good, player, mp, acc, mods, combo, when = c
    badges = "".join(f'<span class="mod {k}">{m}</span>' for k, m in mods)
    if client is None:
        stage = (f'<div class="stage" style="background: repeating-linear-gradient(-45deg, rgba(255,255,255,0.025) 0 6px, transparent 6px 14px);">'
                 f'<div class="hover" style="background: none; flex-direction: column; gap: 8px;"><span class="cap">{t["map_none"]}</span><span class="btn primary" style="height: 28px; font-size: 12px;">{t["get"]}</span></div></div>')
    else:
        over = f'<div class="hover"><span class="btn primary">{t["render"]}</span></div>' if hover else ""
        stage = f'<div class="stage"><img src="frame.jpg"><span class="tag l">{client}</span><span class="tag r{" good" if good else ""}">{outcome}</span>{over}</div>'
    return (f'<div class="card">{stage}<div class="who"><b>{player}</b><p class="map">{mp}</p><span class="acc">{acc}</span>'
            f'<div class="meta"><span class="mods">{badges}</span><span>{combo}</span><span class="when">{when}</span></div></div></div>')

def tools(t, count):
    chips = "".join(
        f'<span class="chip{" on" if i == 0 else ""}">{name} <span class="n">{n}</span></span>'
        for i, (name, n) in enumerate(zip(t["chips"], [count, count - 2, 2]))
    )
    return (f'<div class="title"><h1>{t["title"]}</h1><span>{count}</span></div>'
            f'<div class="tools"><div class="field">{SEARCH}<span>{t["search"]}</span></div><div class="chips">{chips}</div></div>')

def ledger_now(t):
    lines = ""
    for mood, name, detail in t["steps"]:
        g = {"done": TICK, "now": DOT, "todo": NONE}[mood]
        d = f' <span class="d">· {detail}</span>' if detail else ""
        cls = {"now": "cur"}.get(mood, mood)
        lines += f'<div class="line {cls}">{g}<span>{name}{d}</span></div>'
    return (f'<div class="now"><div style="flex: 1;"><div class="head">{t["now_head"]} <span>· {t["now_n"]}</span></div>'
            f'<div class="cap" style="margin-top: 2px;">{t["now_for"]}</div><div class="ledger" style="margin-top: 12px;">{lines}</div></div>'
            f'<span class="btn quiet" style="margin-top: -4px;">Стоп</span></div>')

boards = {}

def replays(t, state="ready", hover=True, running=False):
    cards = "".join(card(t, c, hover=(hover and i == 0)) for i, c in enumerate(CARDS))
    now = ledger_now(t) if running else ""
    return page(side(t, 0, state) + f'\n  <div class="main">{tools(t, 187)}{now}<div class="grid">{cards}</div></div>')

boards["Main"] = replays(EN)
boards["Rendering"] = replays(EN, state="rendering", hover=False, running=True)
boards["Empty"] = page(side(EN, 0, "notlinked") + f'''
  <div class="main"><div class="title"><h1>{EN["title"]}</h1></div>
    <div class="empty"><b>{EN["empty_b"]}</b><p>{EN["empty_p"]}</p><span class="btn primary">{EN["empty_btn"]}</span></div>
  </div>''')
boards["MainRu"] = replays(RU)
boards["RenderingRu"] = replays(RU, state="rendering", hover=False, running=True)

s = boards["Rendering"].replace('<span class="btn quiet" style="margin-top: -4px;">Стоп</span>', '<span class="btn quiet" style="margin-top: -4px;">Stop</span>')
boards["Rendering"] = s

for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

W, H, GX, GY = 980, 720, 80, 120
artboards = [
    {"file": "Main.dc.html", "x": 0, "y": 0, "w": W, "h": H, "page": "page-en", "title": "Replays · ready"},
    {"file": "Rendering.dc.html", "x": W + GX, "y": 0, "w": W, "h": H, "page": "page-en", "title": "Replays · a render running"},
    {"file": "Empty.dc.html", "x": 2 * (W + GX), "y": 0, "w": W, "h": H, "page": "page-en", "title": "Replays · nothing yet, not linked"},
    {"file": "MainRu.dc.html", "x": 0, "y": 0, "w": W, "h": H, "page": "page-ru", "title": "Реплеи"},
    {"file": "RenderingRu.dc.html", "x": W + GX, "y": 0, "w": W, "h": H, "page": "page-ru", "title": "Реплеи · идёт рендер"},
]
canvas = {
    "pages": [{"id": "page-en", "name": "Main · EN"}, {"id": "page-ru", "name": "Main · RU"}],
    "artboards": artboards,
    "annotations": [
        {"id": "menu", "x": 0, "y": -200, "w": 520, "page": "page-en",
         "text": "The menu is three items and a state line. Nothing is a top-level item unless a person opens the application for it: Replays, Farm, Settings.\nThe state line at the bottom is the one place that says what the device is doing; when something runs, it becomes the ledger's headline, and the same ledger appears at the top of Replays until the work is done.\nNo sort control: newest first, search, three chips. The title carries the count."},
    ],
    "launch": {"view": "canvas", "page": "page-en"},
}
(HERE / "canvas.json").write_text(json.dumps(canvas, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards), "artboards")
