from pathlib import Path

from built import frame, page

HERE = Path(__file__).parent

W, H = 980, 720

CSS = """
  <style>
    .bub { position: absolute; width: 300px; border-radius: 10px; background: rgba(12,5,7,0.97); border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 8px 24px rgba(0,0,0,0.45); box-sizing: border-box; padding: 8px 10px; color: #ece7e2; font-size: 12px; line-height: 16px; }
    .bub::after { content: ""; position: absolute; left: 50%; bottom: -8px; width: 0; height: 0; margin-left: -8px; border-left: 8px solid transparent; border-right: 8px solid transparent; border-top: 8px solid rgba(12,5,7,0.97); }
    .bub .h { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 14px; line-height: 18px; }
    .bub .h .r { margin-left: auto; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-weight: 700; font-size: 12px; }
    .bub .m { color: #a9a29b; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .bub .mono { font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; color: #a9a29b; white-space: nowrap; display: flex; align-items: center; gap: 6px; margin-top: 3px; overflow: hidden; }
    .bub .mono > * { flex: none; }
    .bub .mono .cut { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
    .bub .mono b { color: #ece7e2; font-weight: 700; }
    .bub .mono .k { color: #6b655f; }
    .bub .mono .sep { color: #6b655f; }
    .bub .c300 { color: #58aefc; } .bub .c100 { color: #8cd04a; } .bub .c50 { color: #f0c040; } .bub .cx { color: #e24848; }
    .bub .S { color: #f0c040; } .bub .SS { color: #f7e08a; } .bub .A { color: #8cd04a; }
    .bub .mod { display: inline-block; padding: 0 4px; border-radius: 3px; background: #4d9ef0; color: #fff; font-weight: 700; font-size: 9px; line-height: 14px; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; }
    .bub .ok { color: #a9a29b; font-weight: 700; } .bub .no { color: #e24848; font-weight: 700; }
    .bub .link { color: #e24848; font-weight: 600; }
    .bub .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 0 14px; margin-top: 3px; }
    .bub .cols .mono { margin-top: 2px; }
    .bub .foot { margin-top: 5px; padding-top: 5px; border-top: 1px solid rgba(255,255,255,0.06); }
    .zoom { position: absolute; top: 150px; transform: scale(2); transform-origin: top left; }
    .zoom .bub::after { display: none; }
    .zl { position: absolute; left: 50%; top: 122px; transform: translateX(-50%); color: #6b655f; font-family: "JetBrains Mono", ui-monospace, Menlo, monospace; font-size: 11px; }
    .lift { position: absolute; left: 178px; top: 647px; width: 108px; height: 61px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.7); box-sizing: border-box; }
  </style>"""

def bubble(inner, width=300):
    return f'<div class="bub" style="width:{width}px;">{inner}</div>'

def board(base, inner, width=300):
    left = 232 - width // 2
    at = f'<div class="lift"></div><div class="bub" style="left:{left}px; bottom:{720 - 647 + 14 + 8}px; width:{width}px;">{inner}</div>'
    zoom = f'<div class="zl">×2</div><div class="zoom" style="left:{490 - width}px;">{bubble(inner, width)}</div>'
    return page(W, H, f'<img class="frame" src="{base}">{CSS}{at}{zoom}')

NOW = '''<div class="h">Guest<span class="r">98,50%</span></div>
  <div class="m">Blue Zenith</div>
  <div class="mono"><span class="mod">EZ</span><span class="sep">·</span>401x<span class="sep">·</span><b>SB</b><span class="sep">·</span><b class="S">S</b></div>'''

COUNTS = '''<div class="h">Guest<span class="r"><span class="S">S</span> · 98,50%</span></div>
  <div class="m">xi — Blue Zenith</div>
  <div class="mono"><span class="c300">300</span><b>1204</b><span class="sep">·</span><span class="c100">100</span><b>18</b><span class="sep">·</span><span class="c50">50</span><b>2</b><span class="sep">·</span><span class="cx">✕</span><b>0</b></div>
  <div class="mono">401x <span class="k">из 1224</span><span class="sep">·</span><b>SB</b> <span class="k">на 1:32</span><span class="sep">·</span><span class="k">lazer · 14 авг 18:40</span></div>'''

LONG = '''<div class="h">Deeo_XD<span class="r"><span class="A">A</span> · 94,12%</span></div>
  <div class="m">t+pazolite with siromaru — Chambarising Stream Practice 160 BPM</div>
  <div class="mono"><span class="c300">300</span><b>812</b><span class="sep">·</span><span class="c100">100</span><b>61</b><span class="sep">·</span><span class="c50">50</span><b>9</b><span class="sep">·</span><span class="cx">✕</span><b>4</b></div>
  <div class="mono">233x <span class="k">из 1108</span><span class="sep">·</span><b class="cx">×4</b><span class="sep">·</span><span class="k">stable · 3 авг 22:15</span></div>'''

MAP = '''<div class="h">xi — Blue Zenith<span class="r">★ 7,12</span></div>
  <div class="m">[FOUR DIMENSIONS] · Nattu</div>
  <div class="mono">200 BPM<span class="sep">·</span>2:14<span class="sep">·</span>AR 9,4<span class="sep">·</span>CS 4<span class="sep">·</span>OD 9</div>
  <div class="mono"><span class="ok">✓</span>карта на диске<span class="sep">·</span>фон есть<span class="sep">·</span>1224 нот</div>'''

DONE = '''<div class="h">Guest<span class="r">98,50%</span></div>
  <div class="mono"><span class="ok">✓</span>отрендерено 14 авг<span class="sep">·</span>51,0 МБ<span class="sep">·</span><span class="ok">✓</span>в Telegram</div>
  <div class="mono"><span class="ok">✓</span>карта на диске<span class="sep">·</span>lazer<span class="sep">·</span>14 авг 18:40</div>
  <div class="mono"><span class="k">Guest - xi - Blue Zenith [FOUR DIMENSIONS] (2026-08-14).osr · 214 КБ</span></div>'''

DONE_NONE = '''<div class="h">Saki-chan<span class="r">81,20%</span></div>
  <div class="mono"><span class="no">✕</span>карты нет<span class="sep">·</span><span class="link">Скачать</span></div>
  <div class="mono"><span class="k">не рендерилось</span><span class="sep">·</span>stable<span class="sep">·</span>3 авг 22:15</div>'''

ALL = '''<div class="h">Guest<span class="r"><span class="S">S</span> · 98,50%</span></div>
  <div class="mono"><span class="c300">300</span><b>1204</b><span class="sep">·</span><span class="c100">100</span><b>18</b><span class="sep">·</span><span class="c50">50</span><b>2</b><span class="sep">·</span><span class="cx">✕</span><b>0</b><span class="sep">·</span>401x<span class="k">/1224</span></div>
  <div class="mono">★ 7,12<span class="sep">·</span>200 BPM<span class="sep">·</span>2:14<span class="sep">·</span>Nattu<span class="sep">·</span><span class="k">lazer · 18:40</span></div>
  <div class="mono foot"><span class="ok">✓</span>видео 51,0 МБ<span class="sep">·</span><span class="ok">✓</span>Telegram<span class="sep">·</span><span class="ok">✓</span>карта</div>'''

TWO = '''<div class="h">Guest<span class="r"><span class="S">S</span> · 98,50%</span></div>
  <div class="cols">
    <div>
      <div class="mono"><span class="c300">300</span><b>1204</b></div>
      <div class="mono"><span class="c100">100</span><b>18</b></div>
      <div class="mono"><span class="c50">50</span><b>2</b></div>
      <div class="mono"><span class="cx">✕</span><b>0</b></div>
    </div>
    <div>
      <div class="mono">★ 7,12<span class="sep">·</span>200 BPM</div>
      <div class="mono">401x <span class="k">из 1224</span></div>
      <div class="mono">Nattu<span class="sep">·</span>2:14</div>
      <div class="mono"><span class="k">lazer · 14 авг 18:40</span></div>
    </div>
  </div>
  <div class="mono foot"><span class="ok">✓</span>видео 51,0 МБ<span class="sep">·</span><span class="ok">✓</span>Telegram</div>'''

def boards():
    rest = frame("main-rest-ru-RU")
    return {
        "BubbleNow": board(rest, NOW, 260),
        "BubbleCounts": board(rest, COUNTS, 300),
        "BubbleLong": board(rest, LONG, 300),
        "BubbleMap": board(rest, MAP, 300),
        "BubbleDone": board(rest, DONE, 320),
        "BubbleNone": board(rest, DONE_NONE, 280),
        "BubbleAll": board(rest, ALL, 320),
        "BubbleTwo": board(rest, TWO, 300),
    }

NOTES = {
    "BubbleNow": ("Пузырь · как сейчас", "Игрок, точность, карта, моды · комбо · исход · оценка. Всё это уже написано во вьюере, стоит кликнуть — пузырь ничего не добавляет."),
    "BubbleCounts": ("Пузырь · судейство · выбран", "Игрок с оценкой и точностью в шапке, под ним исполнитель — песня, затем 300 · 100 · 50 · промахи, комбо из максимального, исход и где он случился, клиент и время. Всё уже лежит в реплее."),
    "BubbleLong": ("Пузырь · длинное имя", "Длинная строка режется многоточием внутри пузыря, за край ничего не выходит: ширина 300 px постоянна, каждая строка — одна и обрезана."),
    "BubbleMap": ("Пузырь · карта", "Пузырь про карту, а не про игру: сложность звёздами, маппер, BPM, длина, AR/CS/OD, есть ли карта и фон на диске, сколько нот. BPM, длина, маппер и настройки — из .osu, даром; звёзды нужен счётчик сложности."),
    "BubbleDone": ("Пузырь · что с ним сделано", "Состояние реплея в приложении: отрендерен ли, размер видео, ушло ли в Telegram, есть ли карта, клиент, время, имя файла."),
    "BubbleNone": ("Пузырь · без карты", "Тот же пузырь для реплея без карты: крест, «карты нет» и Скачать прямо отсюда, не рендерилось, клиент и время."),
    "BubbleAll": ("Пузырь · всё, чего нет во вьюере", "Игрок с оценкой и точностью в шапке; судейство и комбо из максимума; карта — звёзды, BPM, длина, маппер, клиент и время; в подвале — видео, Telegram, карта галочками. 320 px, четыре строки."),
    "BubbleTwo": ("Пузырь · две колонки", "То же в двух колонках: слева судейство столбиком, справа карта и время. Читается как табличка; выше на 20 px, чем BubbleAll."),
}

RECOMMENDED = "Принято 2026-09-21: BubbleCounts с исполнителем и песней под именем игрока; строки не выходят за край — обрезаются многоточием. Остальное — для записи. Прежнее предложение: BubbleAll — пузырь говорит только то, чего вьюер не говорит: судейство и комбо из максимума, факты карты, и что с реплеем уже сделано. Игрок остаётся шапкой, чтобы было ясно, о ком речь. Звёзды — когда появится счётчик; до него строка карты без них."

if __name__ == "__main__":
    for name, html in boards().items():
        (HERE / f"{name}.dc.html").write_text(html)
    print("wrote bubble boards")
