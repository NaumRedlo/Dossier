from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

CSS = """
  <style>
    .top { position: absolute; top: 22px; left: 40px; right: 40px; height: 36px; display: flex; align-items: center; }
    .top .crest { display: flex; align-items: center; gap: 12px; font-size: 18px; font-weight: 600; color: #ece7e2; }
    .top .crest i { display: block; width: 26px; height: 26px; border-radius: 6px; background: linear-gradient(135deg, #e24848, #7a1c1c); }
    .top .crest s { display: block; width: 1px; height: 22px; background: rgba(255,255,255,0.12); }
    .words { margin-left: auto; display: flex; align-items: center; gap: 22px; font-weight: 600; color: #a9a29b; }
    .words b { color: #ece7e2; }
    .words .login { display: inline-flex; align-items: center; gap: 8px; height: 30px; padding: 0 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); background: rgba(0,0,0,0.25); color: #ece7e2; font-weight: 600; font-size: 13px; }
    .words .login .tg { width: 14px; height: 14px; border-radius: 50%; background: #e24848; flex: none; }
    .words .ava { width: 28px; height: 28px; border-radius: 50%; background: linear-gradient(135deg, #3a1015, #e24848); flex: none; box-shadow: 0 0 0 2px rgba(255,255,255,0.06); }
    .words .ava.on { box-shadow: 0 0 0 2px rgba(226,72,72,0.5); }
    .words .ava.none, .menu .head .ava.none { background: repeating-linear-gradient(135deg, rgba(169,162,155,0.55) 0 1.5px, rgba(20,9,12,0.9) 1.5px 5px); box-shadow: 0 0 0 1px rgba(255,255,255,0.1); }
    .menu .tabs { display: flex; gap: 18px; margin: 14px 0 4px; font-weight: 600; color: #6b655f; font-size: 13px; }
    .menu .tabs span { padding-bottom: 6px; border-bottom: 2px solid transparent; }
    .menu .tabs span.on { color: #ece7e2; border-bottom-color: #e24848; }
    .menu .state { display: flex; align-items: center; gap: 10px; margin-top: 12px; color: #a9a29b; }
    .menu .kv { display: flex; align-items: center; height: 26px; color: #a9a29b; }
    .menu .kv .n { margin-left: auto; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .vlist { position: absolute; left: 40px; right: 40px; top: 92px; }
    .vlist .r { display: grid; grid-template-columns: 72px 70px 1fr 90px 50px 64px 60px; gap: 14px; align-items: center; height: 52px; border-bottom: 1px solid rgba(255,255,255,0.05); color: #ece7e2; font-size: 13px; }
    .vlist .r .th { width: 64px; height: 36px; border-radius: 5px; background: #0a0507 center/cover; border: 1px solid rgba(255,255,255,0.1); opacity: 0.8; }
    .vlist .r.on .th { border-color: #e24848; opacity: 1; }
    .vlist .r .t { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .vlist .r .m { color: #a9a29b; }
    .vlist .r .n { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; text-align: right; }
    .vlist .r .mods { display: flex; gap: 4px; }
    .vlist .r .mods i { display: inline-block; padding: 0 4px; border-radius: 3px; background: #e24848; color: #fff; font-style: normal; font-weight: 700; font-size: 9px; line-height: 14px; }
    .vlist .r .sent { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; text-align: right; }
    .vlist .head { display: grid; grid-template-columns: 72px 70px 1fr 90px 50px 64px 60px; gap: 14px; height: 24px; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; letter-spacing: 0.06em; text-transform: uppercase; align-items: center; }
    .vlist .head .n { text-align: right; }
    .stage { position: absolute; left: 40px; right: 40px; top: 36px; bottom: 36px; border-radius: 12px; background: rgba(12,5,7,0.98); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 30px 80px rgba(0,0,0,0.6); overflow: hidden; }
    .stage .vid { position: absolute; left: 0; right: 0; top: 0; height: 548px; background: #070304 url(frame.jpg) center/cover; }
    .stage .vid.paused::before { content: ""; position: absolute; left: 50%; top: 50%; border-left: 30px solid rgba(236,231,226,0.92); border-top: 18px solid transparent; border-bottom: 18px solid transparent; transform: translate(-40%, -50%); filter: drop-shadow(0 6px 18px rgba(0,0,0,0.6)); }
    .stage .vid.playing::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(7,3,4,0) 70%, rgba(7,3,4,0.7) 100%); }
    .stage .tl { position: absolute; left: 24px; right: 24px; top: 522px; display: flex; align-items: center; gap: 12px; }
    .stage .tl .tr { flex: 1; height: 3px; background: rgba(255,255,255,0.14); position: relative; border-radius: 2px; }
    .stage .tl .tr i { position: absolute; left: 0; top: 0; bottom: 0; background: #ece7e2; border-radius: 2px; }
    .stage .tl .tr b { position: absolute; top: -4px; width: 11px; height: 11px; border-radius: 50%; background: #ece7e2; margin-left: -5px; }
    .stage .tl .tm { color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; text-shadow: 0 1px 4px rgba(0,0,0,0.8); }
    .stage .under { position: absolute; left: 24px; right: 24px; top: 570px; display: flex; align-items: center; gap: 12px; white-space: nowrap; }
    .stage .under .who { font-weight: 700; font-size: 18px; color: #ece7e2; }
    .stage .under .map { color: #a9a29b; font-size: 13px; flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
    .stage .under .n { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .stage .under .acts { position: static; margin-left: auto; flex: none; }
    .stage .x { position: absolute; right: 18px; top: 14px; color: rgba(236,231,226,0.7); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; text-shadow: 0 1px 4px rgba(0,0,0,0.8); }
    .cover { position: absolute; top: 14px; right: 30px; width: 320px; height: 52px; background: #0d0508; }
    .menu { position: absolute; top: 62px; right: 40px; width: 400px; border-radius: 12px; background: rgba(20,9,12,0.97); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 24px 60px rgba(0,0,0,0.55); box-sizing: border-box; padding: 16px 18px; font-size: 13px; color: #ece7e2; }
    .menu .head { display: flex; align-items: center; gap: 12px; padding-bottom: 12px; border-bottom: 1px solid rgba(255,255,255,0.06); }
    .menu .head .ava { width: 40px; height: 40px; border-radius: 50%; background: linear-gradient(135deg, #3a1015, #e24848); flex: none; }
    .menu .head .nm { font-size: 15px; font-weight: 600; }
    .menu .head .hd { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; margin-top: 2px; }
    .menu h3 { margin: 12px 0 6px; font-size: 11px; line-height: 14px; letter-spacing: 0.08em; text-transform: uppercase; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 400; }
    .menu .row { display: flex; align-items: center; gap: 10px; height: 24px; white-space: nowrap; }
    .menu .row .n { margin-left: auto; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .menu .row .t { width: 40px; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .menu .row .g { width: 12px; text-align: center; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; color: #a9a29b; }
    .menu .row.bad, .menu .row.bad .g { color: #e24848; }
    .menu .row.done { color: #a9a29b; }
    .menu .row .d { display: block; width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); flex: none; margin: 0 2px; }
    .menu .pb { height: 2px; border-radius: 1px; background: rgba(255,255,255,0.08); position: relative; margin: 2px 0 8px; }
    .menu .pb i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 1px; background: #e24848; }
    .menu .foot { display: flex; align-items: center; gap: 14px; margin-top: 14px; padding-top: 12px; border-top: 1px solid rgba(255,255,255,0.06); font-size: 12px; color: #a9a29b; }
    .menu .foot .quit { margin-left: auto; color: #6b655f; }
    .menu a { color: #e24848; text-decoration: none; font-weight: 600; }
    .veil { position: absolute; inset: 0; background: rgba(7,3,4,0.72); }
    .card { position: absolute; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; background: rgba(28,14,18,0.96); box-shadow: 0 18px 44px rgba(0,0,0,0.5); box-sizing: border-box; padding: 24px; color: #ece7e2; }
    .card .ttl { font-size: 16px; font-weight: 600; }
    .card .cap { color: #a9a29b; font-size: 12px; line-height: 16px; margin-top: 6px; }
    .card .code { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 26px; font-weight: 700; letter-spacing: 0.18em; margin: 18px 0 4px; }
    .card .wait { display: flex; align-items: center; gap: 8px; margin-top: 14px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .card .wait i { display: block; width: 6px; height: 6px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.16); }
    .card .qr { position: absolute; right: 24px; top: 24px; width: 120px; height: 120px; border-radius: 8px; background: repeating-linear-gradient(90deg, #ece7e2 0 6px, transparent 6px 12px), repeating-linear-gradient(0deg, #ece7e2 0 6px, #2a1418 6px 12px); opacity: 0.85; }
    .card .btns { display: flex; align-items: center; gap: 8px; margin-top: 18px; }
    .btn { display: inline-flex; align-items: center; height: 32px; padding: 0 12px; border-radius: 8px; font-weight: 600; font-size: 14px; box-sizing: border-box; white-space: nowrap; }
    .btn.primary { background: #e24848; color: #fff; }
    .btn.quiet { color: #a9a29b; }
    .btn.soft { background: rgba(226,72,72,0.10); border: 1px solid rgba(226,72,72,0.18); color: #ece7e2; position: relative; overflow: hidden; width: 150px; justify-content: center; }
    .btn.soft i { position: absolute; left: 0; top: 0; bottom: 0; background: rgba(226,72,72,0.16); }
    .btn.soft u { position: absolute; left: 3px; bottom: 2px; height: 2px; border-radius: 1px; background: #e24848; }
    .btn.soft span { position: relative; }
    .player { position: absolute; left: 0; right: 0; top: 0; height: 560px; background: #070304 url(frame.jpg) center/cover; }
    .player::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(7,3,4,0.96) 0%, rgba(7,3,4,0.55) 24%, rgba(7,3,4,0.35) 50%, rgba(7,3,4,0.9) 82%, rgba(7,3,4,0.99) 100%); }
    .player.paused::before { content: ""; position: absolute; left: 50%; top: 46%; width: 0; height: 0; border-left: 26px solid rgba(236,231,226,0.9); border-top: 16px solid transparent; border-bottom: 16px solid transparent; transform: translate(-40%, -50%); z-index: 1; filter: drop-shadow(0 6px 18px rgba(0,0,0,0.6)); }
    .timeline { position: absolute; left: 40px; right: 40px; top: 470px; display: flex; align-items: center; gap: 14px; z-index: 1; }
    .timeline .tr { flex: 1; height: 2px; background: rgba(255,255,255,0.1); position: relative; }
    .timeline .tr i { position: absolute; left: 0; top: 0; bottom: 0; background: #ece7e2; }
    .timeline .tr b { position: absolute; top: -4px; width: 10px; height: 10px; border-radius: 50%; background: #ece7e2; margin-left: -5px; }
    .timeline .tm { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .meta { position: absolute; left: 40px; top: 372px; z-index: 1; color: #ece7e2; }
    .meta .when { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .meta .who { font-size: 26px; font-weight: 700; margin-top: 2px; }
    .meta .map { color: #a9a29b; font-size: 14px; margin-top: 2px; }
    .meta .how { display: flex; align-items: center; gap: 8px; margin-top: 6px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .meta .how .mod { display: inline-block; padding: 0 5px; border-radius: 4px; background: #e24848; color: #fff; font-weight: 700; font-size: 10px; line-height: 16px; }
    .acts { position: absolute; left: 40px; top: 508px; display: flex; align-items: center; gap: 10px; z-index: 1; }
    .big { position: absolute; right: 40px; top: 420px; text-align: right; z-index: 1; }
    .big .n { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 28px; font-weight: 700; color: #ece7e2; }
    .big .c { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; margin-top: 2px; }
    .rail { position: absolute; left: 40px; right: 40px; top: 598px; display: flex; align-items: center; gap: 14px; }
    .rail .tr { flex: 1; height: 2px; background: rgba(255,255,255,0.06); position: relative; }
    .rail .tr i { position: absolute; left: 0; top: 0; bottom: 0; width: 22%; background: rgba(169,162,155,0.55); border-radius: 1px; }
    .rail .cnt { color: rgba(107,101,95,0.9); font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .journal { position: absolute; left: 40px; right: 40px; top: 612px; display: flex; align-items: flex-end; gap: 22px; overflow: hidden; }
    .journal .day { display: flex; flex-direction: column; gap: 6px; }
    .journal .day .lb { display: flex; align-items: center; gap: 6px; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .journal .day .lb i { display: block; width: 5px; height: 5px; border-radius: 50%; background: #6b655f; }
    .journal .day.on .lb { color: #ece7e2; }
    .journal .day.on .lb i { background: #e24848; }
    .journal .fr { display: flex; gap: 6px; align-items: flex-end; }
    .journal .fr .f { width: 108px; height: 61px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); background: #0a0507 center/cover; opacity: 0.6; position: relative; box-sizing: border-box; }
    .journal .fr .f.on { width: 116px; height: 65px; border: 2px solid #e24848; opacity: 1; }
    .journal .fr .f .len { position: absolute; right: 5px; bottom: 4px; padding: 0 4px; border-radius: 3px; background: rgba(7,3,4,0.7); color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; line-height: 14px; }
    .grid { position: absolute; left: 40px; right: 40px; top: 92px; display: grid; grid-template-columns: repeat(4, 1fr); gap: 18px; }
    .grid .v { border-radius: 12px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.25); overflow: hidden; }
    .grid .v.on { border-color: #e24848; }
    .grid .v .th { height: 118px; background: #0a0507 center/cover; position: relative; }
    .grid .v .th .len { position: absolute; right: 8px; bottom: 6px; padding: 0 5px; border-radius: 3px; background: rgba(7,3,4,0.7); color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; line-height: 16px; }
    .grid .v .tx { padding: 10px 12px 12px; }
    .grid .v .tx .p { font-weight: 600; color: #ece7e2; font-size: 13px; }
    .grid .v .tx .m { color: #a9a29b; font-size: 12px; margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .grid .v .tx .w { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; margin-top: 6px; }
    .list { position: absolute; left: 40px; top: 92px; width: 520px; }
    .list .r { display: grid; grid-template-columns: 64px 1fr 60px 64px; gap: 12px; align-items: center; height: 40px; border-bottom: 1px solid rgba(255,255,255,0.05); color: #ece7e2; font-size: 13px; }
    .list .r.on { color: #ece7e2; }
    .list .r .t { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .list .r .m { color: #a9a29b; }
    .list .r.on .m { color: #ece7e2; }
    .list .r .n { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; text-align: right; }
    .side { position: absolute; left: 592px; right: 40px; top: 92px; }
    .side .pv { height: 196px; border-radius: 12px; background: #0a0507 url(frame.jpg) center/cover; border: 1px solid rgba(255,255,255,0.08); position: relative; }
    .side .pv::before { content: ""; position: absolute; left: 50%; top: 50%; border-left: 20px solid rgba(236,231,226,0.9); border-top: 12px solid transparent; border-bottom: 12px solid transparent; transform: translate(-40%, -50%); }
    .side .tt { margin-top: 12px; font-weight: 600; color: #ece7e2; font-size: 15px; }
    .side .mm { color: #a9a29b; font-size: 12px; margin-top: 2px; }
    .side .ww { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; margin-top: 6px; }
    .side .acts { position: static; margin-top: 14px; }
    .empty { position: absolute; left: 40px; top: 300px; right: 40px; text-align: center; color: #a9a29b; }
    .empty .h { font-size: 18px; font-weight: 600; color: #ece7e2; }
    .empty .c { margin-top: 6px; font-size: 13px; }
    .empty a { color: #e24848; text-decoration: none; font-weight: 600; }
    .note { position: absolute; left: 40px; font-size: 12px; line-height: 16px; color: #a9a29b; }
    .toast { position: absolute; left: 40px; bottom: 120px; width: 360px; padding: 12px 14px; border-radius: 12px; border: 1px solid rgba(255,255,255,0.08); background: rgba(20,9,12,0.96); box-shadow: 0 14px 36px rgba(0,0,0,0.5); box-sizing: border-box; color: #ece7e2; z-index: 2; }
    .toast .h { display: flex; align-items: center; gap: 8px; font-weight: 600; }
    .toast .h .v { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .toast .b { margin-top: 3px; color: #a9a29b; font-size: 12px; line-height: 16px; }
  </style>"""

BG = ["bg-astral.jpg", "bg-zenith.jpg", "bg-freedom.jpg", "bg-galactic.jpg", "bg-lucky.jpg", "bg-nevermind.jpg", "bg-sink.jpg"]

def top(words, right, crest=True):
    head = '<div class="crest"><i></i><s></s>Dossier</div>' if crest else ''
    return f'<div class="top">{head}<div class="words">{words}{right}</div></div>'

WORDS3 = '<b>Реплеи</b><span>Воркер</span><span>Настройки</span>'
WORDS4 = '<span>Реплеи</span><b>Видео</b><span>Воркер</span><span>Настройки</span>'
WORDS4_REST = '<b>Реплеи</b><span>Видео</span><span>Воркер</span><span>Настройки</span>'
LOGIN = '<span class="login"><span class="tg"></span>Войти через Telegram</span>'
NONE = '<span class="ava none"></span>'
AVATAR = '<span class="ava"></span>'
AVATAR_ON = '<span class="ava on"></span>'

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{CSS}<div class="cover"></div>{extra}')

def own(extra):
    return page(W, H, f'{CSS}<div style="position:absolute; inset:0; background:#0d0508;"></div>{extra}')

def journal(chosen=0):
    days = [("сегодня", 1), ("14 авг", 2), ("3 авг", 1), ("29 июл", 3), ("10 июл", 2)]
    lens = ["3:51", "2:14", "4:02", "1:48", "3:20", "2:57", "5:11", "2:02", "3:33"]
    out, at = [], 0
    for label, count in days:
        frames = []
        for _ in range(count):
            on = " on" if at == chosen else ""
            frames.append(f'<div class="f{on}" style="background-image:url({BG[at % len(BG)]});"><span class="len">{lens[at]}</span></div>')
            at += 1
        on_day = " on" if at - count <= chosen < at else ""
        out.append(f'<div class="day{on_day}"><div class="lb"><i></i>{label}</div><div class="fr">{"".join(frames)}</div></div>')
    return f'<div class="rail"><div class="tr"><i></i></div><span class="cnt">1 / 9</span></div><div class="journal">{"".join(out)}</div>'

def mirror(acts, big='<div class="big"><div class="n">1920×1080</div><div class="c">60 fps · 84,2 МБ</div></div>', paused=True, extra=""):
    when = 'сегодня · 14:12 · из реплея 13:03'
    meta = f'''<div class="meta"><div class="when">{when}</div><div class="who">NaumRedlo</div><div class="map">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</div>
      <div class="how"><span class="mod">HD</span><span class="mod">DT</span><span class="mod">HR</span>· 604x · 3:51 · 95,00 % · FC</div></div>'''
    timeline = '<div class="timeline"><span class="tm">1:07</span><div class="tr"><i style="width:29%;"></i><b style="left:29%;"></b></div><span class="tm">3:51</span></div>'
    cls = "player paused" if paused else "player"
    return own(f'<div class="{cls}"></div>{top(WORDS4, AVATAR)}{meta}{timeline}<div class="acts">{acts}</div>{big}{journal()}{extra}')

def stage(state="paused", acts=None, extra=""):
    acts = acts or '<span class="btn primary">В Telegram</span><span class="btn quiet">В папке</span><span class="btn quiet">Удалить</span>'
    tl = '<div class="tl"><span class="tm">1:07</span><div class="tr"><i style="width:29%;"></i><b style="left:29%;"></b></div><span class="tm">3:51</span></div>'
    under = f'<div class="under"><span class="who">NaumRedlo</span><span class="map">Dj Grimoire — Astral Quantization [Nattu VN0TH3R]</span><span class="n">HD DT HR · 1920×1080 · 84,2 МБ</span><div class="acts">{acts}</div></div>'
    return f'<div class="veil"></div><div class="stage"><div class="vid {state}"></div>{tl}{under}<span class="x">Esc</span></div>{extra}'

PEOPLE = [("NaumRedlo", "Dj Grimoire — Astral Quantization [Nattu VN0TH3R]", "HD DT HR", "3:51", "84,2 МБ", "сегодня", "14:12", ""), ("Guest", "xi — Blue Zenith [FOUR DIMENSIONS]", "EZ", "2:14", "51,0 МБ", "14 авг", "18:40", "✓ TG"), ("Lopuhh", "xi — FREEDOM DiVE [FOUR DIMENSIONS]", "", "4:02", "97,7 МБ", "14 авг", "11:02", "✓ TG"), ("Saki-chan", "Camellia — GALACTIC [Extra]", "HR", "1:48", "40,3 МБ", "3 авг", "22:15", ""), ("NaumRedlo", "Chocofan — LUCKY CAT [_]", "DT", "3:20", "72,9 МБ", "29 июл", "09:31", "✓ TG"), ("Deeo_XD", "Nevermind [Insane]", "", "2:57", "60,1 МБ", "29 июл", "09:10", ""), ("Guest", "Sink [Expert]", "HD", "5:11", "118,4 МБ", "29 июл", "08:52", ""), ("NaumRedlo", "Dj Grimoire — Astral Quantization [Nattu VN0TH3R]", "HD", "2:02", "44,0 МБ", "10 июл", "17:44", "✓ TG")]

def video_list(chosen=None):
    rows = ['<div class="head"><span></span><span>когда</span><span>кто · карта</span><span>моды</span><span class="n">длина</span><span class="n">размер</span><span class="n">telegram</span></div>']
    for i, (p, m, mods, l, s, d, tm, sent) in enumerate(PEOPLE):
        on = " on" if i == chosen else ""
        badges = "".join(f"<i>{x}</i>" for x in mods.split())
        rows.append(f'<div class="r{on}"><span class="th" style="background-image:url({BG[i % len(BG)]});"></span><span class="t">{d}<br>{tm}</span><span><b>{p}</b> <span class="m">· {m}</span></span><span class="mods">{badges}</span><span class="n">{l}</span><span class="n">{s}</span><span class="sent">{sent}</span></div>')
    return f'<div class="vlist">{"".join(rows)}</div>'

def boards():
    rest = frame("main-rest-ru-RU")
    out = {}

    out["TopGuest"] = over(rest, f'{top(WORDS3, NONE, crest=False)}<div class="note" style="top:64px; left:auto; right:40px; width:300px; text-align:right;">до входа — круг в серую штриховку, как кадр без карты: место есть, человека нет</div>')
    out["TopAvatar"] = over(rest, f'{top(WORDS3, AVATAR, crest=False)}')
    out["TopAvatarLive"] = over(rest, f'{top(WORDS3, AVATAR_ON, crest=False)}<div class="note" style="top:64px; left:auto; right:40px; width:300px; text-align:right;">кольцо вокруг аватара — что-то идёт: рендер, карта, отправка, работа воркера; гаснет, когда всё сделано</div>')

    out["LoginCard"] = over(rest, f'''{top(WORDS3, NONE, crest=False)}<div class="veil"></div>
      <div class="card" style="left:210px; top:200px; width:560px;">
        <div class="ttl">Войти через Telegram</div>
        <div class="cap" style="padding-right:150px;">Откройте бота и нажмите Start — он получит этот код и привяжет устройство. Готовые видео будут уходить в ваш чат.</div>
        <div class="code">4F7K-2M</div>
        <div class="cap">действует 5 минут</div>
        <div class="qr"></div>
        <div class="wait"><i></i>жду подтверждения…</div>
        <div class="btns"><span class="btn primary">Открыть Telegram</span><span class="btn quiet">Скопировать ссылку</span><span style="flex:1;"></span><span class="btn quiet">Позже</span></div>
      </div>''')

    guest = '''
      <div class="head"><span class="ava none"></span><div><div class="nm">Вход не выполнен</div><div class="hd">видео остаются на этом компьютере</div></div></div>
      <div class="tabs"><span class="on">Аккаунт</span><span>Лента</span></div>
      <div class="state">Привяжите устройство к Telegram — готовые видео будут уходить в ваш чат, а воркер сможет брать работу от вашего имени.</div>
      <div class="btns" style="display:flex; gap:8px; margin-top:14px;"><span class="btn primary">Войти через Telegram</span></div>
      <h3>Это устройство</h3>
      <div class="kv"><span>Реплеев в журнале</span><span class="n">187</span></div>
      <div class="kv"><span>Видео</span><span class="n">8 · 568 МБ</span></div>
      <div class="kv"><span>Сборка</span><span class="n">0.12.0</span></div>'''
    out["MenuGuest"] = over(rest, f'{top(WORDS3, NONE, crest=False)}<div class="menu">{guest}</div>')

    account = '''
      <div class="head"><span class="ava"></span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo · привязан 12 сен</div></div></div>
      <div class="tabs"><span class="on">Аккаунт</span><span>Лента</span></div>
      <div class="kv"><span>Видео уходят в чат</span><span class="n">@naumredlo</span></div>
      <div class="kv"><span>Воркер</span><span class="n">готов · 2 в очереди · 14 сегодня</span></div>
      <div class="kv"><span>Отправлено за месяц</span><span class="n">23 · 1,9 ГБ</span></div>
      <h3>Это устройство</h3>
      <div class="kv"><span>Реплеев в журнале</span><span class="n">187</span></div>
      <div class="kv"><span>Видео</span><span class="n">8 · 568 МБ</span></div>
      <div class="kv"><span>Сборка</span><span class="n">0.12.0 · та же, что у бота</span></div>
      <div class="foot"><span>Esc — закрыть</span><span class="quit">Выйти</span></div>'''
    out["MenuAccount"] = over(rest, f'{top(WORDS3, AVATAR, crest=False)}<div class="menu">{account}</div>')

    story = '''
      <div class="head"><span class="ava"></span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo · привязан 12 сен</div></div></div>
      <div class="tabs"><span>Аккаунт</span><span class="on">Лента</span></div>
      <div style="margin-top:8px;">
      <div class="row"><span class="t">14:03</span><span class="d"></span><b>Рисую</b><span style="color:#6b655f;">· Daisuke · 62 %</span></div>
      <div class="pb"><i style="width:62%;"></i></div>
      <div class="row"><span class="t">14:03</span><span class="d"></span><b>Отправляю</b><span style="color:#6b655f;">· Astral Quantization · 37 %</span></div>
      <div class="pb"><i style="width:37%;"></i></div>
      <div class="row bad"><span class="t">14:02</span><span class="g">✕</span><span>Рендер не завершился</span><span style="color:#6b655f;">· <a href="#">Ещё раз</a></span></div>
      <div class="row done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана</span><span style="color:#6b655f;">· Blue Zenith</span></div>
      <div class="row done"><span class="t">12:20</span><span class="g">✓</span><span>Видео в Telegram</span><span style="color:#6b655f;">· FREEDOM DiVE</span></div>
      <div class="row done"><span class="t">12:04</span><span class="g">✓</span><span>Воркер взял работу</span><span style="color:#6b655f;">· @friend</span></div>
      <div class="row done"><span class="t">09:00</span><span class="g">·</span><span style="color:#6b655f;">Открыто · сборка 0.12.0</span></div>
      </div>
      <div class="foot"><span>Esc — закрыть</span><span class="quit">Всё прочитано</span></div>'''
    out["MenuStory"] = over(rest, f'{top(WORDS3, AVATAR_ON, crest=False)}<div class="menu">{story}</div>')

    guest_story = '''
      <div class="head"><span class="ava none"></span><div><div class="nm">Вход не выполнен</div><div class="hd">лента этого устройства</div></div></div>
      <div class="tabs"><span>Аккаунт</span><span class="on">Лента</span></div>
      <div style="margin-top:8px;">
      <div class="row"><span class="t">14:03</span><span class="d"></span><b>Рисую</b><span style="color:#6b655f;">· Daisuke · 62 %</span></div>
      <div class="pb"><i style="width:62%;"></i></div>
      <div class="row done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана</span><span style="color:#6b655f;">· Blue Zenith</span></div>
      <div class="row done"><span class="t">13:20</span><span class="g">✓</span><span>Отрендерено</span><span style="color:#6b655f;">· LUCKY CAT · 72,9 МБ</span></div>
      <div class="row done"><span class="t">09:00</span><span class="g">·</span><span style="color:#6b655f;">Открыто · сборка 0.12.0</span></div>
      </div>
      <div class="foot"><span>Esc — закрыть</span><span class="quit">Всё прочитано</span></div>'''
    out["MenuGuestStory"] = over(rest, f'{top(WORDS3, NONE, crest=False)}<div class="menu">{guest_story}</div>')

    out["VideoList"] = own(f'{top(WORDS4, AVATAR)}{video_list()}<div class="note" style="top:640px;">клик по строке открывает плеер на весь экран; правая кнопка — В папке · В Telegram · Удалить; ✓ TG — уже отправлено</div>')
    out["VideoOpen"] = own(f'{top(WORDS4, AVATAR)}{video_list(0)}{stage("paused")}')
    out["VideoPlaying"] = own(f'{top(WORDS4, AVATAR)}{video_list(0)}{stage("playing")}')
    sending = '<span class="btn soft"><i style="width:37%;"></i><u style="width:37%;"></u><span>Отправляю</span></span><span class="btn quiet">В папке</span><span class="btn quiet">Удалить</span>'
    out["VideoSending"] = own(f'{top(WORDS4, AVATAR_ON)}{video_list(0)}{stage("paused", sending)}')
    toast = '<div class="toast" style="left:64px; bottom:60px; z-index:3;"><div class="h"><span class="v">✓</span>Ушло в Telegram</div><div class="b">@naumredlo · 84,2 МБ · 14:14</div></div>'
    out["VideoSent"] = own(f'{top(WORDS4, AVATAR)}{video_list(0)}{stage("paused", extra=toast)}')
    del_card = '''<div class="veil" style="z-index:3;"></div><div class="card" style="left:290px; top:260px; width:400px; z-index:4;">
        <div class="ttl">Удалить видео?</div>
        <div class="cap">NaumRedlo — Astral Quantization · 84,2 МБ. Файл уйдёт в корзину; реплей и карта останутся.</div>
        <div class="btns"><span style="flex:1;"></span><span class="btn quiet">Оставить</span><span class="btn primary">Удалить</span></div>
      </div>'''
    out["VideoDelete"] = own(f'{top(WORDS4, AVATAR)}{video_list(0)}{stage("paused", extra=del_card)}')
    out["VideoEmpty"] = own(f'{top(WORDS4, AVATAR)}<div class="empty"><div class="h">Пока ни одного видео</div><div class="c">Отрендерите реплей — оно появится здесь. <a href="#">К реплеям</a></div></div>')

    return out

NOTES = {
    "TopGuest": ("Справа · без входа", "Круг в серую штриховку — как кадр без карты: место человека есть, человека нет. Клик открывает то же меню, что и аватар."),
    "TopAvatar": ("Справа · аватар", "После входа круг становится аватаром из Telegram, 28 px, без имени — имя в меню."),
    "TopAvatarLive": ("Справа · аватар с кольцом", "Кольцо цвета акцента вокруг аватара, пока что-то идёт: рендер, карта, отправка, работа воркера. Одна деталь вместо полосы и точки; гаснет сама."),
    "LoginCard": ("Вход · карточка", "Из кнопки в меню: карточка с кодом и QR. «Открыть Telegram» ведёт на t.me/бот?start=код; бот отвечает «Привязано», приложение ждёт ответа, аватар проявляется на месте штриховки. Никаких паролей — только код, который живёт пять минут."),
    "MenuGuest": ("Меню · без входа · Аккаунт", "Шапка говорит «Вход не выполнен», ниже две вкладки: Аккаунт и Лента. На Аккаунте — зачем входить и кнопка входа, под ней это устройство: реплеи, видео, сборка."),
    "MenuGuestStory": ("Меню · без входа · Лента", "Лента работает и без входа — это дела и события устройства, не аккаунта."),
    "MenuAccount": ("Меню · Аккаунт", "После входа: имя, @handle, дата привязки; куда уходят видео, воркер, сколько отправлено; это устройство; в подвале Выйти."),
    "MenuStory": ("Меню · Лента", "Вторая вкладка: одна лента дня — время · знак · слово · деталь, дела сверху с полосами хода, ошибка красным со своим «Ещё раз». В подвале «Всё прочитано»."),
    "VideoList": ("Видео · список", "Четвёртое слово — Видео. Реестр: кадр, когда, кто · карта, моды, длина, размер, отправлено ли в Telegram. Новое сверху."),
    "VideoOpen": ("Видео · плеер", "Клик по строке — плеер на почти весь экран: видео 16:9, под ним ползунок и время, ниже имя, карта, моды · разрешение · размер и кнопки В Telegram · В папке · Удалить. Esc или клик мимо — назад к списку."),
    "VideoPlaying": ("Видео · играет", "Пробел — пауза, стрелки — на 5 с, двойной клик — на весь экран. Пока играет, ползунок и время видны, остальное чуть гаснет."),
    "VideoSending": ("Видео · отправка", "В Telegram — та же кнопка-прогресс, что у рендера: одно слово и заливка на всю ширину, по байтам. Клик — остановить. Аватар получает кольцо."),
    "VideoSent": ("Видео · отправлено", "Кнопка возвращается к «В Telegram», рядом ничего не остаётся; карточка внизу слева говорит, куда ушло, и уходит сама через 6 с; в списке у строки появляется ✓ TG."),
    "VideoDelete": ("Видео · удаление", "Единственный вопрос на этом экране: удалить — в корзину, реплей и карта остаются."),
    "VideoEmpty": ("Видео · пусто", "Пока ничего не отрендерено: одна строка и ссылка к реплеям."),
}

RECOMMENDED = "Принято 2026-09-21: штрихованный круг до входа, аватар с кольцом после; меню из круга с вкладками Аккаунт · Лента, вход — кнопкой в меню; видео — список, клик открывает плеер на почти весь экран, оттуда же отправка в Telegram."

TO_BUILD = "Что строить, по порядку: (1) хранилище видео — ~/.dossier/Renders с индексом videos.json: реплей, карта, размер, длительность, разрешение, время, отправлено ли; (2) экран Видео: список и плеер (кадры через ffmpeg в текстуру, звук через cpal); (3) круг справа и меню с вкладками, очередь уведомлений для ленты (~/.dossier/notices.json); (4) вход через Telegram: код, t.me/бот?start=код, ожидание ответа бота, аватар из getUserProfilePhotos; (5) отправка видео ботом: POST /render/send, прогресс по байтам в кнопке, карточка «Ушло»; (6) кольцо на аватаре, пока что-то идёт."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "store boards")
