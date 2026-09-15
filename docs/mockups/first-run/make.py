import json
from pathlib import Path

HERE = Path(__file__).parent

STYLE = """
    body { margin: 0; background: #070304; color: #ece7e2; font-family: Commissioner, "Helvetica Neue", Arial, sans-serif; font-size: 14px; line-height: 20px; -webkit-font-smoothing: antialiased; }
    a { color: #e24848; } a:hover { color: #e0bd7a; }
    .win { position: relative; width: 980px; height: 720px; overflow: hidden; background: radial-gradient(115% 95% at 50% -18%, #3a1015 0%, #26090f 30%, #17070b 55%, #0d0508 78%, #070304 100%); }
    .col { position: absolute; left: 210px; top: 96px; width: 560px; display: flex; flex-direction: column; align-items: stretch; gap: 0; }
    .mono { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .brand { display: flex; align-items: center; gap: 10px; }
    .brand span { font-size: 16px; line-height: 22px; font-weight: 600; letter-spacing: -0.01em; }
    .head { display: flex; align-items: center; justify-content: center; gap: 10px; margin-top: 48px; font-size: 16px; line-height: 22px; font-weight: 600; }
    .head .n { color: #a9a29b; font-weight: 400; }
    .card { margin-top: 16px; display: flex; flex-direction: column; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; background: rgba(255,255,255,0.045); overflow: hidden; }
    .ledger { display: flex; flex-direction: column; gap: 4px; padding: 20px 24px; }
    .line { display: flex; align-items: center; gap: 12px; height: 24px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 14px; line-height: 20px; color: #6b655f; }
    .line.done { color: #a9a29b; }
    .line.now { color: #ece7e2; font-weight: 700; }
    .line.bad { color: #e24848; font-weight: 700; }
    .g { display: flex; align-items: center; justify-content: center; width: 14px; height: 14px; flex: none; }
    .g span { display: block; }
    .line .d { color: #6b655f; font-weight: 400; }
    .line.now .d { color: #a9a29b; }
    .sub { padding-left: 26px; margin-top: -2px; font-size: 12px; line-height: 16px; color: #a9a29b; }
    .step { display: flex; flex-direction: column; gap: 16px; padding: 24px; border-top: 1px solid rgba(255,255,255,0.08); }
    .step h2 { margin: 0; font-size: 16px; line-height: 22px; font-weight: 600; }
    .step .why { margin: -12px 0 0; color: #a9a29b; }
    .row { display: flex; align-items: center; gap: 8px; }
    .btn { display: inline-flex; align-items: center; justify-content: center; height: 32px; padding: 0 12px; border-radius: 8px; border: 1px solid transparent; font-weight: 600; font-size: 14px; white-space: nowrap; box-sizing: border-box; }
    .btn.primary { background: #e24848; color: #ffffff; }
    .btn.quiet { color: #a9a29b; }
    .btn.dim { background: rgba(226,72,72,0.16); color: #6b655f; }
    .field { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 0; }
    .field .cap { font-size: 12px; line-height: 16px; color: #a9a29b; }
    .field .box { display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.26); box-sizing: border-box; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 14px; color: #ece7e2; overflow: hidden; white-space: nowrap; }
    .box .ph { color: #6b655f; }
    .box .tag { margin-left: auto; padding: 0 6px; border-radius: 4px; background: rgba(255,255,255,0.06); color: #a9a29b; font-size: 12px; line-height: 18px; font-weight: 700; }
    .code { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 28px; line-height: 32px; font-weight: 700; letter-spacing: 0.12em; }
    .wait { display: flex; align-items: center; gap: 10px; color: #a9a29b; }
    .wait.ok { color: #ece7e2; }
    .links { display: flex; gap: 16px; font-size: 12px; line-height: 16px; }
    .links a { color: #a9a29b; text-decoration: none; }
    .src { display: flex; align-items: center; gap: 12px; height: 44px; padding: 0 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.26); box-sizing: border-box; }
    .src .path { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; color: #ece7e2; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .src .tag { padding: 0 6px; border-radius: 4px; background: rgba(255,255,255,0.06); color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; line-height: 18px; font-weight: 700; flex: none; }
    .src .n { margin-left: auto; color: #a9a29b; font-size: 12px; line-height: 16px; white-space: nowrap; flex: none; }
    .switch { display: flex; align-items: center; width: 32px; height: 18px; padding: 2px; border-radius: 9px; background: rgba(255,255,255,0.14); box-sizing: border-box; flex: none; }
    .switch.on { background: #e24848; justify-content: flex-end; }
    .switch i { display: block; width: 14px; height: 14px; border-radius: 50%; background: #14070a; }
    .switch:not(.on) i { background: #ece7e2; }
    .qr { width: 112px; height: 112px; padding: 6px; border-radius: 8px; background: #fff; flex: none; box-sizing: border-box; }
    .tiles { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
    .tile { padding: 12px 16px; border-radius: 8px; background: rgba(0,0,0,0.26); }
    .tile b { display: block; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 22px; line-height: 28px; font-weight: 700; font-variant-numeric: tabular-nums; }
    .tile span { display: block; font-size: 12px; line-height: 16px; color: #a9a29b; }
    .opts { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
    .opt { display: flex; align-items: center; justify-content: center; height: 64px; border-radius: 12px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.26); font-size: 16px; font-weight: 600; }
    .opt.on { border-color: #e24848; background: rgba(226,72,72,0.16); }
    .cap { font-size: 12px; line-height: 16px; color: #6b655f; }
    .grow { flex: 1; }
"""

MARK = '''<svg viewBox="0 0 28 28" width="28" height="28"><circle cx="14" cy="14" r="12.5" fill="none" stroke="#e24848" stroke-opacity="0.45" stroke-width="1.2"></circle><circle cx="14" cy="14" r="9.4" fill="none" stroke="#e24848" stroke-width="1.8"></circle><path d="M11 9.5h2.4a4.5 4.5 0 0 1 0 9H11z" fill="none" stroke="#ece7e2" stroke-width="2" stroke-linejoin="round"></path><path d="M9.6 12.6h4.6M9.6 15.4h4.6" stroke="#0d0508" stroke-width="1.2"></path></svg>'''

TICK = '<span class="g"><svg viewBox="0 0 16 16" width="14" height="14"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="#e24848" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path></svg></span>'
DOT = '<span class="g"><span style="width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16);"></span></span>'
CROSS = '<span class="g"><svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 4l8 8M12 4l-8 8" fill="none" stroke="#e24848" stroke-width="1.8" stroke-linecap="round"></path></svg></span>'
NONE = '<span class="g"></span>'
import random

def qr(seed=7, n=25, cell=4):
    rnd = random.Random(seed)
    cells = []
    def finder(x, y):
        for i in range(7):
            for j in range(7):
                edge = i in (0, 6) or j in (0, 6)
                core = 2 <= i <= 4 and 2 <= j <= 4
                if edge or core:
                    cells.append((x + j, y + i))
    finder(0, 0); finder(n - 7, 0); finder(0, n - 7)
    taken = set(cells)
    for y in range(n):
        for x in range(n):
            near = (x < 8 and y < 8) or (x >= n - 8 and y < 8) or (x < 8 and y >= n - 8)
            if near or (x, y) in taken:
                continue
            if rnd.random() < 0.44:
                cells.append((x, y))
    rects = "".join(f'<rect x="{x*cell}" y="{y*cell}" width="{cell}" height="{cell}"></rect>' for x, y in cells)
    return f'<svg class="qr" viewBox="0 0 {n*cell} {n*cell}" fill="#14070a">{rects}</svg>'

FOLDER = '<svg viewBox="0 0 16 16" width="14" height="14"><path d="M2 4.5a1.5 1.5 0 0 1 1.5-1.5h2.8l1.5 1.5H12.5A1.5 1.5 0 0 1 14 6v5.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5z" fill="none" stroke="#a9a29b" stroke-width="1.3"></path></svg>'

def line(kind, text, detail=""):
    glyph = {"done": TICK, "now": DOT, "bad": CROSS, "todo": NONE}[kind]
    d = f' <span class="d">· {detail}</span>' if detail else ""
    return f'<div class="line {kind}">{glyph}<span>{text}{d}</span></div>'

def head(glyph, text, count):
    return f'<div class="head">{glyph}<span>{text} <span class="n">· {count}</span></span></div>'

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
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Commissioner:wght@400;600&family=JetBrains+Mono:wght@400;700&display=swap">
  <style>{STYLE}  </style>
</helmet>
<div class="win">
  <div class="col">
    <div class="brand">{MARK}<span>Dossier</span></div>
{body}
  </div>
</div>
</x-dc>
</body>
</html>
'''

def ledger(lines):
    return '    <div class="ledger">\n' + "\n".join("      " + l for l in lines) + "\n    </div>"

def step(inner):
    return '    <div class="step">\n' + inner + '\n    </div>'

def card(*parts):
    return '  <div class="card">\n' + "\n".join(parts) + '\n  </div>'

EN = {
    "steps": ["Language", "osu! folder", "This device", "The bot"],
    "setting": "Setting up",
    "checking": "Checking",
    "works": "Everything works",
    "of": "of",
}
RU = {
    "steps": ["Язык", "Папка osu!", "Это устройство", "Бот"],
    "setting": "Настройка",
    "checking": "Проверка",
    "works": "Всё работает",
    "of": "из",
}

def setup_ledger(t, at):
    out = []
    for i, name in enumerate(t["steps"], start=1):
        kind = "done" if i < at else "now" if i == at else "todo"
        out.append(line(kind, name))
    return ledger(out)

def buttons(*btns):
    return '      <div class="row">' + "".join(btns) + "</div>"

def btn(text, kind="quiet"):
    return f'<span class="btn {kind}">{text}</span>'

GROW = '<span class="grow"></span>'

boards = {}

boards["Language"] = page(
    head(DOT, EN["setting"], f"1 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 1), step(
        '      <h2>Language</h2>\n'
        '      <p class="why">Every word after this will be in it.</p>\n'
        '      <div class="opts"><span class="opt on">English</span><span class="opt">Русский</span></div>\n'
        + buttons(GROW, btn("Continue", "primary"))
    ))
)

boards["Main"] = page(
    head(DOT, EN["setting"], f"2 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 2), step(
        '      <h2>osu! folder</h2>\n'
        '      <p class="why">Found on this device.</p>\n'
        f'      <div class="field"><div class="box">{FOLDER}<span>~/osu</span><span class="tag">stable</span></div></div>\n'
        '      <div class="tiles"><div class="tile"><b>1,342</b><span>maps</span></div><div class="tile"><b>14</b><span>skins</span></div><div class="tile"><b>187</b><span>replays</span></div></div>\n'
        + buttons(btn("Back"), GROW, btn("Add another…"), btn("Use this", "primary"))
    ))
)

boards["FolderLazer"] = page(
    head(DOT, EN["setting"], f"2 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 2), step(
        '      <h2>osu! folder</h2>\n'
        '      <p class="why">Found on this device.</p>\n'
        f'      <div class="field"><div class="box">{FOLDER}<span>~/.local/share/osu</span><span class="tag">lazer</span></div></div>\n'
        '      <div class="tiles"><div class="tile"><b>—</b><span>maps come from the mirrors</span></div><div class="tile"><b>3</b><span>skins, exported</span></div><div class="tile"><b>212</b><span>replays</span></div></div>\n'
        + buttons(btn("Back"), GROW, btn("Add another…"), btn("Use this", "primary"))
    ))
)

def src(tag, path, counts, on=True):
    return (f'<div class="src"><span class="tag">{tag}</span><span class="path">{path}</span>'
            f'<span class="n">{counts}</span><span class="switch{" on" if on else ""}"><i></i></span></div>')

boards["FolderBoth"] = page(
    head(DOT, EN["setting"], f"2 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 2), step(
        '      <h2>osu! folder</h2>\n'
        '      <p class="why">Found two on this device. Both are read; a switch leaves one out.</p>\n'
        '      <div style="display: flex; flex-direction: column; gap: 8px;">'
        + src("stable", "~/osu", "1,342 maps · 14 skins · 187 replays")
        + src("lazer", "~/.local/share/osu", "3 skins · 212 replays")
        + '</div>\n'
        + buttons(btn("Back"), GROW, btn("Add another…"), btn("Use both", "primary"))
    ))
)

boards["FolderMissing"] = page(
    head(DOT, EN["setting"], f"2 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 2), step(
        '      <h2>osu! folder</h2>\n'
        '      <p class="why">Couldn\'t find osu! on this device.</p>\n'
        '      <p class="cap" style="margin: 0;">Without it, maps the bot sends are kept in the application\'s own folder.</p>\n'
        + buttons(btn("Back"), GROW, btn("I\'ll only render for the bot"), btn("Browse…", "primary"))
    ))
)

boards["Device"] = page(
    head(DOT, EN["setting"], f"3 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 3), step(
        '      <h2>This device</h2>\n'
        '      <p class="why">This is how the farm will see it.</p>\n'
        '      <div class="field"><span class="cap">Name</span><div class="box"><span>MacBook Pro</span></div></div>\n'
        + buttons(btn("Back"), GROW, btn("Continue", "primary"))
    ))
)

def bot_step(t, title, why, code_cap, code, open_tg, wait, wait_ok, links, back, cont, linked=False):
    waiting = (f'<div class="wait ok">{TICK}<span>{wait_ok}</span></div>' if linked
               else f'<div class="wait">{DOT}<span>{wait}</span></div>')
    left = (f'<div style="display: flex; flex-direction: column; gap: 12px; flex: 1; min-width: 0;">'
            f'<div><span class="cap">{code_cap}</span><div class="code">{code}</div></div>'
            f'<div class="row">{btn(open_tg, "primary" if not linked else "quiet")}</div>'
            f'{waiting}</div>')
    return step(
        f'      <h2>{title}</h2>\n'
        f'      <p class="why">{why}</p>\n'
        f'      <div class="row" style="align-items: flex-start; gap: 24px;">{left}{qr()}</div>\n'
        '      <div class="links">' + "".join(f'<a href="#">{l}</a>' for l in links) + '</div>\n'
        + buttons(btn(back), GROW, btn(cont, "primary" if linked else "dim"))
    )

boards["Bot"] = page(
    head(DOT, EN["setting"], f"4 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 4), bot_step(EN, "The bot", "Scan, or open Telegram here, and press yes.",
        "Code · 10 minutes", "K7QN-M4XZ", "Open Telegram", "Waiting for Telegram…", "",
        ["I have a code from the bot", "Later — just my own replays"], "Back", "Continue"))
)

boards["BotLinked"] = page(
    head(DOT, EN["setting"], f"4 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 4), bot_step(EN, "The bot", "Scan, or open Telegram here, and press yes.",
        "Code · 10 minutes", "K7QN-M4XZ", "Open Telegram", "", "Linked to @naumredlo",
        ["I have a code from the bot", "Later — just my own replays"], "Back", "Continue", linked=True))
)

boards["BotOnly"] = page(
    head(DOT, EN["setting"], f"4 {EN['of']} 4") + "\n" +
    card(setup_ledger(EN, 4), bot_step(EN, "The bot", "This device renders for the bot, so it has to be linked.",
        "Code · 10 minutes", "K7QN-M4XZ", "Open Telegram", "Waiting for Telegram…", "",
        ["I have a code from the bot"], "Back", "Continue"))
)

boards["ChecksBotOnly"] = page(
    head(DOT, EN["checking"], f"1 {EN['of']} 3") + "\n" +
    card(ledger([
        line("bad", "ffmpeg", "not installed"),
        '<div class="sub">This device renders for the bot, and rendering needs it. <a href="#">Where to get it</a></div>',
        line("todo", "Engine"),
        line("todo", "Bot"),
    ]), step(buttons(GROW, btn("Check again", "primary"))))
)

boards["Checks"] = page(
    head(DOT, EN["checking"], f"3 {EN['of']} 4") + "\n" +
    card(ledger([
        line("done", "osu! folder", "1,342 maps"),
        line("done", "ffmpeg", "7.1"),
        line("now", "Engine", "0.11.0, asking the bot…"),
        line("todo", "Bot"),
    ]))
)

boards["ChecksFailed"] = page(
    head(DOT, EN["checking"], f"2 {EN['of']} 4") + "\n" +
    card(ledger([
        line("done", "osu! folder", "1,342 maps"),
        line("bad", "ffmpeg", "not installed"),
        '<div class="sub">Rendering needs it; judging does not. <a href="#">Where to get it</a></div>',
        line("todo", "Engine"),
        line("todo", "Bot"),
    ]), step(buttons(btn("Check again"), GROW, btn("Continue anyway", "primary"))))
)

boards["Done"] = page(
    head(TICK, EN["works"], f"4 {EN['of']} 4") + "\n" +
    card(ledger([
        line("done", "osu! folder", "1,342 maps"),
        line("done", "ffmpeg", "7.1"),
        line("done", "Engine", "0.11.0, same as the bot's"),
        line("done", "Bot", "41 ms"),
    ]), step(buttons(GROW, btn("Open Dossier", "primary"))))
)

boards["FolderRu"] = page(
    head(DOT, RU["setting"], f"2 {RU['of']} 4") + "\n" +
    card(setup_ledger(RU, 2), step(
        '      <h2>Папка osu!</h2>\n'
        '      <p class="why">Нашлась на этом устройстве.</p>\n'
        f'      <div class="field"><div class="box">{FOLDER}<span>~/osu</span><span class="tag">stable</span></div></div>\n'
        '      <div class="tiles"><div class="tile"><b>1 342</b><span>карты</span></div><div class="tile"><b>14</b><span>скинов</span></div><div class="tile"><b>187</b><span>реплеев</span></div></div>\n'
        + buttons(btn("Назад"), GROW, btn("Добавить ещё…"), btn("Взять эту", "primary"))
    ))
)

boards["BotRu"] = page(
    head(DOT, RU["setting"], f"4 {RU['of']} 4") + "\n" +
    card(setup_ledger(RU, 4), bot_step(RU, "Бот", "Отсканируйте или откройте Telegram здесь и нажмите «да».",
        "Код · 10 минут", "K7QN-M4XZ", "Открыть Telegram", "Жду Telegram…", "",
        ["У меня есть код от бота", "Позже — только свои реплеи"], "Назад", "Продолжить"))
)

boards["ChecksFailedRu"] = page(
    head(DOT, RU["checking"], f"2 {RU['of']} 4") + "\n" +
    card(ledger([
        line("done", "Папка osu!", "1 342 карты"),
        line("bad", "ffmpeg", "не установлен"),
        '<div class="sub">Нужен для рендера; для судейства — нет. <a href="#">Где взять</a></div>',
        line("todo", "Движок"),
        line("todo", "Бот"),
    ]), step(buttons(btn("Проверить снова"), GROW, btn("Продолжить без него", "primary"))))
)

for name, html in boards.items():
    (HERE / f"{name}.dc.html").write_text(html)

W, H, GX, GY = 980, 720, 80, 120
en = ["Language", "Main", "FolderLazer", "FolderBoth", "FolderMissing", "Device", "Bot", "BotLinked", "Checks", "ChecksFailed", "Done"]
only = ["BotOnly", "ChecksBotOnly"]
ru = ["FolderRu", "BotRu", "ChecksFailedRu"]
artboards = []
for i, name in enumerate(en):
    artboards.append({"file": f"{name}.dc.html", "x": (i % 6) * (W + GX), "y": (i // 6) * (H + GY), "w": W, "h": H, "page": "page-en"})
for i, name in enumerate(only):
    artboards.append({"file": f"{name}.dc.html", "x": i * (W + GX), "y": 2 * (H + GY), "w": W, "h": H, "page": "page-en"})
for i, name in enumerate(ru):
    artboards.append({"file": f"{name}.dc.html", "x": i * (W + GX), "y": 0, "w": W, "h": H, "page": "page-ru"})
canvas = {
    "pages": [{"id": "page-en", "name": "First run · EN"}, {"id": "page-ru", "name": "First run · RU"}],
    "artboards": artboards,
    "annotations": [
        {"id": "bot-only", "x": 0, "y": 2 * (H + GY) - 150, "w": 480, "page": "page-en",
         "text": "Bot-only device (chose “I'll only render for the bot” at the folder step): the steps stay, the ways out go. No “Later — just my own replays”; a missing ffmpeg has no “Continue anyway”."},
        {"id": "order", "x": 0, "y": -160, "w": 420, "page": "page-en",
         "text": "Order: Language → osu! folder → This device → The bot → Checks.\nWindow 980×720, column 560. Tokens from docs/design.md.\nThe headline and the card never move between steps; only the card's lower half changes.\nThe folder step reads osu!.<user>.cfg (stable) or storage.ini (lazer) instead of guessing. The bot step needs no address and no token: the QR opens the bot with the code; the QR here is a placeholder.\nSources: stable + lazer are both read by default; a switch leaves one out. The same list is Settings → Folders."},
    ],
    "launch": {"view": "canvas", "page": "page-en"},
}
(HERE / "canvas.json").write_text(json.dumps(canvas, ensure_ascii=False, indent=2) + "\n")
print("wrote", len(boards), "artboards")
