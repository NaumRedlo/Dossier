from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

CSS = """
  <style>
    .bar { position: absolute; left: 0; right: 0; top: 76px; height: 30px; display: flex; align-items: stretch; padding: 0 40px; font-size: 12px; color: #a9a29b; background: rgba(7,3,4,0.55); border-top: 1px solid rgba(255,255,255,0.05); border-bottom: 1px solid rgba(255,255,255,0.06); box-sizing: border-box; }
    .bar .seg { position: relative; display: flex; align-items: center; gap: 8px; padding: 0 14px 0 0; margin-right: 18px; white-space: nowrap; }
    .bar .seg b { color: #ece7e2; font-weight: 600; }
    .bar .seg .n { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .bar .seg .d { display: block; width: 6px; height: 6px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 3px rgba(226,72,72,0.16); flex: none; }
    .bar .seg .d.off { background: #6b655f; box-shadow: none; }
    .bar .seg .p { position: absolute; left: 0; bottom: -1px; height: 2px; border-radius: 1px; background: #e24848; }
    .bar .seg .p.track { background: rgba(255,255,255,0.08); }
    .bar .seg.bad b { color: #e24848; }
    .bar .seg .x { color: #e24848; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .bar .seg .v { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .bar .seg a { color: #e24848; text-decoration: none; font-weight: 600; }
    .bar .right { margin-left: auto; margin-right: 0; padding-right: 0; }
    .bar .sep { width: 1px; background: rgba(255,255,255,0.08); margin: 8px 18px 8px 0; }
    .words { position: absolute; top: 22px; right: 40px; display: flex; align-items: center; gap: 22px; height: 36px; font-weight: 600; color: #a9a29b; }
    .words b { color: #ece7e2; }
    .words .chip { display: flex; align-items: center; gap: 8px; height: 28px; padding: 0 10px 0 4px; border-radius: 14px; border: 1px solid rgba(255,255,255,0.08); background: rgba(0,0,0,0.3); font-weight: 500; color: #ece7e2; font-size: 13px; }
    .words .ava { width: 22px; height: 22px; border-radius: 50%; background: linear-gradient(135deg, #3a1015, #e24848); flex: none; }
    .words .ava.big { width: 26px; height: 26px; }
    .words .cnt { display: inline-block; margin-left: 6px; padding: 0 5px; border-radius: 8px; background: rgba(226,72,72,0.16); color: #e24848; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 10px; line-height: 16px; font-weight: 700; vertical-align: 2px; }
    .cover { position: absolute; top: 14px; right: 30px; width: 300px; height: 52px; background: #0d0508; }
    .menu { position: absolute; top: 60px; right: 40px; width: 440px; border-radius: 12px; background: rgba(20,9,12,0.97); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 24px 60px rgba(0,0,0,0.55); box-sizing: border-box; padding: 16px 18px; font-size: 13px; }
    .menu h3 { margin: 12px 0 6px; font-size: 11px; line-height: 14px; letter-spacing: 0.08em; text-transform: uppercase; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 400; }
    .menu h3:first-child { margin-top: 0; }
    .menu .job { display: flex; align-items: center; gap: 10px; }
    .menu .job .n { margin-left: auto; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .menu .pb { height: 2px; border-radius: 1px; background: rgba(255,255,255,0.08); position: relative; margin: 5px 0 9px; }
    .menu .pb i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 1px; background: #e24848; }
    .menu .line { height: 22px; }
    .menu .who { display: flex; align-items: center; gap: 10px; }
    .menu .ava { width: 26px; height: 26px; border-radius: 50%; background: linear-gradient(135deg, #3a1015, #e24848); flex: none; }
    .menu .n2 { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .menu .foot { display: flex; gap: 14px; margin-top: 14px; font-size: 12px; color: #a9a29b; }
    .menu .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 0 22px; }
    .menu .tl .line { position: relative; padding-left: 4px; }
    .menu .tl .t { width: 40px; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .menu .btn { display: inline-flex; align-items: center; height: 28px; padding: 0 10px; border-radius: 8px; color: #a9a29b; font-weight: 600; }
    .menu .btn.primary { background: #e24848; color: #fff; }
    .note { position: absolute; left: 40px; font-size: 12px; line-height: 16px; color: #a9a29b; }
  </style>"""

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{CSS}{extra}')

def seg(inner, cls="", progress=None):
    p = ""
    if progress is not None:
        p = f'<span class="p track" style="width:100%;"></span><span class="p" style="width:{progress}%;"></span>'
    return f'<span class="seg {cls}">{inner}{p}</span>'

def boards():
    rest = frame("main-rest-ru-RU")
    rendered = frame("main-rendering-en-US")
    out = {}
    jobs = seg('<span class="d"></span><b>Рисую</b> · Daisuke <span class="n">62 % · 18 с</span>', progress=62) + seg('<span class="d"></span><b>Скачиваю</b> · Blue Zenith <span class="n">14,2 / 21,8 МБ</span>', progress=65)
    worker = seg('<span class="d"></span><b>Воркер</b> готов <span class="n">2 в очереди</span>', "right")

    out["StripJobs"] = over(rest, f'<div class="bar">{jobs}</div>')
    failed = seg('<span class="x">✕</span><b>Рендер не завершился</b> · 14:02 <a href="#">Ещё раз</a>', "bad")
    short_jobs = seg('<span class="d"></span><b>Рисую</b> · Daisuke <span class="n">62 %</span>', progress=62) + seg('<span class="d"></span><b>Скачиваю</b> · Blue Zenith <span class="n">65 %</span>', progress=65)
    short_worker = seg('<span class="d"></span><b>Воркер</b> <span class="n">2 в очереди</span>', "right")
    out["StripFull"] = over(rest, f'<div class="bar">{short_jobs}<span class="sep"></span>{failed}{short_worker}</div>')
    drawing = seg('<span class="d"></span><b>Воркер</b> рисует для @friend <span class="n">3 из 7 · 2 в очереди</span>', progress=43)
    fetched = seg('<span class="v">✓</span>Карта скачана · Blue Zenith <span class="n">13:51</span>', "right")
    out["StripWorker"] = over(rest, f'<div class="bar">{drawing}{fetched}</div>')
    done = seg('<span class="v">✓</span><b>Готово</b> · Daisuke <a href="#">Открыть</a> <span class="n">· В папке</span>')
    out["StripDone"] = over(rest, f'<div class="bar">{done}{worker}</div>')
    bad = seg('<span class="x">✕</span><b>Не вышло</b> · Daisuke · ffmpeg завершился с кодом 1 <a href="#">Ещё раз</a> <span class="n">· Подробнее</span>', "bad")
    out["StripBad"] = over(rest, f'<div class="bar">{bad}{worker}</div>')
    offline = seg('<span class="d off"></span><b>Офлайн</b> · бот молчит · воркер отдыхает · карты не скачать <span class="n">повтор через 40 с</span>')
    out["StripOffline"] = over(rest, f'<div class="bar">{offline}</div>')
    out["StripQuiet"] = over(rest, '<div class="note" style="top:80px;">в покое полосы нет — верхняя строка как всегда; полоса выезжает сверху за 320 мс, когда есть что сказать, и уезжает, когда сказать нечего</div>')

    out["RightChip"] = over(rest, '<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="chip"><span class="ava"></span>naumredlo</span></div>')
    out["RightAvatar"] = over(rest, '<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="ava big"></span></div>')
    out["RightCount"] = over(rest, '<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер<span class="cnt">2</span></span><span>Настройки</span></div>')
    out["RightPlain"] = over(rest, '<div class="note" style="top:60px; left:auto; right:40px; width:320px; text-align:right;">как сейчас: три слова и ничего больше; всё оперативное — в полосе, всё личное — в Настройках</div>')

    sections = '''
      <h3>Сейчас</h3>
      <div class="job"><span>Рисую · NaumRedlo — Daisuke</span><span class="n">62 %</span></div><div class="pb"><i style="width:62%;"></i></div>
      <div class="job"><span>Скачиваю · xi — Blue Zenith</span><span class="n">14,2 / 21,8 МБ</span></div><div class="pb"><i style="width:65%;"></i></div>
      <h3>Воркер</h3>
      <div class="job"><span class="d" style="display:block; width:8px; height:8px; border-radius:50%; background:#e24848; box-shadow:0 0 0 4px rgba(226,72,72,0.16);"></span><span>Готов брать работу</span><span class="n">2 в очереди · 14 сегодня</span></div>
      <h3>Уведомления</h3>
      <div class="line bad"><span class="g">✕</span><span>Рендер не завершился</span><span class="d">· 14:02</span></div>
      <div class="line done"><span class="g">✓</span><span>Карта скачана · Blue Zenith</span><span class="d">· 13:51</span></div>
      <div class="line done"><span class="g">✓</span><span>Отрендерено для @friend · отправлено</span><span class="d">· 12:20</span></div>
      <h3>Telegram</h3>
      <div class="who"><span class="ava"></span><div><div>@naumredlo</div><div class="n2">привязан · готовые рендеры уходят в этот чат</div></div></div>
      <div class="foot"><span>Сборка 0.12.0 · та же, что у бота</span><span style="margin-left:auto;">Esc — закрыть</span></div>'''
    out["MenuSections"] = over(rest, f'<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="chip"><span class="ava"></span>naumredlo</span></div><div class="menu">{sections}</div>')

    timeline = '''
      <div class="job" style="margin-bottom:10px;"><span class="d" style="display:block; width:8px; height:8px; border-radius:50%; background:#e24848; box-shadow:0 0 0 4px rgba(226,72,72,0.16);"></span><b>Сейчас</b><span class="n">2 дела · воркер готов</span></div>
      <div class="tl">
      <div class="line now"><span class="t">14:03</span><span class="g" style="color:#e24848;">●</span><b>Рисую</b><span class="d">· Daisuke · 62 %</span></div>
      <div class="line now"><span class="t">14:03</span><span class="g" style="color:#e24848;">●</span><b>Скачиваю</b><span class="d">· Blue Zenith · 65 %</span></div>
      <div class="line bad"><span class="t">14:02</span><span class="g">✕</span><span>Рендер не завершился</span><span class="d">· Ещё раз</span></div>
      <div class="line done"><span class="t">13:51</span><span class="g">✓</span><span>Карта скачана</span><span class="d">· Blue Zenith</span></div>
      <div class="line done"><span class="t">12:20</span><span class="g">✓</span><span>Отрендерено для @friend</span><span class="d">· отправлено</span></div>
      <div class="line done"><span class="t">12:04</span><span class="g">✓</span><span>Воркер взял работу</span><span class="d">· @friend</span></div>
      <div class="line todo"><span class="t">09:00</span><span class="g">·</span><span>Открыто</span><span class="d">· сборка 0.12.0</span></div>
      </div>
      <div class="foot"><span class="who"><span class="ava" style="width:20px; height:20px;"></span>@naumredlo</span><span style="margin-left:auto;">Esc — закрыть</span></div>'''
    out["MenuTimeline"] = over(rest, f'<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="chip"><span class="ava"></span>naumredlo</span></div><div class="menu">{timeline}</div>')

    columns = '''
      <div class="cols">
        <div>
          <h3>Сейчас</h3>
          <div class="job"><span>Рисую · Daisuke</span><span class="n">62 %</span></div><div class="pb"><i style="width:62%;"></i></div>
          <div class="job"><span>Скачиваю · Blue Zenith</span><span class="n">65 %</span></div><div class="pb"><i style="width:65%;"></i></div>
          <h3>Воркер</h3>
          <div class="job"><span>Готов</span><span class="n">2 в очереди</span></div>
          <div class="job"><span>Сегодня</span><span class="n">14 · 1,2 ГБ</span></div>
        </div>
        <div>
          <h3>Уведомления</h3>
          <div class="line bad"><span class="g">✕</span><span>Рендер не завершился</span></div>
          <div class="line done"><span class="g">✓</span><span>Карта скачана</span></div>
          <div class="line done"><span class="g">✓</span><span>Для @friend · отправлено</span></div>
          <div class="line done"><span class="g">✓</span><span>Воркер взял работу</span></div>
        </div>
      </div>
      <div class="foot"><span class="who"><span class="ava" style="width:20px; height:20px;"></span>@naumredlo</span><span>Сборка 0.12.0</span><span style="margin-left:auto;" class="btn">Всё прочитано</span></div>'''
    out["MenuColumns"] = over(rest, f'<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="chip"><span class="ava"></span>naumredlo</span></div><div class="menu" style="width:520px;">{columns}</div>')

    minimal = '''
      <h3>Сейчас</h3>
      <div class="job"><span>Рисую · Daisuke</span><span class="n">62 %</span></div><div class="pb"><i style="width:62%;"></i></div>
      <div class="job"><span>Скачиваю · Blue Zenith</span><span class="n">65 %</span></div><div class="pb"><i style="width:65%;"></i></div>
      <h3>Уведомления</h3>
      <div class="line bad"><span class="g">✕</span><span>Рендер не завершился</span><span class="d">· 14:02 · Ещё раз</span></div>
      <div class="line done"><span class="g">✓</span><span>Карта скачана · Blue Zenith</span><span class="d">· 13:51</span></div>
      <div class="foot"><span class="n2">Воркер, Telegram и сборка — в Настройках</span></div>'''
    out["MenuMinimal"] = over(rest, f'<div class="cover"></div><div class="words"><b>Реплеи</b><span>Воркер</span><span>Настройки</span><span class="chip"><span class="ava"></span>naumredlo</span></div><div class="menu" style="width:380px;">{minimal}</div>')
    return out

NOTES = {
    "StripQuiet": ("Полоса · покой", "В покое полосы нет: верхняя строка как всегда. Полоса выезжает из-под неё за 320 мс, когда есть что сказать, и уезжает, когда сказать нечего. Никогда не висит пустой."),
    "StripJobs": ("Полоса · только дела", "Каждое дело — сегмент: точка, слово, имя, моно-цифра справа и двухпиксельная линия хода под сегментом (как у кнопок в просмотре). Ничего лишнего."),
    "StripFull": ("Полоса · всё важное", "Слева дела, через хайрлайн — последнее уведомление (ошибка — красным, со своим «Ещё раз»), справа воркер: готов / рисует, очередь. Три вещи, которые стоит знать, не открывая ничего."),
    "StripWorker": ("Полоса · воркер рисует", "Когда устройство рисует для бота, это дело идёт первым: «Воркер рисует для @friend · 3 из 7», справа — что только что случилось."),
    "StripDone": ("Полоса · готово", "Дело кончилось: галочка, «Готово · Daisuke», ссылки «Открыть» и «В папке». Держится 8 с или до клика, потом сегмент уходит; если он был последним — уходит полоса."),
    "StripBad": ("Полоса · не вышло", "Ошибка занимает сегмент дела: крест, слово, причина в одну строку, «Ещё раз» и «Подробнее». Сама не уходит."),
    "StripOffline": ("Полоса · офлайн", "Состояние: точка гаснет, одной строкой что отдыхает и когда попробует снова. Уходит сама, когда бот ответил."),
    "RightPlain": ("Справа · как сейчас", "Три слова и ничего больше. Полоса берёт на себя всё оперативное, Настройки — всё личное. Самый тихий вариант; ему не хватает только «кто я»."),
    "RightChip": ("Справа · чип аккаунта", "После трёх слов — чип: аватар из Telegram и имя. Клик открывает оперативное меню. Показывает, к кому привязано устройство, не добавляя четвёртого слова."),
    "RightAvatar": ("Справа · только аватар", "Тот же чип без имени: кружок 26 px. Меньше шума, но имя видно только в меню."),
    "RightCount": ("Справа · счётчик у слова", "Без нового элемента: у слова «Воркер» появляется счётчик очереди или непрочитанного. Дёшево, но перегружает слово-экран смыслом события."),
    "MenuSections": ("Меню · разделы", "Карточка из чипа: Сейчас с полосами, Воркер, Уведомления с временем, Telegram, строка сборки. Всё на месте, читается сверху вниз."),
    "MenuTimeline": ("Меню · лента времени", "Одна лента, новое сверху: время, знак, слово, деталь. Дела и уведомления — одна история дня; Telegram — в подвале. Ближе всего к языку реестра."),
    "MenuColumns": ("Меню · две колонки", "Слева — что идёт и воркер, справа — что случилось. Шире, но всё видно без прокрутки; в подвале «Всё прочитано»."),
    "MenuMinimal": ("Меню · минимум", "Только Сейчас и Уведомления. Воркер, Telegram и сборка живут в Настройках — меню про сегодня, не про устройство."),
}

DEFERRED = "Отложено 2026-09-21: полоса не строится. Право верхнего угла — аватар или кнопка входа (страница «Справа сверху»); чип с именем и счётчик у слова сняты вместе с точкой."

RECOMMENDED = "Предложение: полоса «всё важное» (дела · последнее уведомление · воркер), справа — чип аккаунта, меню — лента времени: полоса говорит, что происходит, чип — от чьего имени, лента — что было. Три места, три вопроса, ничего не дублируется."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "strip boards")
