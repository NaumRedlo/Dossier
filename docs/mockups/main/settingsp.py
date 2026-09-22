from pathlib import Path

from built import page

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
    .words .ava { width: 28px; height: 28px; border-radius: 50%; background: #e24848; }
    .pane { border-radius: 12px; background: rgba(12,5,7,0.97); border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 24px rgba(0,0,0,0.35); box-sizing: border-box; padding: 14px 16px; color: #ece7e2; font-size: 13px; }
    .pane h3 { margin: 0 0 8px; font-size: 10px; letter-spacing: 0.08em; text-transform: uppercase; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 400; }
    .ln { display: flex; align-items: center; min-height: 32px; gap: 12px; }
    .ln .k { color: #ece7e2; }
    .ln .k small { display: block; color: #6b655f; font-size: 11px; margin-top: 1px; }
    .ln .v { margin-left: auto; display: flex; align-items: center; gap: 8px; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .sg { display: flex; gap: 3px; padding: 3px; border-radius: 9px; background: rgba(0,0,0,0.35); border: 1px solid rgba(255,255,255,0.06); }
    .sg span { padding: 4px 10px; border-radius: 7px; color: #6b655f; font-weight: 600; font-size: 12px; font-family: Commissioner, sans-serif; }
    .sg span.on { background: rgba(255,255,255,0.09); color: #ece7e2; }
    .tg { width: 34px; height: 20px; border-radius: 10px; background: rgba(255,255,255,0.1); position: relative; }
    .tg i { position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: #a9a29b; }
    .tg.on { background: #e24848; }
    .tg.on i { left: 17px; background: #fff; }
    .bt { display: inline-flex; align-items: center; height: 28px; padding: 0 10px; border-radius: 8px; font-weight: 600; font-size: 12px; color: #a9a29b; font-family: Commissioner, sans-serif; }
    .bt.primary { background: #e24848; color: #fff; }
    .bt.soft { background: rgba(255,255,255,0.07); color: #ece7e2; }
    .fld { height: 28px; padding: 0 10px; border-radius: 8px; background: rgba(0,0,0,0.35); border: 1px solid rgba(255,255,255,0.08); display: flex; align-items: center; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; min-width: 180px; }
    .pth { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 320px; }
    .stacked { position: absolute; left: 40px; top: 92px; width: 900px; display: flex; flex-direction: column; gap: 10px; }
    .twocol { position: absolute; left: 40px; top: 92px; width: 900px; display: grid; grid-template-columns: 1fr 1fr; gap: 10px; align-items: start; }
    .sidebar { position: absolute; left: 40px; top: 92px; width: 200px; display: flex; flex-direction: column; gap: 2px; }
    .sidebar span { padding: 8px 12px; border-radius: 8px; color: #a9a29b; font-weight: 600; font-size: 13px; }
    .sidebar span.on { background: rgba(255,255,255,0.07); color: #ece7e2; }
    .sidebar small { display: block; color: #6b655f; font-weight: 400; font-size: 11px; margin-top: 1px; }
    .mainc { position: absolute; left: 264px; top: 92px; width: 676px; display: flex; flex-direction: column; gap: 10px; }
    .hint { position: absolute; left: 40px; bottom: 24px; font-size: 12px; line-height: 16px; color: #a9a29b; width: 900px; }
    .dt { display: inline-block; width: 7px; height: 7px; border-radius: 50%; background: #8cd04a; margin-right: 6px; }
    .dt.off { background: #6b655f; }
    .srcl { display: flex; align-items: center; gap: 12px; min-height: 40px; }
    .srcl .th { width: 36px; height: 36px; border-radius: 8px; background: rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: center; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; flex: none; }
    .srcl .k { flex: 1; min-width: 0; }
    .srcl .k b { display: block; }
  </style>"""

WORDS = '<div class="top"><div class="crest"><i></i><s></s>Dossier</div><div class="words"><span>Реплеи</span><span>Видео</span><span>Сообщество</span><span>Воркер</span><b>Настройки</b><span class="ava"></span></div></div>'

def own(extra):
    return page(W, H, f'{CSS}<div style="position:absolute; inset:0; background:#0d0508;"></div>{WORDS}{extra}')

def seg(opts, on):
    return '<span class="sg">' + "".join(f'<span class="{"on" if o == on else ""}">{o}</span>' for o in opts) + "</span>"

def tog(on=True):
    return f'<span class="tg {"on" if on else ""}"><i></i></span>'

def row(k, v, small=""):
    s = f"<small>{small}</small>" if small else ""
    return f'<div class="ln"><span class="k">{k}{s}</span><span class="v">{v}</span></div>'

GENERAL = f'''<div class="pane"><h3>общее</h3>
  {row("Язык", seg(["English", "Русский"], "Русский"))}
  {row("Имя устройства", '<span class="fld">drejk starsij</span>', "так его видит бот")}
  {row("Показывать реплей в сцене", tog(True), "живая игра за вьюером; выключено — размытый фон карты")}
</div>'''

SOURCES = f'''<div class="pane"><h3>источники реплеев</h3>
  <div class="srcl"><span class="th">osu!</span><div class="k"><b>osu!stable</b><div class="pth">~/Library/Application Support/osu/ · 187 реплеев · 1 240 карт</div></div>{tog(True)}<span class="bt">В папке</span></div>
  <div class="srcl"><span class="th">lazer</span><div class="k"><b>osu!lazer</b><div class="pth">~/Library/Application Support/osu-lazer/ · 42 реплея</div></div>{tog(True)}<span class="bt">В папке</span></div>
  <div class="srcl"><span class="th">dir</span><div class="k"><b>Dossier Corpus</b><div class="pth">~/Documents/Dossier Corpus · 96 реплеев</div></div>{tog(False)}<span class="bt">Убрать</span></div>
  <div class="ln"><span class="bt soft">Добавить папку…</span><span class="bt">Поискать на устройстве</span></div>
</div>'''

RENDER = f'''<div class="pane"><h3>рендер</h3>
  {row("Размер", seg(["720p", "1080p", "1440p"], "1080p"))}
  {row("Кадров в секунду", seg(["30", "60"], "60"))}
  {row("Качество", seg(["Хорошо", "Лучше", "Максимум"], "Лучше"), "CRF 20 · medium — как у бота")}
  {row("Фон карты", tog(True))}
  {row("Сториборд", tog(False))}
  {row("Видео карты", tog(False))}
  {row("Звук нажатий", seg(["click", "soft", "none"], "click"))}
</div>'''

STORAGE = f'''<div class="pane"><h3>хранилище</h3>
  {row("Видео", '<span class="pth">~/.dossier/Renders</span><span class="bt">Изменить</span><span class="bt">В папке</span>', "8 видео · 568 МБ")}
  {row("Карты, скачанные приложением", '<span class="pth">~/.dossier/Songs</span><span class="bt">В папке</span>', "31 карта · 1,1 ГБ")}
  {row("Кэш", '<span class="pth">~/.dossier</span><span class="bt">Очистить</span>', "миниатюры, индекс карт, найденные реплеи")}
</div>'''

TOOLS = f'''<div class="pane"><h3>инструменты</h3>
  {row("ffmpeg", '<span class="dt"></span>7.1 · своя сборка <span class="bt">Обновить</span>', "~/.dossier/bin/ffmpeg")}
  {row("Бот", '<span class="dt"></span>onenineeightfour.ignorelist.com <span class="bt">Изменить</span>', "сборка 0.12.0 · та же, что у приложения")}
  {row("Обновления", 'сборка 0.12.0 <span class="bt soft">Проверить</span>', "последняя проверка сегодня в 14:00")}
</div>'''

ABOUT = f'''<div class="pane"><h3>о программе</h3>
  {row("Dossier", "0.12.0 · движок dossier 0.11.0 (53e3cc7)")}
  {row("Исходники", '<span class="bt">github.com/NaumRedlo/Dossier</span>')}
  {row("Лицензия", "MIT")}
</div>'''

BENTO_CSS = '''
  <style>
    .bento { position: absolute; left: 40px; top: 92px; width: 900px; display: grid; grid-template-columns: repeat(3, 1fr); grid-auto-rows: minmax(120px, auto); gap: 10px; align-items: stretch; }
    .bento .pane { margin: 0; }
    .bento .tall { grid-row: span 2; }
    .bento .wide { grid-column: span 2; }
    .bento .pane.mini { display: flex; flex-direction: column; justify-content: space-between; }
    .big { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 22px; font-weight: 700; color: #ece7e2; }
    .big small { display: block; color: #6b655f; font-size: 11px; font-family: Commissioner, sans-serif; font-weight: 400; margin-top: 2px; }
    .pane.dim { opacity: 0.35; }
    .pane.focus { position: absolute; left: 40px; top: 92px; width: 900px; z-index: 2; box-shadow: 0 30px 80px rgba(0,0,0,0.6); }
    .pane .x { position: absolute; right: 14px; top: 12px; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .chips { position: absolute; left: 40px; top: 92px; display: flex; gap: 6px; }
    .chips span { padding: 5px 10px; border-radius: 8px; background: rgba(255,255,255,0.05); color: #a9a29b; font-size: 12px; font-weight: 600; }
    .chips span.on { background: rgba(255,255,255,0.1); color: #ece7e2; }
    .pane.sm h3 { margin-bottom: 4px; }
    .pane .sub { color: #6b655f; font-size: 11px; }
  </style>'''

def own2(extra):
    return page(W, H, f'{CSS}{BENTO_CSS}<div style="position:absolute; inset:0; background:#0d0508;"></div>{WORDS}{extra}')

MINI_LANG = f'<div class="pane mini sm"><h3>язык</h3>{seg(["English", "Русский"], "Русский")}</div>'
MINI_DEVICE = '<div class="pane mini sm"><h3>устройство</h3><span class="fld" style="min-width:0;">drejk starsij</span><span class="sub">так его видит бот</span></div>'
MINI_SCENE = f'<div class="pane mini sm"><h3>сцена</h3><div class="ln"><span class="k">Живой реплей</span><span class="v">{tog(True)}</span></div><span class="sub">выключено — размытый фон карты</span></div>'
MINI_FFMPEG = '<div class="pane mini sm"><h3>ffmpeg</h3><span class="big">7.1<small>своя сборка · ~/.dossier/bin</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Обновить</span></div></div>'
MINI_BOT = '<div class="pane mini sm"><h3>бот</h3><span class="big" style="font-size:14px;">onenineeightfour.ignorelist.com<small>сборка 0.12.0 · та же, что у приложения</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Изменить</span></div></div>'
MINI_UPDATE = '<div class="pane mini sm"><h3>обновления</h3><span class="big">0.12.0<small>последняя проверка сегодня в 14:00</small></span><div class="ln" style="min-height:0;"><span class="bt soft">Проверить</span></div></div>'
MINI_VIDEOS = '<div class="pane mini sm"><h3>видео</h3><span class="big">8 · 568 МБ<small>~/.dossier/Renders</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Изменить</span><span class="bt">В папке</span></div></div>'
MINI_MAPS = '<div class="pane mini sm"><h3>карты</h3><span class="big">31 · 1,1 ГБ<small>~/.dossier/Songs · скачаны приложением</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">В папке</span></div></div>'
MINI_CACHE = '<div class="pane mini sm"><h3>кэш</h3><span class="big">140 МБ<small>миниатюры, индекс карт, найденные реплеи</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Очистить</span></div></div>'
MINI_ABOUT = '<div class="pane mini sm"><h3>о программе</h3><span class="big" style="font-size:14px;">Dossier 0.12.0<small>движок 0.11.0 (53e3cc7) · MIT</small></span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Исходники</span></div></div>'

S_LANG = f'<div class="pane mini sm"><h3>язык</h3>{seg(["English", "Русский"], "Русский")}</div>'
S_DEVICE = '<div class="pane mini sm"><h3>устройство</h3><span class="fld" style="min-width:0;">drejk starsij</span></div>'
S_SCENE = f'<div class="pane mini sm"><h3>сцена</h3><div class="ln"><span class="k">Живой реплей</span><span class="v">{tog(True)}</span></div></div>'
S_RENDER = f'''<div class="pane" style="height:100%;"><h3>рендер</h3>
  {row("Размер", seg(["720p", "1080p", "1440p"], "1080p"))}
  {row("Кадры", seg(["30", "60"], "60"))}
  {row("Качество", seg(["Хорошо", "Лучше", "Максимум"], "Лучше"))}
  {row("Фон карты", tog(True))}
  {row("Сториборд", tog(False))}
  {row("Видео карты", tog(False))}
  {row("Нажатия", seg(["click", "soft", "нет"], "click"))}
</div>'''
S_SOURCES = f'''<div class="pane" style="height:100%;"><h3>источники</h3>
  <div class="srcl"><span class="th">osu!</span><div class="k"><b>osu!stable</b><div class="pth">187 реплеев · 1 240 карт</div></div>{tog(True)}</div>
  <div class="srcl"><span class="th">lazer</span><div class="k"><b>osu!lazer</b><div class="pth">42 реплея</div></div>{tog(True)}</div>
  <div class="srcl"><span class="th">dir</span><div class="k"><b>Dossier Corpus</b><div class="pth">96 реплеев</div></div>{tog(False)}</div>
  <div class="ln" style="min-height:28px;"><span class="bt soft">Добавить папку…</span></div>
</div>'''
S_VIDEOS = '<div class="pane mini sm"><h3>видео</h3><span class="big">8 · 568 МБ</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">В папке</span><span class="bt">Изменить</span></div></div>'
S_MAPS = '<div class="pane mini sm"><h3>карты</h3><span class="big">31 · 1,1 ГБ</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">В папке</span></div></div>'
S_CACHE = '<div class="pane mini sm"><h3>кэш</h3><span class="big">140 МБ</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Очистить</span></div></div>'
S_FFMPEG = '<div class="pane mini sm"><h3>ffmpeg</h3><span class="big">7.1</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Обновить</span></div></div>'
S_UPDATE = '<div class="pane mini sm"><h3>dossier</h3><span class="big">0.12.0</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Проверить</span><span class="bt">Исходники</span></div></div>'
S_ABOUT = '<div class="pane mini sm"><h3>о программе</h3><span class="big" style="font-size:14px;">Dossier · MIT</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Исходники</span></div></div>'

B_ACCOUNT = '<div class="pane mini sm"><h3>аккаунт</h3><div class="ln" style="min-height:0;"><span class="ava" style="width:28px; height:28px; border-radius:50%; background:#e24848; display:inline-block;"></span><span class="k" style="margin-left:10px;"><b>Stepan Kapitsa</b><br><span class="pth">@NaumRedlo</span></span></div><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Выйти</span></div></div>'
B_CHAT = '<div class="pane mini sm"><h3>видео уходят в</h3><span class="big" style="font-size:16px;">@NaumRedlo</span><div class="ln" style="min-height:0;"><span class="bt" style="padding-left:0;">Личный чат</span></div></div>'
B_SERVER = '<div class="pane mini sm"><h3>сервер</h3><span class="big" style="font-size:14px;">onenineeightfour.ignorelist.com</span><div class="ln" style="min-height:0;"><span class="dt"></span><span class="pth">отвечает · сборка 0.12.0</span></div></div>'
B_WORKER = f'<div class="pane mini sm"><h3>воркер</h3><div class="ln"><span class="k">Брать работу бота</span><span class="v">{tog(False)}</span></div><span class="pth">будет доступно позже</span></div>'
B_NOTIFY = f'<div class="pane mini sm"><h3>в чат</h3><div class="ln"><span class="k">Готовые рендеры</span><span class="v">{tog(True)}</span></div><div class="ln"><span class="k">Ошибки</span><span class="v">{tog(False)}</span></div></div>'
B_NAME = '<div class="pane mini sm"><h3>это устройство у бота</h3><span class="fld" style="min-width:0;">drejk starsij</span><span class="pth">привязано 12 сен</span></div>'

GROUP_CSS = '''<style>
  .group { position: absolute; left: 40px; width: 900px; }
  .group h2 { margin: 0 0 8px; font-size: 13px; font-weight: 600; color: #ece7e2; display: flex; align-items: center; gap: 10px; }
  .group h2 span { color: #6b655f; font-weight: 400; font-size: 12px; }
  .group .bento { position: static; }
  .segtop { position: absolute; left: 40px; top: 92px; }
  .bento.small { grid-auto-rows: minmax(104px, auto); }
</style>'''

def own3(extra):
    return page(W, H, f'{CSS}{BENTO_CSS}{GROUP_CSS}<div style="position:absolute; inset:0; background:#0d0508;"></div>{WORDS}{extra}')

CC_CSS = '''<style>
  .switch { position: absolute; left: 0; right: 0; top: 88px; display: flex; justify-content: center; gap: 36px; font-weight: 600; font-size: 14px; color: #6b655f; }
  .switch span { padding-bottom: 6px; border-bottom: 2px solid transparent; }
  .switch span.on { color: #ece7e2; border-bottom-color: #e24848; }
  .ccgrid { position: absolute; left: 40px; top: 136px; width: 900px; display: flex; gap: 10px; align-items: flex-start; }
  .colw { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .cc { border-radius: 16px; background: rgba(255,255,255,0.055); border: 1px solid rgba(255,255,255,0.06); padding: 12px; box-sizing: border-box; color: #ece7e2; font-size: 12px; display: flex; flex-direction: column; gap: 6px; overflow: hidden; }
  .cc.w2 { grid-column: span 2; }
  .cc.h2 { grid-row: span 2; }
  .cc.h3 { grid-row: span 3; }
  .cc .t { font-size: 11px; color: #a9a29b; font-weight: 600; text-align: center; }
  .it { display: flex; align-items: center; gap: 10px; min-height: 30px; }
  .it .ic { width: 28px; height: 28px; border-radius: 50%; background: rgba(255,255,255,0.1); display: flex; align-items: center; justify-content: center; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; font-weight: 700; flex: none; }
  .it.on .ic { background: #e24848; color: #fff; }
  .it .lab { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; }
  .it .lab b { font-weight: 600; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it .lab span { color: #a9a29b; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 6px 10px; }
  .pill { display: flex; align-items: center; gap: 8px; height: 40px; padding: 0 10px; border-radius: 12px; background: rgba(255,255,255,0.06); font-weight: 600; font-size: 12px; }
  .pill .ic { width: 24px; height: 24px; border-radius: 50%; background: rgba(255,255,255,0.1); flex: none; }
  .pill.on { background: rgba(226,72,72,0.18); }
  .pill.on .ic { background: #e24848; }
  .sl { display: flex; flex-direction: column; gap: 4px; }
  .sl .bar { position: relative; height: 26px; border-radius: 13px; background: rgba(255,255,255,0.08); overflow: hidden; }
  .sl .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: rgba(255,255,255,0.22); border-radius: 13px; }
  .sl .bar b { position: absolute; left: 10px; top: 0; line-height: 26px; font-size: 12px; font-weight: 600; }
  .sl .bar em { position: absolute; right: 10px; top: 0; line-height: 26px; font-size: 11px; font-style: normal; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
  .num { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 22px; font-weight: 700; margin-top: auto; }
  .num small { display: block; font-family: Commissioner, sans-serif; font-weight: 400; font-size: 11px; color: #a9a29b; }
  .acts { display: flex; gap: 6px; margin-top: auto; }
  .acts span { display: inline-flex; align-items: center; height: 26px; padding: 0 10px; border-radius: 8px; background: rgba(255,255,255,0.08); color: #ece7e2; font-weight: 600; font-size: 12px; }
  .acts span.hot { background: rgba(226,72,72,0.16); color: #e24848; }
  .sl .bar .tk { position: absolute; top: 10px; width: 6px; height: 6px; border-radius: 50%; background: rgba(255,255,255,0.28); margin-left: -3px; }
  .sl .bar .tk.hit { background: #fff; box-shadow: 0 0 0 3px rgba(255,255,255,0.14); }
  .sl .bar .knob { position: absolute; top: 4px; width: 18px; height: 18px; border-radius: 50%; background: #ece7e2; margin-left: -9px; box-shadow: 0 1px 4px rgba(0,0,0,0.5); }
  .cc .mid { margin-top: auto; margin-bottom: auto; display: flex; flex-direction: column; gap: 8px; }
  .avatar { width: 40px; height: 40px; border-radius: 50%; background: #e24848; flex: none; }
</style>'''

def own4(extra):
    return page(W, H, f'{CSS}{BENTO_CSS}{CC_CSS}<div style="position:absolute; inset:0; background:#0d0508;"></div>{WORDS}{extra}')

def switch(on):
    return '<div class="switch">' + "".join(f'<span class="{"on" if k == on else ""}">{k}</span>' for k in ("Приложение", "Бот")) + "</div>"

def it(ic, b, s, on=False):
    return f'<div class="it {"on" if on else ""}"><span class="ic">{ic}</span><span class="lab"><b>{b}</b><span>{s}</span></span></div>'

def pill(label, on):
    return f'<div class="pill {"on" if on else ""}"><span class="ic"></span>{label}</div>'

def slider(label, value, pct, stops=(50,)):
    ticks = "".join(f'<span class="tk {"hit" if abs(s - pct) < 2 else ""}" style="left:calc(10px + ({s} / 100) * (100% - 20px));"></span>' for s in stops)
    return f'<div class="sl"><div class="bar"><i style="width:calc(10px + ({pct} / 100) * (100% - 20px));"></i>{ticks}<span class="knob" style="left:calc(10px + ({pct} / 100) * (100% - 20px));"></span><b>{label}</b><em>{value}</em></div></div>'

def tile(inner, title=""):
    head = f'<span class="t">{title}</span>' if title else ""
    return f'<div class="cc">{head}{inner}</div>'

def colw(units, *tiles):
    return f'<div class="colw" style="flex:{units};">{"".join(tiles)}</div>'

T_LANG = tile(f'<div style="display:flex; flex-direction:column; gap:2px;">{it("RU", "Русский", "", True)}{it("EN", "English", "")}</div>', "Язык")
T_RENDER = tile(f'<div class="two">{pill("Фон карты", True)}{pill("Сториборд", False)}{pill("Видео карты", False)}</div>{slider("Размер", "1080p", 50, ())}{slider("Кадры", "60", 100, ())}{slider("Качество", "CRF 20", 50, ())}', "Рендер")
T_VIDEOS = tile('<span class="num" style="margin:0;">8<small>568 МБ</small></span><div class="acts" style="margin-top:6px;"><span>В папке</span><span>Изменить</span></div>', "Видео")
T_DEVICE = tile('<span class="fld" style="min-width:0;">drejk starsij</span><div class="acts" style="margin-top:6px;"><span>Переименовать</span></div>', "Устройство")
T_MAPS = tile('<span class="num" style="margin:0;">31<small>1,1 ГБ · кэш 140 МБ</small></span><div class="acts" style="margin-top:6px;"><span>В папке</span><span class="hot">Очистить</span></div>', "Карты и кэш")
T_SCENE = tile(f'<div style="display:flex; flex-direction:column; gap:6px;">{pill("Живой реплей", True)}{pill("Пауза без фокуса", True)}</div>', "Сцена")
T_SOURCES = tile(f'<div class="two">{it("stb", "osu!stable", "187 реплеев · 1 240 карт", True)}{it("lz", "osu!lazer", "42 реплея", True)}{it("dsr", "Dossier Corpus", "96 реплеев")}{it("+", "Добавить папку…", "")}</div>', "Источники")
T_BUILDS = tile('<span class="num" style="margin:0;">0.12.0<small>движок 0.11.0 · ffmpeg 7.1</small></span><div class="acts" style="margin-top:6px;"><span>Проверить</span><span>ffmpeg</span></div>', "Сборки")

CC_APP = f'<div class="ccgrid">{colw(1, T_LANG, T_DEVICE, T_SCENE)}{colw(2, T_RENDER, T_SOURCES)}{colw(1, T_VIDEOS, T_MAPS, T_BUILDS)}</div>'

B_ACC = tile('<div class="it on"><span class="avatar"></span><span class="lab"><b style="font-size:14px;">Stepan Kapitsa</b><span>@NaumRedlo · ID 1060298719</span></span></div><div class="acts" style="margin-top:6px;"><span>Выйти</span></div>', "Аккаунт")
B_CHATS = tile(f'<div style="display:flex; flex-direction:column; gap:4px;">{it("@", "Личный чат", "@NaumRedlo", True)}{it("#", "osu! RU · lounge", "группа · бот внутри")}{it("#", "1984 crew", "группа · бот внутри")}{it("#", "Nattu & friends", "группа · бот внутри")}{it("#", "Calvaria mapping", "группа · бот внутри")}</div>', "Видео уходят в")
B_TELLS = tile(f'<div class="two">{pill("Готовые рендеры", True)}{pill("Ошибки", False)}{pill("Скачанные карты", False)}{pill("Работы воркера", True)}</div>', "Бот сообщает в чат о")
B_WORK = tile(f'{pill("Брать работу", False)}<span class="num" style="margin-top:6px;">0<small>работ сегодня</small></span>', "Воркер")
B_DEV = tile(f'{it("·", "drejk starsij", "привязано 12 сен")}<div class="acts" style="margin-top:6px;"><span class="hot">Отвязать</span></div>', "Это устройство")

CC_BOT = f'<div class="ccgrid">{colw(2, B_ACC, B_TELLS)}{colw(1, B_CHATS)}{colw(1, B_WORK, B_DEV)}</div>'

def boards():
    out = {}
    out["CcApp"] = own4(switch("Приложение") + CC_APP)
    out["CcBot"] = own4(switch("Бот") + CC_BOT)
    app_grid = f'<div class="bento small">{S_LANG}{S_DEVICE}{S_SCENE}<div class="tall">{S_RENDER}</div><div class="wide">{S_SOURCES}</div>{S_VIDEOS}{S_MAPS}{S_CACHE}{S_FFMPEG}{S_UPDATE}</div>'
    bot_grid = f'<div class="bento small">{B_ACCOUNT}{B_CHAT}{B_SERVER}{B_WORKER}{B_NOTIFY}{B_NAME}</div>'
    out["BentoStatic"] = own3(f'<div class="group" style="top:92px;">{app_grid}</div>')
    out["BentoBot"] = own3(f'<div class="segtop">{seg(["Приложение", "Бот"], "Бот")}</div><div class="group" style="top:140px;">{bot_grid}</div>')
    out["BentoApp"] = own3(f'<div class="segtop">{seg(["Приложение", "Бот"], "Приложение")}</div><div class="group" style="top:140px;">{app_grid}</div>')
    out["BentoBoth"] = own3(f'<div class="group" style="top:92px;"><h2>Приложение</h2>{app_grid}</div><div class="group" style="top:604px;"><h2>Бот <span>· @NaumRedlo</span></h2>{bot_grid}</div>')
    out["BentoSettings"] = own2(f'<div class="bento">{MINI_LANG}{MINI_DEVICE}{MINI_SCENE}<div class="tall">{RENDER.replace("class=\"pane\"", "class=\"pane\" style=\"height:100%;\"")}</div><div class="wide">{SOURCES.replace("class=\"pane\"", "class=\"pane\" style=\"height:100%;\"")}</div>{MINI_VIDEOS}{MINI_MAPS}{MINI_CACHE}{MINI_FFMPEG}{MINI_BOT}{MINI_UPDATE}</div>')
    out["BentoAbove"] = own2(f'<div class="bento" style="top:92px;">{MINI_LANG}{MINI_SCENE}{MINI_DEVICE}<div class="wide">{SOURCES.replace("class=\"pane\"", "class=\"pane\" style=\"height:100%;\"")}</div>{MINI_VIDEOS}{MINI_FFMPEG}{MINI_BOT}{MINI_UPDATE}</div><div class="hint">Второй ряд: мелкие плитки — одна настройка на плитку с крупным значением, как в статистике меню; большие — рендер и источники. Прокрутки нет при 980×720, всё в трёх колонках.</div>')
    focus = f'''<div class="bento">{MINI_LANG.replace("pane", "pane dim")}{MINI_DEVICE.replace("pane", "pane dim")}{MINI_SCENE.replace("pane", "pane dim")}<div class="tall">{RENDER.replace("class=\"pane\"", "class=\"pane dim\" style=\"height:100%;\"")}</div><div class="wide">{SOURCES.replace("class=\"pane\"", "class=\"pane dim\" style=\"height:100%;\"")}</div>{MINI_VIDEOS.replace("pane", "pane dim")}{MINI_MAPS.replace("pane", "pane dim")}{MINI_CACHE.replace("pane", "pane dim")}</div>
      <div class="pane focus"><span class="x">Esc</span><h3>рендер</h3>
        <div style="display:grid; grid-template-columns: 1fr 1fr; gap: 0 32px;">
          <div>{row("Размер", seg(["720p", "1080p", "1440p"], "1080p"))}{row("Кадров в секунду", seg(["30", "60"], "60"))}{row("Качество", seg(["Хорошо", "Лучше", "Максимум"], "Лучше"), "CRF 20 · medium — как у бота")}{row("Звук нажатий", seg(["click", "soft", "none"], "click"))}</div>
          <div>{row("Фон карты", tog(True))}{row("Сториборд", tog(False))}{row("Видео карты", tog(False))}<div class="ln"><span class="k">1920×1080 · 60 к/с · CRF 20<small>около 84 МБ на 3:51 · рендер займёт ~2:40 на этой машине</small></span><span class="v"><span class="bt soft">Как у бота</span></span></div></div>
        </div>
      </div>'''
    out["BentoFocus"] = own2(focus + '<div class="hint">Клик по плитке раскрывает её на всю ширину поверх остальных (остальные гаснут), с двумя колонками настроек и расчётом; Esc или клик мимо — обратно в плитку. Так плитки остаются короткими, а подробности — по требованию.</div>')
    chips = '<div class="chips"><span class="on">Всё</span><span>Общее</span><span>Источники</span><span>Рендер</span><span>Хранилище</span><span>Инструменты</span></div>'
    out["BentoChips"] = own2(chips + f'<div class="bento" style="top:130px;">{MINI_LANG}{MINI_DEVICE}{MINI_SCENE}<div class="tall">{RENDER.replace("class=\"pane\"", "class=\"pane\" style=\"height:100%;\"")}</div><div class="wide">{SOURCES.replace("class=\"pane\"", "class=\"pane\" style=\"height:100%;\"")}</div>{MINI_VIDEOS}{MINI_MAPS}{MINI_CACHE}</div><div class="hint">Те же плитки, сверху чипы-фильтры: «Всё» показывает всю сетку, чип оставляет плитки своего раздела. Замена боковой колонке разделов без лишней площади.</div>')
    out["SettingsColumn"] = own(f'<div class="stacked">{GENERAL}{RENDER}{SOURCES}</div><div class="hint">Одна колонка карточек, прокручивается: общее, рендер, источники, хранилище, инструменты, о программе. Каждая карточка — как плитка меню; подпись под строкой объясняет, что она значит.</div>')
    out["SettingsSidebar"] = own(f'''<div class="sidebar"><span class="on">Общее<small>язык, устройство, сцена</small></span><span>Источники<small>папки osu! и свои</small></span><span>Рендер<small>размер, качество, звук</small></span><span>Хранилище<small>видео, карты, кэш</small></span><span>Инструменты<small>ffmpeg, бот, обновления</small></span><span>О программе</span></div>
      <div class="mainc">{GENERAL}{TOOLS}</div><div class="hint">Слева разделы с подписью, справа карточки раздела — как Системные настройки macOS. Больше воздуха, но лишний клик до каждого раздела.</div>''')
    out["SettingsTiles"] = own(f'<div class="twocol">{GENERAL}{RENDER}{SOURCES}{STORAGE}{TOOLS}{ABOUT}</div><div class="hint">Плитки в две колонки, всё видно сразу без прокрутки при 980 px; на узком окне колонка одна.</div>')
    out["SettingsRender"] = own(f'''<div class="sidebar"><span>Общее<small>язык, устройство, сцена</small></span><span>Источники<small>папки osu! и свои</small></span><span class="on">Рендер<small>размер, качество, звук</small></span><span>Хранилище<small>видео, карты, кэш</small></span><span>Инструменты<small>ffmpeg, бот, обновления</small></span><span>О программе</span></div>
      <div class="mainc">{RENDER}<div class="pane"><h3>предпросмотр</h3><div class="ln"><span class="k">1920×1080 · 60 к/с · CRF 20<small>около 84 МБ на 3:51 · рендер займёт ~2:40 на этой машине</small></span><span class="v"><span class="bt soft">Как у бота</span></span></div></div></div>
      <div class="hint">Раздел «Рендер»: сегменты вместо полей, в подписи — что это значит; внизу расчёт размера и времени для выбранного реплея и кнопка «Как у бота», возвращающая значения бота.</div>''')
    out["SettingsSources"] = own(f'''<div class="sidebar"><span>Общее<small>язык, устройство, сцена</small></span><span class="on">Источники<small>папки osu! и свои</small></span><span>Рендер<small>размер, качество, звук</small></span><span>Хранилище<small>видео, карты, кэш</small></span><span>Инструменты<small>ffmpeg, бот, обновления</small></span><span>О программе</span></div>
      <div class="mainc">{SOURCES}{STORAGE}</div>
      <div class="hint">Источники: строка на папку — вид, путь, сколько в ней, выключатель, В папке / Убрать; ниже «Добавить папку…» и поиск на устройстве из главного экрана. Хранилище — где лежит своё и кнопка очистить кэш.</div>''')
    return out

NOTES = {
    "CcApp": ("Пункт управления · Приложение", "Плитка высотой по содержимому: сетка из четырёх колонок, плитки укладываются как кладка — добавил кнопку, плитка выросла, соседи подвинулись. Ползунки без делений: заливка, ручка, значение; шаги прилипают молча. Звуков нажатий нет."),
    "CcBot": ("Пункт управления · Бот", "То же в боте. «Бот сообщает в чат о» — какие события бот пишет в ваш Telegram: готовые рендеры, ошибки, скачанные карты, работы воркера."),
    "BentoStatic": ("Бенто · статичное", "Без подсказок и длинных строк: плитка — заголовок, значение или переключатели, одна-две кнопки. Три колонки, рендер высокий, источники широкие; ни одна плитка не раскрывается."),
    "BentoApp": ("Приложение · Бот — вкладка «Приложение»", "Сегменты над сеткой делят настройки на две: своё (язык, устройство, сцена, источники, рендер, хранилище, ffmpeg, сборка) и ботовское."),
    "BentoBot": ("Приложение · Бот — вкладка «Бот»", "Всё, что касается бота: аккаунт (и Выйти), куда уходят видео, сервер и его сборка, воркер (позже), что слать в чат, как бот зовёт это устройство."),
    "BentoBoth": ("Приложение · Бот — одной страницей", "Обе группы подряд с заголовками, с прокруткой: сверху приложение, ниже бот с ником в заголовке."),
    "BentoSettings": ("Плитки · бенто", "Три колонки: мелкие плитки по одной настройке с крупным значением (язык, устройство, сцена, видео, карты, кэш, ffmpeg, бот, обновления), рендер — высокая плитка, источники — широкая. Всё на одном экране при 980×720."),
    "BentoAbove": ("Плитки · бенто без прокрутки", "Тот же принцип, меньше плиток: без карт, кэша и «о программе» — они уезжают в плитку «Хранилище» и в подвал. Совсем без прокрутки."),
    "BentoFocus": ("Плитки · раскрытие", "Клик по плитке раскрывает её на всю ширину поверх остальных с полным набором настроек и расчётом; Esc — обратно. Плитки короткие, подробности по требованию."),
    "BentoChips": ("Плитки · чипы", "Чипы-фильтры над сеткой вместо боковой колонки: «Всё» или один раздел."),
    "SettingsColumn": ("Настройки · колонка", "Одна прокручиваемая колонка карточек. Просто и в духе меню-плиток; для шести разделов уже длинновато."),
    "SettingsSidebar": ("Настройки · разделы", "Слева разделы с подписью, справа карточки выбранного — Системные настройки macOS. Ясно, что где; предлагаемый вариант."),
    "SettingsTiles": ("Настройки · плитки", "Все разделы плитками в две колонки. Всё сразу перед глазами, но плотно и на узком окне рассыпается в колонку."),
    "SettingsRender": ("Раздел · Рендер", "Сегменты, выключатели, подписи-объяснения; расчёт размера и времени для выбранного реплея; «Как у бота» возвращает его значения."),
    "SettingsSources": ("Раздел · Источники и Хранилище", "Папки osu! и свои с выключателями; хранилище видео и карт, кэш."),
}

RECOMMENDED = "Направление 2026-09-22: плитки как в Пункте управления macOS (CcApp / CcBot): переключатель Приложение · Бот по центру, кружки-значки, пилюли и ползунки. Ранее: статичное бенто без подсказок, настройки разделены на Приложение и Бот. Предложение: сегменты Приложение · Бот над сеткой (BentoApp / BentoBot) — бот отдельно и на своём месте, обе сетки без прокрутки. Раскрытие плитки (BentoFocus) — идея на будущее для подсказок. Ранее: бенто (BentoSettings) с раскрытием по клику — сетка в три колонки, мелкие плитки с крупным значением, рендер и источники крупнее, подробности раскрываются поверх. Прежнее предложение: разделы слева (SettingsSidebar) с карточками справа — шесть разделов: Общее · Источники · Рендер · Хранилище · Инструменты · О программе. Что настраивается: язык, имя устройства, живая сцена; папки реплеев; размер, к/с, качество, фон/сториборд/видео, звук нажатий; папки видео и карт, кэш; ffmpeg, адрес бота, обновления. Значения рендера по умолчанию — как у бота, и кнопка вернуть их."

if __name__ == "__main__":
    for name, html in boards().items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote settings boards")
