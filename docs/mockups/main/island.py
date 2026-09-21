from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

CSS = """
  <style>
    .isl { position: absolute; top: 22px; right: 318px; height: 36px; display: flex; align-items: center; gap: 10px; padding: 0 14px 0 12px; border-radius: 18px; background: rgba(20,9,12,0.94); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 10px 30px rgba(0,0,0,0.45); font-size: 13px; color: #ece7e2; white-space: nowrap; overflow: hidden; box-sizing: border-box; }
    .isl .fill { position: absolute; left: 0; top: 0; bottom: 0; background: rgba(226,72,72,0.16); border-radius: 18px; }
    .isl .fill.line { top: auto; height: 2px; border-radius: 1px; background: #e24848; left: 12px; }
    .isl > * { position: relative; }
    .isl .d { display: block; width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); flex: none; }
    .isl .d.ok { background: #a9a29b; box-shadow: none; }
    .isl .t { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .isl .n { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .isl b { font-weight: 600; }
    .isl .sep { width: 1px; height: 16px; background: rgba(255,255,255,0.12); }
    .isl.bad { border-color: rgba(226,72,72,0.45); }
    .isl.bad .x { color: #e24848; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .isl .go { color: #e24848; font-weight: 600; }
    .isl.tick .v { color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; }
    .isl.quiet { width: 36px; padding: 0; justify-content: center; opacity: 0.55; }
    .open { position: absolute; top: 22px; right: 40px; width: 460px; border-radius: 18px; background: rgba(20,9,12,0.96); border: 1px solid rgba(255,255,255,0.08); box-shadow: 0 24px 60px rgba(0,0,0,0.55); box-sizing: border-box; overflow: hidden; }
    .open .head { display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 14px 0 12px; font-size: 13px; }
    .open .head .d { display: block; width: 8px; height: 8px; border-radius: 50%; background: #e24848; box-shadow: 0 0 0 4px rgba(226,72,72,0.16); }
    .open .head .n { margin-left: auto; color: #a9a29b; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .open .body { padding: 6px 18px 16px; border-top: 1px solid rgba(255,255,255,0.06); }
    .open h3 { margin: 12px 0 6px; font-size: 11px; line-height: 14px; letter-spacing: 0.08em; text-transform: uppercase; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 400; }
    .open .job { display: flex; align-items: center; gap: 10px; font-size: 13px; }
    .open .job .n { margin-left: auto; color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .open .bar { height: 2px; border-radius: 1px; background: rgba(255,255,255,0.08); position: relative; margin: 5px 0 9px; }
    .open .bar i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 1px; background: #e24848; }
    .open .line { height: 22px; }
    .open .who { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
    .open .ava { width: 26px; height: 26px; border-radius: 50%; background: linear-gradient(135deg, #3a1015, #e24848); flex: none; }
    .open .n2 { color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .open .foot { display: flex; gap: 14px; margin-top: 14px; font-size: 12px; color: #a9a29b; }
    .story { position: absolute; left: 40px; right: 40px; top: 100px; display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; }
    .story .shot { position: relative; height: 210px; border-radius: 10px; overflow: hidden; border: 1px solid rgba(255,255,255,0.1); background: #0d0508; }
    .story .shot .inner { position: absolute; left: 0; top: 0; width: 980px; height: 720px; transform: scale(0.3); transform-origin: 0 0; }
    .story .cap { margin-top: 10px; font-size: 13px; line-height: 18px; color: #a9a29b; }
    .story .cap b { display: block; color: #ece7e2; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 12px; margin-bottom: 2px; }
    .caption { position: absolute; left: 40px; top: 70px; font-size: 16px; font-weight: 600; }
  </style>"""

def over(base, extra):
    return page(W, H, f'<img class="frame" src="{base}">{CSS}{extra}')

def island(inner, cls="", fill=None, line=None):
    parts = ""
    if fill is not None:
        parts += f'<span class="fill" style="width:{fill}%;"></span>'
    if line is not None:
        parts += f'<span class="fill line" style="width:calc({line}% - 24px);"></span>'
    return f'<div class="isl {cls}">{parts}{inner}</div>'

OPEN = '''
  <div class="open">
    <div class="head"><span class="d"></span><span><b>Рисую</b> · Daisuke</span><span class="n">62 % · 18 с</span></div>
    <div class="body">
      <h3>Сейчас</h3>
      <div class="job"><span>Рисую · NaumRedlo — Daisuke</span><span class="n">62 %</span></div>
      <div class="bar"><i style="width:62%;"></i></div>
      <div class="job"><span>Скачиваю · xi — Blue Zenith</span><span class="n">14,2 / 21,8 МБ</span></div>
      <div class="bar"><i style="width:65%;"></i></div>
      <h3>Воркер</h3>
      <div class="job"><span class="d" style="display:block; width:8px; height:8px; border-radius:50%; background:#e24848; box-shadow:0 0 0 4px rgba(226,72,72,0.16);"></span><span>Готов брать работу</span><span class="n">2 в очереди · 14 сегодня</span></div>
      <h3>Уведомления</h3>
      <div class="line bad"><span class="g">✕</span><span>Рендер не завершился</span><span class="d">· 14:02</span></div>
      <div class="line done"><span class="g">✓</span><span>Карта скачана · Blue Zenith</span><span class="d">· 13:51</span></div>
      <div class="line done"><span class="g">✓</span><span>Отрендерено для @friend · отправлено</span><span class="d">· 12:20</span></div>
      <h3>Telegram</h3>
      <div class="who"><span class="ava"></span><div><div>@naumredlo</div><div class="n2">привязан · готовые рендеры уходят в этот чат</div></div></div>
      <div class="foot"><span>Сборка 0.12.0 · та же, что у бота</span><span style="margin-left:auto;">Esc — закрыть</span></div>
    </div>
  </div>'''

def boards():
    rest = frame("main-rest-ru-RU")
    rendered = frame("main-rendering-en-US")
    out = {}
    out["IslandQuiet"] = over(rest, island('<span class="d ok"></span>', "quiet") + '<div class="caption" style="top:66px; left:auto; right:318px; font-size:12px; font-weight:400; color:#a9a29b; text-align:right; width:360px;">в покое — либо ничего, либо едва заметная капля: 36 px, приглушённая точка, без текста</div>')
    out["IslandOne"] = over(rest, island('<span class="d"></span><span><b>Рисую</b> · Daisuke</span><span class="n">62 %</span>', fill=62))
    out["IslandLine"] = over(rest, island('<span class="d"></span><span><b>Рисую</b> · Daisuke</span><span class="n">18 с</span>', line=62))
    out["IslandTwo"] = over(rest, island('<span class="d"></span><span><b>Рисую</b></span><span class="n">62 %</span><span class="sep"></span><span><b>Скачиваю</b></span><span class="n">65 %</span>', fill=62))
    out["IslandDone"] = over(rest, island('<span class="v">✓</span><span><b>Готово</b> · Daisuke</span><span class="go">Открыть</span>', "tick"))
    out["IslandBad"] = over(rest, island('<span class="x">✕</span><span><b>Не вышло</b> · Daisuke</span><span class="go">Ещё раз</span>', "bad"))
    out["IslandWorker"] = over(rest, island('<span class="d"></span><span><b>Воркер</b> · для @friend</span><span class="n">3 из 7 · 2 в очереди</span>', fill=43))
    out["IslandOffline"] = over(rest, island('<span class="d ok"></span><span><b>Офлайн</b> · бот молчит</span><span class="n">повтор через 40 с</span>'))
    out["IslandOpen"] = over(rest, OPEN)
    quiet = island('<span class="d ok"></span>', "quiet")
    one = island('<span class="d"></span><span><b>Рисую</b> · Daisuke</span><span class="n">62 %</span>', fill=62)
    mid = '''<div class="open" style="width:320px;"><div class="head"><span class="d"></span><span><b>Рисую</b> · Daisuke</span><span class="n">62 %</span></div><div class="body" style="height:70px; opacity:0.5;"><h3>Сейчас</h3><div class="job"><span>Рисую · NaumRedlo — Daisuke</span></div></div></div>'''
    def shot(inner, cap_b, cap):
        return f'<div><div class="shot"><div class="inner"><img class="frame" src="{rest}" style="width:980px;height:720px;">{CSS}{inner}</div></div><div class="cap"><b>{cap_b}</b>{cap}</div></div>'
    out["IslandMorph"] = page(W, H, f'''<div style="position:absolute; inset:0; background:#0d0508;"></div>{CSS}
  <div class="caption">Как остров живёт</div>
  <div class="story">
    {shot(quiet, "покой → работа", "Капля растёт в таблетку за 320 мс, слово печатается, заливка ползёт слева направо вместе с делом.")}
    {shot(one, "клик или наведение", "Таблетка тянется вниз и вправо в карточку за 450 мс; заголовок остаётся её первой строкой, ничего не прыгает.")}
    {shot(mid, "дело кончилось", "Заливка добегает до края, слово меняется на «Готово · Открыть», остров держится 5 с и сжимается обратно в каплю.")}
  </div>''')
    return out

NOTES = {
    "IslandQuiet": ("Покой", "Остров сидит в верхней строке между знаком и тремя словами, у правого края: в покое — капля 36 px с приглушённой точкой (или ничего, если так тише). Не занимает места, не просит внимания."),
    "IslandOne": ("Одно дело · заливка", "Таблетка: дышащая точка, слово и имя дела, процент в моно. Состояние полосы — сама таблетка наливается мягким акцентом слева направо; когда заливка дошла до края, дело сделано."),
    "IslandLine": ("Одно дело · линия", "Тот же остров, но прогресс — двухпиксельная линия по низу, как у кнопок в просмотре, а вместо процента — сколько осталось. Одна грамматика на всё приложение."),
    "IslandTwo": ("Два дела", "Остров делится хайрлайном на два сегмента, у каждого своё слово и процент; заливка — по первому, второе ждёт своей очереди в карточке."),
    "IslandDone": ("Готово", "Точка становится галочкой, слово — «Готово», справа ссылка «Открыть». Пять секунд — и остров сжимается обратно."),
    "IslandBad": ("Не вышло", "Край в цвет опасности, крест вместо точки, «Ещё раз» справа. Остров не уходит сам — только по клику или после повтора; причина ждёт в карточке."),
    "IslandWorker": ("Воркер", "Когда устройство рисует для бота: «Воркер · для @friend», доля кадров, очередь. Тот же остров, тот же ход заливки."),
    "IslandOffline": ("Офлайн", "Состояние, а не ошибка: точка гаснет, «Офлайн · бот молчит», когда попробует снова. Уходит сам, когда связь вернулась."),
    "IslandOpen": ("Раскрыт · панель справа сверху", "Клик по острову (или наведение с задержкой) растягивает его в карточку 460 px к правому краю: первая строка — тот же остров; ниже Сейчас с полосами, Воркер, Уведомления с временем, Telegram, строка сборки. Esc или клик мимо сжимает обратно."),
    "IslandMorph": ("Переходы", "Три момента жизни острова: рост из капли, раскрытие в карточку, схлопывание после дела."),
}

RECOMMENDED = "Остров — одно живое место в верхней строке, оно же панель. Заливка таблетки (IslandOne) выразительнее линии, но линия (IslandLine) повторяет кнопки просмотра; предлагаю заливку для острова и линию для кнопок — они читаются на разном масштабе. Точки в углу нет вовсе."

if __name__ == "__main__":
    out = boards()
    for name, html in out.items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote", len(out), "island boards")
