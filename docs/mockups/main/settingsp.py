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

def boards():
    out = {}
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
    "SettingsColumn": ("Настройки · колонка", "Одна прокручиваемая колонка карточек. Просто и в духе меню-плиток; для шести разделов уже длинновато."),
    "SettingsSidebar": ("Настройки · разделы", "Слева разделы с подписью, справа карточки выбранного — Системные настройки macOS. Ясно, что где; предлагаемый вариант."),
    "SettingsTiles": ("Настройки · плитки", "Все разделы плитками в две колонки. Всё сразу перед глазами, но плотно и на узком окне рассыпается в колонку."),
    "SettingsRender": ("Раздел · Рендер", "Сегменты, выключатели, подписи-объяснения; расчёт размера и времени для выбранного реплея; «Как у бота» возвращает его значения."),
    "SettingsSources": ("Раздел · Источники и Хранилище", "Папки osu! и свои с выключателями; хранилище видео и карт, кэш."),
}

RECOMMENDED = "Предложение: разделы слева (SettingsSidebar) с карточками справа — шесть разделов: Общее · Источники · Рендер · Хранилище · Инструменты · О программе. Что настраивается: язык, имя устройства, живая сцена; папки реплеев; размер, к/с, качество, фон/сториборд/видео, звук нажатий; папки видео и карт, кэш; ffmpeg, адрес бота, обновления. Значения рендера по умолчанию — как у бота, и кнопка вернуть их."

if __name__ == "__main__":
    for name, html in boards().items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote settings boards")
