from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

CSS = """
  <style>
    .cover { position: absolute; top: 14px; right: 30px; width: 320px; height: 52px; background: #0d0508; }
    .words { position: absolute; top: 22px; right: 40px; display: flex; align-items: center; gap: 22px; height: 36px; font-weight: 600; color: #a9a29b; }
    .words b { color: #ece7e2; }
    .words .ava { width: 28px; height: 28px; border-radius: 50%; background: #e24848; display: flex; align-items: center; justify-content: center; color: #fff; font-size: 13px; }
    .panel { position: absolute; top: 62px; right: 40px; width: 380px; padding: 8px; border-radius: 18px; background: rgba(20,9,12,0.86); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 24px 60px rgba(0,0,0,0.55); box-sizing: border-box; display: flex; flex-direction: column; gap: 8px; color: #ece7e2; font-size: 13px; backdrop-filter: blur(24px); }
    .tile { border-radius: 12px; background: rgba(255,255,255,0.045); border: 1px solid rgba(255,255,255,0.05); padding: 12px 14px; box-sizing: border-box; }
    .tile.quiet { background: rgba(255,255,255,0.025); }
    .tile.bad { background: rgba(226,72,72,0.08); border-color: rgba(226,72,72,0.18); }
    .head { display: flex; align-items: center; gap: 12px; }
    .head .ava { width: 40px; height: 40px; border-radius: 50%; background: #e24848; display: flex; align-items: center; justify-content: center; color: #fff; font-size: 18px; font-weight: 600; flex: none; }
    .head .nm { font-size: 15px; font-weight: 600; }
    .head .hd { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; margin-top: 2px; }
    .head .out { margin-left: auto; color: #a9a29b; font-size: 12px; }
    .seg { display: flex; gap: 4px; padding: 4px; }
    .seg span { flex: 1; text-align: center; padding: 6px 0; border-radius: 8px; color: #6b655f; font-weight: 600; font-size: 12px; }
    .seg span.on { background: rgba(255,255,255,0.08); color: #ece7e2; }
    .kv { display: flex; align-items: center; height: 24px; color: #a9a29b; font-size: 12px; }
    .kv .n { margin-left: auto; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; }
    .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
    .big { display: flex; flex-direction: column; gap: 2px; }
    .big .v { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 22px; font-weight: 700; color: #ece7e2; }
    .big .k { color: #6b655f; font-size: 11px; }
    .row2 { display: flex; align-items: center; gap: 10px; }
    .th { width: 40px; height: 40px; border-radius: 8px; background: #0a0507 center/cover; flex: none; position: relative; }
    .th i { position: absolute; right: -4px; bottom: -4px; width: 16px; height: 16px; border-radius: 50%; background: #8cd04a; border: 2px solid #1a0b0f; color: #fff; font-size: 9px; font-weight: 700; display: flex; align-items: center; justify-content: center; font-style: normal; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .th i.x { background: #e24848; }
    .ev { flex: 1; min-width: 0; }
    .ev .w { font-weight: 600; font-size: 13px; display: flex; align-items: center; gap: 8px; }
    .ev .w .t { margin-left: auto; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; font-weight: 400; }
    .ev .w a { color: #e24848; text-decoration: none; font-size: 12px; }
    .ev .d { color: #a9a29b; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }
    .ev .w.bad { color: #e24848; }
    .job { display: flex; align-items: center; gap: 8px; height: 22px; font-size: 12px; }
    .job b { color: #ece7e2; }
    .job .d { display: block; width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); }
    .job .m { color: #a9a29b; }
    .pb { height: 3px; border-radius: 2px; background: rgba(255,255,255,0.08); position: relative; margin-top: 6px; }
    .pb i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 2px; background: #e24848; }
    .h3 { font-size: 10px; letter-spacing: 0.08em; text-transform: uppercase; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; margin-bottom: 6px; }
    .note { position: absolute; left: 40px; font-size: 12px; line-height: 16px; color: #a9a29b; width: 420px; }
    .btn { display: inline-flex; align-items: center; height: 30px; padding: 0 12px; border-radius: 8px; font-weight: 600; font-size: 13px; background: #e24848; color: #fff; }
  </style>"""

BG = ["bg-astral.jpg", "bg-zenith.jpg", "bg-freedom.jpg", "bg-galactic.jpg"]

WORDS = '<div class="cover"></div><div class="words"><b>Реплеи</b><span>Видео</span><span>Воркер</span><span>Настройки</span><span class="ava">N</span></div>'
HEAD = '<div class="tile head"><span class="ava">N</span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo · ID 7</div></div><span class="out">Выйти</span></div>'

def seg(on):
    return '<div class="tile seg quiet">' + "".join(f'<span class="{"on" if k == on else ""}">{k}</span>' for k in ("Аккаунт", "Лента", "Статистика")) + "</div>"

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{CSS}{extra}')

def event(bg, mark, words, detail, time, link="", bad=False):
    cls = "bad" if bad else ""
    icon = f'<i class="{"x" if bad else ""}">{mark}</i>'
    a = f'<a href="#">{link}</a>' if link else ""
    return f'<div class="tile row2 {cls}"><span class="th" style="background-image:url({bg});">{icon}</span><div class="ev"><div class="w {cls}">{words}{a}<span class="t">{time}</span></div><div class="d">{detail}</div></div></div>'

def boards():
    rest = frame("main-rest-ru-RU")
    out = {}

    account = f'''<div class="panel">{HEAD}{seg("Аккаунт")}
      <div class="tile"><div class="kv"><span>Видео уходят в чат</span><span class="n">@naumredlo</span></div><div class="kv"><span>Воркер</span><span class="n">готов · берёт работу</span></div></div>
      <div class="grid2"><div class="tile big"><span class="v">0.12.0</span><span class="k">сборка · та же, что у бота</span></div><div class="tile big"><span class="v">12 сен</span><span class="k">привязано</span></div></div>
    </div>'''
    out["TilesAccount"] = over(rest, WORDS + account + '<div class="note" style="top:96px;">Плитки, как в Пункте управления: шапка — своя плитка, переключатель — сегменты, дальше каждая группа сведений — отдельная плитка; между ними 8 px воздуха, панель чуть просвечивает.</div>')

    feed = f'''<div class="panel">{HEAD}{seg("Лента")}
      <div class="tile"><div class="job"><span class="d"></span><b>Рисую</b><span class="m">· Astral Quantization</span></div><div class="pb"><i style="width:62%;"></i></div></div>
      {event(BG[1], "✕", "Рендер не завершился", "Guest — xi — Blue Zenith · ffmpeg завершился с кодом 1", "14:43", "Ещё раз", True)}
      {event(BG[1], "✓", "Карта скачана", "xi — Blue Zenith · [FOUR DIMENSIONS]", "14:26")}
      {event(BG[2], "✓", "Ушло в Telegram", "-legusshhka- — xi — FREEDOM DiVE [Extra] · @naumredlo · 97,7 МБ", "14:09")}
      {event(BG[0], "✓", "Отрендерено", "NaumRedlo — Dj Grimoire — Astral Quantization · 3:51 · 84,2 МБ", "13:52", "Открыть")}
    </div>'''
    out["TilesFeed"] = over(rest, WORDS + feed + '<div class="note" style="top:96px;">Лента: каждое событие — плитка с кадром карты и бейджем; идущее дело — плитка с полосой хода. Ошибка — плитка с красной подложкой.</div>')

    stats = f'''<div class="panel">{HEAD}{seg("Статистика")}
      <div class="tile quiet"><div class="h3">как воркер</div><div class="grid2"><div class="big"><span class="v">142</span><span class="k">сделано работ · 23 за месяц</span></div><div class="big"><span class="v">11,3 ГБ</span><span class="k">отдано</span></div></div></div>
      <div class="tile quiet"><div class="h3">на этом устройстве</div><div class="grid2"><div class="big"><span class="v">187</span><span class="k">реплеев в журнале</span></div><div class="big"><span class="v">8</span><span class="k">видео · 568 МБ</span></div><div class="big"><span class="v">23</span><span class="k">отправлено · 1,9 ГБ</span></div><div class="big"><span class="v">2:40</span><span class="k">среднее время работы</span></div></div></div>
    </div>'''
    out["TilesStats"] = over(rest, WORDS + stats + '<div class="note" style="top:96px;">Статистика: цифры крупно, по две в ряд, подпись под цифрой — как ползунки Пункта управления, только читаются, а не двигаются.</div>')

    split = f'''<div class="panel" style="padding:0; background:transparent; border:none; box-shadow:none; gap:10px;">
      <div class="tile head" style="background: rgba(20,9,12,0.9); box-shadow: 0 18px 44px rgba(0,0,0,0.5); border-radius: 14px;"><span class="ava">N</span><div><div class="nm">Naum Redlo</div><div class="hd">@naumredlo · ID 7</div></div><span class="out">Выйти</span></div>
      <div class="tile" style="background: rgba(20,9,12,0.9); box-shadow: 0 18px 44px rgba(0,0,0,0.5); border-radius: 14px; padding: 8px;">
        {seg("Аккаунт")}
        <div style="padding: 6px 8px 2px;"><div class="kv"><span>Видео уходят в чат</span><span class="n">@naumredlo</span></div><div class="kv"><span>Telegram ID</span><span class="n">7</span></div><div class="kv"><span>Воркер</span><span class="n">готов · берёт работу</span></div><div class="kv"><span>Сборка</span><span class="n">0.12.0</span></div></div>
      </div>
    </div>'''
    out["SplitHead"] = over(rest, WORDS + split + '<div class="note" style="top:96px;">Две карточки: шапка отдельно, ниже — карточка с сегментами и содержимым. Проще плиток: разделена только верхняя часть, остальное как сейчас.</div>')

    stacked = f'''<div class="panel" style="width:380px;">{HEAD}
      <div class="tile"><div class="h3">сейчас</div><div class="job"><span class="d"></span><b>Рисую</b><span class="m">· Astral Quantization</span></div><div class="pb"><i style="width:62%;"></i></div></div>
      <div class="tile quiet" style="padding:8px;">{event(BG[1], "✕", "Рендер не завершился", "Guest — Blue Zenith · ffmpeg завершился с кодом 1", "14:43", "Ещё раз", True).replace('class="tile row2 bad"', 'class="row2"')}<div style="height:8px;"></div>{event(BG[2], "✓", "Ушло в Telegram", "-legusshhka- — FREEDOM DiVE · 97,7 МБ", "14:09").replace('class="tile row2 "', 'class="row2"')}</div>
      <div class="grid2"><div class="tile big"><span class="v">187</span><span class="k">реплеев</span></div><div class="tile big"><span class="v">8</span><span class="k">видео · 568 МБ</span></div></div>
      <div class="tile kv" style="height:auto;"><span>Видео уходят в чат</span><span class="n">@naumredlo</span></div>
    </div>'''
    out["NoTabs"] = over(rest, WORDS + stacked + '<div class="note" style="top:96px;">Без вкладок, как настоящий Пункт управления: всё на одной панели — шапка, сейчас, два последних события, две цифры, чат. Короче, но лента и статистика видны лишь краем.</div>')

    return out

NOTES = {
    "TilesAccount": ("Плитки · Аккаунт", "Шапка, сегменты и группы сведений — отдельные плитки на просвечивающей панели, 8 px между ними. Предлагаемый вариант."),
    "TilesFeed": ("Плитки · Лента", "Событие — плитка с кадром карты, бейджем и временем; дело — плитка с полосой; ошибка — красная подложка."),
    "TilesStats": ("Плитки · Статистика", "Цифры крупно по две в ряд с подписью под ними; группы — «как воркер» и «на этом устройстве»."),
    "SplitHead": ("Две карточки", "Шапка отдельной карточкой над основной; в основной — сегменты и строки. Минимальное изменение нынешнего меню."),
    "NoTabs": ("Без вкладок", "Одна панель со всем сразу: сейчас, два события, две цифры, чат. Ближе всего к Пункту управления, дальше всего от того, что уже принято."),
}

RECOMMENDED = "Предложение: плитки (TilesAccount / TilesFeed / TilesStats). Анимации: панель появляется из угла — проявление и рост 0,96 → 1 за 200 мс, плитки догоняют друг друга с шагом 30 мс; закрытие 140 мс; смена вкладки — содержимое перекрёстно гаснет и сдвигается на 6 px за 160 мс, сегмент скользит."

if __name__ == "__main__":
    for name, html in boards().items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote menu boards")
