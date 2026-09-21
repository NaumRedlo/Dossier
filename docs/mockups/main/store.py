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

def boards():
    rest = frame("main-rest-ru-RU")
    out = {}

    out["TopLogin"] = over(rest, f'{top(WORDS3, LOGIN, crest=False)}')
    out["TopAvatar"] = over(rest, f'{top(WORDS3, AVATAR, crest=False)}')
    out["TopAvatarLive"] = over(rest, f'{top(WORDS3, AVATAR_ON, crest=False)}<div class="note" style="top:64px; left:auto; right:40px; width:300px; text-align:right;">кольцо вокруг аватара — что-то идёт: рендер, карта, работа воркера; гаснет, когда всё сделано</div>')

    out["LoginCard"] = over(rest, f'''{top(WORDS3, LOGIN, crest=False)}<div class="veil"></div>
      <div class="card" style="left:210px; top:200px; width:560px;">
        <div class="ttl">Войти через Telegram</div>
        <div class="cap" style="padding-right:150px;">Откройте бота и нажмите Start — он получит этот код и привяжет устройство. Готовые видео будут уходить в ваш чат.</div>
        <div class="code">4F7K-2M</div>
        <div class="cap">действует 5 минут</div>
        <div class="qr"></div>
        <div class="wait"><i></i>жду подтверждения…</div>
        <div class="btns"><span class="btn primary">Открыть Telegram</span><span class="btn quiet">Скопировать ссылку</span><span style="flex:1;"></span><span class="btn quiet">Позже</span></div>
      </div>''')

    account = '''
      <div class="head"><span class="ava"></span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo · привязан 12 сен</div></div></div>
      <h3>Сейчас</h3>
      <div class="row"><span class="d"></span><span>Рисую · Daisuke</span><span class="n">62 %</span></div><div class="pb"><i style="width:62%;"></i></div>
      <div class="row"><span class="d"></span><span>Отправляю · Astral Quantization</span><span class="n">31 МБ из 84</span></div><div class="pb"><i style="width:37%;"></i></div>
      <h3>Недавно</h3>
      <div class="row bad"><span class="t">14:02</span><span class="g">✕</span><span>Рендер не завершился</span><span class="n"><a href="#">Ещё раз</a></span></div>
      <div class="row done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана · Blue Zenith</span></div>
      <div class="row done"><span class="t">12:20</span><span class="g">✓</span><span>Видео ушло в Telegram · FREEDOM DiVE</span></div>
      <h3>Воркер</h3>
      <div class="row"><span>Готов брать работу</span><span class="n">2 в очереди · 14 сегодня</span></div>
      <div class="foot"><span>Видео уходят в этот чат</span><span class="quit">Выйти</span></div>'''
    out["MenuAccount"] = over(rest, f'{top(WORDS3, AVATAR, crest=False)}<div class="menu">{account}</div>')

    story = '''
      <div class="head"><span class="ava"></span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo</div></div><span class="n" style="margin-left:auto; color:#6b655f; font-family:'JetBrains Mono', ui-monospace, Menlo, monospace; font-size:11px;">2 дела</span></div>
      <div style="margin-top:10px;">
      <div class="row"><span class="t">14:03</span><span class="d"></span><b>Рисую</b><span style="color:#6b655f;">· Daisuke · 62 %</span></div>
      <div class="row"><span class="t">14:03</span><span class="d"></span><b>Отправляю</b><span style="color:#6b655f;">· Astral Quantization · 37 %</span></div>
      <div class="row bad"><span class="t">14:02</span><span class="g">✕</span><span>Рендер не завершился</span><span style="color:#6b655f;">· <a href="#">Ещё раз</a></span></div>
      <div class="row done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана</span><span style="color:#6b655f;">· Blue Zenith</span></div>
      <div class="row done"><span class="t">12:20</span><span class="g">✓</span><span>Видео в Telegram</span><span style="color:#6b655f;">· FREEDOM DiVE</span></div>
      <div class="row done"><span class="t">12:04</span><span class="g">✓</span><span>Воркер взял работу</span><span style="color:#6b655f;">· @friend</span></div>
      <div class="row done"><span class="t">09:00</span><span class="g">·</span><span style="color:#6b655f;">Открыто · сборка 0.12.0</span></div>
      </div>
      <div class="foot"><span>Воркер готов · 2 в очереди</span><span class="quit">Выйти</span></div>'''
    out["MenuStory"] = over(rest, f'{top(WORDS3, AVATAR, crest=False)}<div class="menu">{story}</div>')

    guest = '''
      <div class="head"><span class="ava" style="background:#2a1418; box-shadow:inset 0 0 0 1px rgba(255,255,255,0.1);"></span><div><div class="nm">Без аккаунта</div><div class="hd">видео остаются на этом компьютере</div></div></div>
      <h3>Сейчас</h3>
      <div class="row"><span class="d"></span><span>Рисую · Daisuke</span><span class="n">62 %</span></div><div class="pb"><i style="width:62%;"></i></div>
      <h3>Недавно</h3>
      <div class="row done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана · Blue Zenith</span></div>
      <div class="foot"><span class="btn primary" style="height:28px; font-size:13px;">Войти через Telegram</span><span class="quit">чтобы отправлять видео в чат</span></div>'''
    out["MenuGuest"] = over(rest, f'{top(WORDS3, LOGIN, crest=False)}<div class="menu">{guest}</div>')

    send = '<span class="btn primary">В Telegram</span><span class="btn quiet">В папке</span><span class="btn quiet">Удалить</span>'
    out["VideoMirror"] = mirror(send)
    sending = '<span class="btn soft"><i style="width:37%;"></i><u style="width:37%;"></u><span>Отправляю</span></span><span class="btn quiet">В папке</span><span class="btn quiet">Удалить</span>'
    out["VideoSending"] = mirror(sending)
    sent = '<span class="btn primary">В Telegram</span><span class="btn quiet">В папке</span><span class="btn quiet">Удалить</span>'
    toast = '<div class="toast"><div class="h"><span class="v">✓</span>Ушло в Telegram</div><div class="b">@naumredlo · 84,2 МБ · 14:14</div></div>'
    out["VideoSent"] = mirror(sent, extra=toast)
    out["VideoPlaying"] = mirror(send, paused=False, extra='<div class="note" style="top:540px; left:auto; right:40px; width:320px; text-align:right;">пробел — пауза, ← → — на 5 с, двойной клик — на весь экран; при игре ползунок и время видны, остальное чуть гаснет</div>')
    playing_big = '<div class="big"><div class="n">1920×1080</div><div class="c">60 fps · 84,2 МБ</div></div>'
    del_card = '''<div class="veil"></div><div class="card" style="left:290px; top:260px; width:400px;">
        <div class="ttl">Удалить видео?</div>
        <div class="cap">NaumRedlo — Astral Quantization · 84,2 МБ. Файл уйдёт в корзину; реплей и карта останутся.</div>
        <div class="btns"><span style="flex:1;"></span><span class="btn quiet">Оставить</span><span class="btn primary">Удалить</span></div>
      </div>'''
    out["VideoDelete"] = mirror(send, big=playing_big, extra=del_card)

    cells = []
    people = [("NaumRedlo", "Dj Grimoire — Astral Quantization", "3:51", "84,2 МБ", "сегодня 14:12"), ("Guest", "xi — Blue Zenith", "2:14", "51,0 МБ", "14 авг"), ("Lopuhh", "xi — FREEDOM DiVE", "4:02", "97,7 МБ", "14 авг"), ("Saki-chan", "Camellia — GALACTIC", "1:48", "40,3 МБ", "3 авг"), ("NaumRedlo", "Chocofan — LUCKY CAT", "3:20", "72,9 МБ", "29 июл"), ("Deeo_XD", "Nevermind", "2:57", "60,1 МБ", "29 июл"), ("Guest", "Sink", "5:11", "118,4 МБ", "29 июл"), ("NaumRedlo", "Dj Grimoire — Astral Quantization", "2:02", "44,0 МБ", "10 июл")]
    for i, (p, m, l, s, w) in enumerate(people):
        on = " on" if i == 0 else ""
        cells.append(f'<div class="v{on}"><div class="th" style="background-image:url({BG[i % len(BG)]});"><span class="len">{l}</span></div><div class="tx"><div class="p">{p}</div><div class="m">{m}</div><div class="w">{w} · {s}</div></div></div>')
    out["VideoGrid"] = own(f'{top(WORDS4, AVATAR)}<div class="grid">{"".join(cells)}</div><div class="note" style="top:640px;">клик — открыть в плеере (следующий кадр), правая кнопка — В папке · В Telegram · Удалить</div>')

    rows = []
    for i, (p, m, l, s, w) in enumerate(people):
        on = " on" if i == 0 else ""
        rows.append(f'<div class="r{on}"><span class="t">{w.replace(" 14:12", "")}</span><span><b>{p}</b> <span class="m">· {m}</span></span><span class="n">{l}</span><span class="n">{s}</span></div>')
    side = f'''<div class="side"><div class="pv"></div><div class="tt">NaumRedlo · Astral Quantization</div><div class="mm">Dj Grimoire — Astral Quantization [Nattu VN0TH3R] · HD DT HR</div><div class="ww">сегодня 14:12 · 3:51 · 1920×1080 · 84,2 МБ</div><div class="acts">{send}</div></div>'''
    out["VideoList"] = own(f'{top(WORDS4, AVATAR)}<div class="list">{"".join(rows)}</div>{side}')

    out["VideoEmpty"] = own(f'{top(WORDS4, AVATAR)}<div class="empty"><div class="h">Пока ни одного видео</div><div class="c">Отрендерите реплей — оно появится здесь. <a href="#">К реплеям</a></div></div>')

    return out

NOTES = {
    "TopLogin": ("Справа · кнопка входа", "Пока устройство ни к кому не привязано: после трёх слов — «Войти через Telegram». Кнопка, а не точка: она про действие, и она исчезает, как только дело сделано."),
    "TopAvatar": ("Справа · аватар", "После входа кнопка становится аватаром из Telegram, 28 px, без имени — имя в меню. Клик открывает меню аккаунта."),
    "TopAvatarLive": ("Справа · аватар с кольцом", "Кольцо цвета акцента вокруг аватара, пока что-то идёт: рендер, карта, отправка, работа воркера. Одна деталь вместо полосы и точки; гаснет сама."),
    "LoginCard": ("Вход · карточка", "Клик по кнопке: карточка с кодом и QR. «Открыть Telegram» ведёт на t.me/бот?start=код; бот отвечает «Привязано», приложение ждёт ответа, аватар появляется на месте кнопки. Никаких паролей — только код, который живёт пять минут."),
    "MenuAccount": ("Меню · аккаунт", "Из аватара: шапка с именем и датой привязки, Сейчас с полосами, Недавно с временем, строка воркера, подвал «Видео уходят в этот чат · Выйти». Меню про человека и про сегодня."),
    "MenuStory": ("Меню · лента", "Та же шапка, под ней одна лента дня: время · знак · слово · деталь, дела сверху. Короче секций, ближе к реестру; воркер — одной строкой в подвале."),
    "MenuGuest": ("Меню · без аккаунта", "Меню открывается и без входа — дела и уведомления местные, а в подвале кнопка входа и зачем он нужен: чтобы отправлять видео в чат."),
    "VideoMirror": ("Видео · зеркало главного", "Четвёртое слово — Видео. Экран повторяет главный: сверху плеер вместо живой сцены, слева данные видео и кнопки В Telegram · В папке · Удалить, справа разрешение и размер, снизу журнал видео с длительностями. Ничему не надо учиться заново."),
    "VideoPlaying": ("Видео · играет", "Пробел — пауза, стрелки — на 5 с, двойной клик — на весь экран. Пока играет, остаются ползунок и время, остальное чуть гаснет."),
    "VideoSending": ("Видео · отправка", "В Telegram — та же кнопка-прогресс, что у рендера: одно слово и заливка на всю ширину, по байтам. Клик — остановить."),
    "VideoSent": ("Видео · отправлено", "Готово — кнопка возвращается к «В Telegram», рядом ничего не остаётся; карточка внизу слева говорит, куда ушло, и уходит сама через 6 с."),
    "VideoDelete": ("Видео · удаление", "Единственный вопрос на этом экране: удалить — в корзину, реплей и карта остаются."),
    "VideoGrid": ("Видео · сетка", "Другой взгляд: сетка карточек, по четыре в ряд, длительность на кадре, кто · карта · когда · размер. Клик открывает плеер; для десятков видео быстрее, чем журнал."),
    "VideoList": ("Видео · список", "Реестр слева, превью и кнопки справа. Самый плотный; ближе к таблице, чем к приложению."),
    "VideoEmpty": ("Видео · пусто", "Пока ничего не отрендерено: одна строка и ссылка к реплеям."),
}

RECOMMENDED = "Предложение: справа — кнопка входа, потом аватар с кольцом (TopAvatarLive), из него меню-аккаунт (MenuAccount); видео — четвёртое слово и зеркало главного экрана (VideoMirror): плеер на месте сцены, кнопки на месте Отрендерить, журнал видео на месте журнала реплеев. Отправка в Telegram — только отсюда, той же кнопкой-прогрессом."

TO_BUILD = "Что строить, по порядку: (1) хранилище видео — ~/.dossier/Renders с индексом videos.json: реплей, карта, размер, длительность, разрешение, время; (2) экран Видео как зеркало главного: плеер (декодер через ffmpeg — кадры в текстуру, звук через cpal), журнал, кнопки; (3) вход через Telegram: код, t.me/бот?start=код, ожидание ответа бота, аватар из getUserProfilePhotos; (4) отправка видео ботом: POST /render/send с файлом, прогресс по байтам в кнопке, карточка «Ушло»; (5) меню аккаунта с очередью уведомлений (~/.dossier/notices.json); (6) кольцо на аватаре, пока что-то идёт."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "store boards")
