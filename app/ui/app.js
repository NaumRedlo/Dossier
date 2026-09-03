// In its own file, not in a `<script>` tag: the window's content policy is
// `default-src 'self'`, which blocks an inline script — a page that worked
// perfectly in a browser and showed nothing at all in the application.

/// Куда идёт жалоба на баг. Почта и репозиторий известны; ник в Telegram знаете
/// только вы — впишите его сюда без «собаки», и кнопка появится сама.
/// Как зовётся то, чем движок рисует, когда своего скина не назвали. Это не
/// папка нигде на диске, поэтому имя, а не путь.
const DEFAULT_SKIN = "Dossier Default";

const CONTACT = {
  mail: "naumredlo@zohomail.com",
  telegram: "",
  repo: "https://github.com/NaumRedlo/Dossier",
};

/// The bridge, looked up when it is used rather than when this is parsed. A
/// window that shows nothing and says nothing is the worst way to fail.
function invoke(name, args) {
  const bridge = window.__TAURI__;
  if (!bridge) return Promise.reject(new Error("нет моста к приложению"));
  return bridge.core.invoke(name, args);
}

function plural(n, one, few, many) {
  if (n % 10 === 1 && n % 100 !== 11) return one;
  if (n % 10 >= 2 && n % 10 <= 4 && !(n % 100 >= 12 && n % 100 <= 14)) return few;
  return many;
}

function el(tag, className, text) {
  const made = document.createElement(tag);
  if (className) made.className = className;
  if (text !== undefined) made.textContent = text;
  return made;
}

const byId = (id) => document.getElementById(id);

/// One line of a card: a sign, a name, what was said, and a remedy when it is
/// the sort of thing somebody can do something about.
function line({ mark, name, said, fix }) {
  const row = el("div", "row");
  row.append(el("span", `mark-sign ${mark[0]}`, mark[1]), el("span", "name", name), el("span", "said", said));
  if (fix) row.append(el("span", "fix", fix));
  return row;
}

const sign = (ok) => (ok === null || ok === undefined ? ["huh", "?"] : ok ? ["ok", "+"] : ["no", "!"]);

// ── свои выпадающие списки ─────────────────────────────────────────────

/// Системный `<select>` — единственная деталь окна, которую рисует не оно.
/// На macOS он приносит свою рамку, свою стрелку и свой шрифт, и посреди
/// тёмной панели читается как чужая кнопка.
///
/// Настоящий `<select>` остаётся в разметке и остаётся источником правды: он
/// хранит значение и рассылает `change`, поэтому всё, что было написано вокруг
/// него, продолжает работать, ничего не зная об этой обёртке.
function dressSelect(select) {
  if (select.dataset.dressed) {
    select.repaint();
    return;
  }
  select.dataset.dressed = "1";

  const box = el("div", "picker");
  select.parentNode.insertBefore(box, select);
  box.append(select);
  const button = el("button", "picked");
  const label = el("span", "who");
  button.append(label, el("span", "chev", "⌄"));
  box.append(button);

  // Список — в `<body>`, не в `.picker`: карточка вокруг него обрезает своё
  // содержимое ради скруглённых углов, и список, оставленный внутри неё,
  // обрезался бы точно так же. Здесь он сам решает, где встать, от
  // положения кнопки на экране, а не от того, что вокруг него лежит.
  const list = el("div", "options");
  list.hidden = true;
  document.body.append(list);

  function place() {
    const rect = button.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 16;
    const above = rect.top - 16;
    const flip = below < 120 && above > below;
    list.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - rect.width - 8))}px`;
    list.style.width = `${rect.width}px`;
    list.classList.toggle("above", flip);
    if (flip) {
      list.style.top = "";
      list.style.bottom = `${window.innerHeight - rect.top + 6}px`;
      list.style.maxHeight = `${Math.min(240, above)}px`;
    } else {
      list.style.bottom = "";
      list.style.top = `${rect.bottom + 6}px`;
      list.style.maxHeight = `${Math.min(240, Math.max(120, below))}px`;
    }
  }

  const close = () => {
    list.hidden = true;
    button.setAttribute("aria-expanded", "false");
  };

  select.repaint = () => {
    const chosen = select.options[select.selectedIndex];
    label.textContent = chosen ? chosen.textContent : "";
    list.replaceChildren(
      ...[...select.options].map((option, index) => {
        const row = el("button", `option${index === select.selectedIndex ? " on" : ""}`, option.textContent);
        row.addEventListener("click", () => {
          select.value = option.value;
          select.dispatchEvent(new Event("change", { bubbles: true }));
          select.repaint();
          close();
        });
        return row;
      }),
    );
  };

  button.addEventListener("click", (event) => {
    event.preventDefault();
    const opening = list.hidden;
    for (const other of document.querySelectorAll(".options")) other.hidden = true;
    if (opening) place();
    list.hidden = !opening;
    button.setAttribute("aria-expanded", String(opening));
  });
  document.addEventListener("pointerdown", (event) => {
    if (!box.contains(event.target) && !list.contains(event.target)) close();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
  window.addEventListener("resize", () => {
    if (!list.hidden) place();
  });
  window.addEventListener(
    "scroll",
    () => {
      if (!list.hidden) place();
    },
    true,
  );
  select.repaint();
}

function dressAll() {
  for (const select of document.querySelectorAll("select")) dressSelect(select);
}

// ── что помнит само окно ───────────────────────────────────────────────

// Сторона панели и живость — это про это окно, а не про воркера: боту всё
// равно, где у вас вкладки. Поэтому здесь, а не в `worker.env`.
function remembered(key, fallback) {
  try {
    return localStorage.getItem(`dossier.${key}`) || fallback;
  } catch {
    return fallback;
  }
}

function remember(key, value) {
  try {
    localStorage.setItem(`dossier.${key}`, value);
  } catch {
    // Приватное окно, запрещённое хранилище — не повод ломать вкладку.
  }
}

const still = () => document.body.classList.contains("still");
const vertical = () => document.body.classList.contains("dock-left");

// ── панель, которая растёт под курсором ────────────────────────────────

const dock = byId("dock");
const items = [...document.querySelectorAll(".item")];
const glider = byId("glider");

/// Насколько крупнее значок прямо под курсором, и как далеко это чувствуется.
/// Те же две величины, что у дока в macOS, и подобраны так же — на глаз, пока
/// не перестало казаться, что панель дышит.
const GROW = 0.42;
const REACH = 78;

/// Каждая вкладка в покое: где начинается, какой длины, где середина и какого
/// размера значок. Меряется без сдвигов и без увеличения — иначе меряли бы
/// собственный предыдущий ответ.
let base = null;

function relax() {
  dock.style.setProperty("--grow", "0px");
  for (const item of items) {
    item.style.setProperty("--tx", "0px");
    item.style.setProperty("--ty", "0px");
    const glyph = item.querySelector(".glyph");
    glyph.style.setProperty("--s", "1");
    glyph.style.setProperty("--lx", "0px");
    glyph.style.setProperty("--ly", "0px");
  }
  placeGlider();
}

function measure() {
  relax();
  const down = vertical();
  base = items.map((item) => {
    const box = item.getBoundingClientRect();
    const glyph = item.querySelector(".glyph").getBoundingClientRect();
    return {
      start: down ? box.top : box.left,
      size: down ? box.height : box.width,
      centre: down ? box.top + box.height / 2 : box.left + box.width / 2,
      glyph: down ? glyph.height : glyph.width,
    };
  });
  placeGlider();
}

/// Курсор в точке `pos` вдоль панели: растёт значок, а вкладки расступаются
/// ровно настолько, насколько он вырос. Считаем в точках, а не в процентах, —
/// иначе на длинной подписи «Готовность» соседи разъезжаются вдвое сильнее,
/// чем на короткой «Ферма».
function magnify(pos) {
  if (!base || still()) return;
  const scale = base.map((box) => 1 + GROW * Math.exp(-(((pos - box.centre) / REACH) ** 2)));
  const span = base.map((box, i) => box.size + box.glyph * (scale[i] - 1));
  const grew = span.reduce((a, b) => a + b, 0) - base.reduce((a, box) => a + box.size, 0);

  // Группа остаётся на месте серединой: панель не ползёт вбок от того, что на
  // неё смотрят.
  dock.style.setProperty("--grow", `${grew.toFixed(1)}px`);
  let run = base[0].start - grew / 2;
  let under = 0;
  base.forEach((box, i) => {
    const gap = i + 1 < base.length ? base[i + 1].start - box.start - box.size : 0;
    const shift = run + span[i] / 2 - box.centre;
    place(items[i], shift, scale[i]);
    if (items[i].getAttribute("aria-selected") === "true") under = shift;
    run += span[i] + gap;
  });
  placeGlider(under);
}

function place(item, shift, scale) {
  const down = vertical();
  item.style.setProperty(down ? "--ty" : "--tx", `${shift.toFixed(2)}px`);
  item.style.setProperty(down ? "--tx" : "--ty", "0px");
  const glyph = item.querySelector(".glyph");
  glyph.style.setProperty("--s", scale.toFixed(3));
  // И чуть в сторону от края, как значок в доке подаётся от кромки экрана.
  const lean = (((scale - 1) / GROW) * 1.6).toFixed(2);
  glyph.style.setProperty(down ? "--lx" : "--ly", `${lean}px`);
  glyph.style.setProperty(down ? "--ly" : "--lx", "0px");
}

/// Белая лепёшка под открытой вкладкой: где вкладка лежит плюс насколько её
/// подвинули.
///
/// Именно так, а не по её нарисованным координатам: вкладка едет с переходом,
/// и лепёшка, снятая с неё в тот же миг, отстаёт на шаг — а когда курсор
/// останавливается, так и остаётся стоять рядом.
function placeGlider(shift = 0) {
  const open = items.find((item) => item.getAttribute("aria-selected") === "true");
  if (!open) return;
  const down = vertical();
  const x = open.offsetLeft + (down ? 0 : shift);
  const y = open.offsetTop + (down ? shift : 0);
  glider.style.transform = `translate(${x.toFixed(2)}px, ${y.toFixed(2)}px)`;
  glider.style.width = `${open.offsetWidth}px`;
  glider.style.height = `${open.offsetHeight}px`;
}

/// Указатель шлёт события чаще, чем экран рисует кадры, и считать на каждое —
/// это считать втрое и показывать одно. Копим последнюю точку, отвечаем раз в
/// кадр.
let waiting = null;
dock.addEventListener("pointermove", (event) => {
  const pos = vertical() ? event.clientY : event.clientX;
  if (waiting !== null) {
    waiting = pos;
    return;
  }
  waiting = pos;
  requestAnimationFrame(() => {
    const where = waiting;
    waiting = null;
    magnify(where);
  });
});

// Въезд и возврат — с переходом, чтобы не дёргало от покоя к полному росту.
// Всё, что между ними, идёт за курсором напрямую.
const items_box = byId("items");
dock.addEventListener("pointerenter", () => {
  items_box.classList.add("easing");
  setTimeout(() => items_box.classList.remove("easing"), 240);
});
dock.addEventListener("pointerleave", () => {
  items_box.classList.add("easing");
  relax();
  setTimeout(() => items_box.classList.remove("easing"), 260);
});
window.addEventListener("resize", measure);

// ── переключатели вида ─────────────────────────────────────────────────

function pressed(group, chosen) {
  for (const button of group.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(button === chosen));
  }
}

function dockSide(side, save = true, slide = true) {
  const was = dock.getBoundingClientRect();
  document.body.classList.toggle("dock-left", side === "left");
  document.body.classList.toggle("dock-top", side !== "left");
  pressed(byId("seg-dock"), byId("seg-dock").querySelector(`[data-side="${side}"]`));
  if (save) remember("dock", side);

  // Переезд, а не телепорт: где панель была минус где она теперь — это и есть
  // сдвиг, с которого ей ехать обратно к нулю.
  if (slide && !still()) {
    const now = dock.getBoundingClientRect();
    dock.classList.add("jumping");
    dock.style.setProperty("--sx", `${(was.left - now.left).toFixed(1)}px`);
    dock.style.setProperty("--sy", `${(was.top - now.top).toFixed(1)}px`);
    void dock.offsetWidth;
    dock.classList.remove("jumping");
    dock.style.setProperty("--sx", "0px");
    dock.style.setProperty("--sy", "0px");
  }
  // Мерить есть смысл там, где панель встанет, а не там, где она сейчас едет.
  relax();
  setTimeout(measure, slide ? 540 : 0);
}

function sky(how, save = true) {
  document.body.classList.toggle("sky-still", how === "still");
  pressed(byId("seg-sky"), byId("seg-sky").querySelector(`[data-sky="${how}"]`));
  if (save) remember("sky", how);
}

function splashWhen(how, save = true) {
  pressed(byId("seg-splash"), byId("seg-splash").querySelector(`[data-splash="${how}"]`));
  if (save) remember("splash", how);
}

function idleAfter(minutes, save = true) {
  byId("s-idle").value = String(minutes);
  if (save) remember("idle", String(minutes));
  lastStir = 0;
  armIdle();
}

byId("s-idle").addEventListener("change", (event) => idleAfter(event.target.value));

function motion(how, save = true) {
  document.body.classList.toggle("still", how === "off");
  pressed(byId("seg-motion"), byId("seg-motion").querySelector(`[data-motion="${how}"]`));
  if (save) remember("motion", how);
  if (how === "off") relax();
}

for (const button of byId("seg-dock").querySelectorAll("button")) {
  button.addEventListener("click", () => dockSide(button.dataset.side));
}
for (const button of byId("seg-sky").querySelectorAll("button")) {
  button.addEventListener("click", () => sky(button.dataset.sky));
}
for (const button of byId("seg-splash").querySelectorAll("button")) {
  button.addEventListener("click", () => splashWhen(button.dataset.splash));
}
for (const button of byId("seg-motion").querySelectorAll("button")) {
  button.addEventListener("click", () => motion(button.dataset.motion));
}

// ── заставка ───────────────────────────────────────────────────────────

/// Что написано на обложке. Ни одной строки про то, как всё замечательно: это
/// перерыв, а не реклама.
const SAYINGS = [
  "Стабильность и правильная работа — превыше всего.",
  "Реплей помнит, где был курсор и что было нажато. Остальное приходится восстанавливать.",
  "Судейство считается, а не угадывается.",
  "Кадр за кадром, ровно так, как это было.",
  "Ничего из игры: свой разбор карты, свой судья, свой рисунок.",
];

/// Сколько молчать до обложки. Пять минут по умолчанию — достаточно долго,
/// чтобы не мешать тому, кто читает список реплеев, и достаточно коротко, чтобы
/// окно не стояло сутками с открытой вкладкой настроек. Меняется в настройках,
/// потому что «долго» у каждого своё.
const idleMs = () => Number(remembered("idle", "5")) * 60 * 1000;

const splash = byId("splash");
const splashView = byId("splash-view");
let asleep = false;
let idleTimer = null;
let lastStir = 0;
let scene_run = null;

/// Марка и звук. Оба лежат рядом с окном, оба грузятся один раз: заставка
/// открывается на первой секунде запуска, и подгружать в этот момент нечего.
/// Буква без плитки, если она нарисована, и плитка, если нет.
///
/// `icons/make.py` рисует обе из одного описания, но шрифт, которым набрана
/// буква, — покупной и живёт в репозитории бота: класть его сюда нельзя, а
/// значит `ui/letter.png` собирается там, где он есть. Пока его нет, заставка
/// показывает плитку и работает.
const markImage = new Image();
let markIsLetter = true;
markImage.onerror = () => {
  markImage.onerror = null;
  markIsLetter = false;
  markImage.src = "mark.png";
};
markImage.src = "letter.png";
const hitSound = new Audio("hit.wav");
hitSound.preload = "auto";

/// Сколько кольцо сходится и сколько сцена стоит после удара.
const APPROACH_MS = 1150;
const AFTER_HIT_MS = 620;

/// Кольцо подходит к букве и попадает по ней — та же нота, что рисует движок,
/// только вместо круга наша марка. Звук ровно в момент схождения: это и есть
/// осу, и другого способа сказать это одним кадром нет.
///
/// Заперта на всё время, что экран чёрный, — включая ожидание, — и на всё
/// время самой сцены. Раньше движение мыши во время загрузки гасило чёрный
/// экран за секунду до того, как на нём вообще было что показывать; теперь
/// от запуска до последнего кадра кольца это одна запертая полоса.
let locked = false;

/// Холст под размер окна. Меряется каждый кадр: заставка открывается раньше,
/// чем окно успевает разложиться, и первый замер бывает не тем — а буфер,
/// выставленный один раз, потом растягивает круг в овал.
function sizeSplash(c) {
  const dpr = window.devicePixelRatio || 1;
  const w = splash.clientWidth;
  const h = splash.clientHeight;
  if (splashView.width !== Math.round(w * dpr) || splashView.height !== Math.round(h * dpr)) {
    splashView.width = Math.round(w * dpr);
    splashView.height = Math.round(h * dpr);
  }
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, w, h);
  return { w, h };
}

/// Закрыть окно чёрным и не открывать его никому, включая курсор. Вызывается
/// первым делом при запуске, до единого `await`, — а сама анимация просится
/// только позже, когда всё, что могло украсть у нее кадр, уже позади.
function showBlackCover() {
  locked = true;
  asleep = true;
  byId("splash-say").textContent = "";
  splash.classList.remove("going");
  splash.hidden = false;
  // Чёрное — значит чёрное: холст держит последний нарисованный кадр, и без
  // этого экран ожидания оказался бы обрывком прошлой сцены.
  sizeSplash(splashView.getContext("2d"));
}

function playOpening() {
  const c = splashView.getContext("2d");
  const started = performance.now();
  let struck = false;
  locked = true;

  const frame = (now) => {
    const { w, h } = sizeSplash(c);
    const t = now - started;
    const mid = [w / 2, h / 2 - 10];
    const size = Math.min(w, h) * 0.16;
    const settling = Math.min(1, t / APPROACH_MS);

    // Круг, к которому кольцо и сходится. Он стоит на месте — по нему видно,
    // куда именно всё придёт, и без него сужение не о чем.
    c.globalAlpha = Math.min(1, t / 420) * 0.5;
    c.strokeStyle = "#e24848";
    c.lineWidth = 2;
    c.beginPath();
    c.arc(mid[0], mid[1], size, 0, Math.PI * 2);
    c.stroke();

    c.globalAlpha = Math.min(1, t / 420);
    const swell = struck ? 1 + 0.07 * Math.max(0, 1 - (t - APPROACH_MS) / 260) : 1;
    if (markImage.complete && markImage.naturalWidth) {
      // Буква занимает около трети своего холста, плитка — весь: чтобы обе
      // вставали в кольцо одного размера, множитель у них разный.
      const side = size * (markIsLetter ? 2.6 : 1.5) * swell;
      c.drawImage(markImage, mid[0] - side / 2, mid[1] - side / 2, side, side);
    }

    if (!struck) {
      // Равномерно, а не с торможением: игра сводит кольцо линейно, и по нему
      // считают, когда нажимать. Кольцо, которое замедляется на подлёте,
      // читается как анимация, а не как нота, к которой надо успеть.
      c.globalAlpha = Math.min(1, t / 300) * 0.9;
      c.strokeStyle = "#e24848";
      c.lineWidth = 2.5;
      c.beginPath();
      c.arc(mid[0], mid[1], size * (1 + 2.4 * (1 - settling)), 0, Math.PI * 2);
      c.stroke();
    } else {
      const since = (t - APPROACH_MS) / 420;
      if (since < 1) {
        c.globalAlpha = (1 - since) * 0.7;
        c.strokeStyle = "#fff";
        c.lineWidth = 2;
        c.beginPath();
        c.arc(mid[0], mid[1], size * (1 + since * 0.9), 0, Math.PI * 2);
        c.stroke();
      }
      // И свет от удара — то, чем игра отмечает попадание: аддитивное пятно
      // цвета ноты, которое расходится и гаснет. Из всей осу это самый
      // узнаваемый кадр.
      const lit = (t - APPROACH_MS) / 520;
      if (lit < 1) {
        c.save();
        c.globalCompositeOperation = "lighter";
        c.globalAlpha = (1 - lit) * 0.55;
        const glow = c.createRadialGradient(mid[0], mid[1], 0, mid[0], mid[1], size * (1.3 + lit * 1.8));
        glow.addColorStop(0, "rgba(226, 72, 72, 0.85)");
        glow.addColorStop(1, "rgba(226, 72, 72, 0)");
        c.fillStyle = glow;
        c.fillRect(0, 0, w, h);
        c.restore();
      }
    }
    c.globalAlpha = 1;

    if (!struck && t >= APPROACH_MS) {
      struck = true;
      hitSound.currentTime = 0;
      // Окно может не дать звуку идти без нажатия — тогда сцена просто тихая,
      // и это не повод её ронять.
      hitSound.play().catch(() => {});
    }
    if (t > APPROACH_MS + AFTER_HIT_MS) {
      locked = false;
      hideSplash();
      return;
    }
    scene_run = requestAnimationFrame(frame);
  };
  scene_run = requestAnimationFrame(frame);
}

// ── заставка в простое: игра, которая играет сама ──────────────────────

/// Ноты появляются, курсор идёт к ним и нажимает. Иногда мимо — потому что
/// сцена, где всё всегда попадает, перестаёт быть интересной на второй ноте, а
/// смотрят на неё минутами.
const IDLE_APPROACH_MS = 780;
const IDLE_GAP_MS = 620;

/// Цвета нот, пока скин не подгрузился или не назвал своих. Те же четыре, что
/// служат синтетическими тестовыми картами в остальном приложении.
const IDLE_PALETTE = ["#e24848", "#66ccff", "#88d64c", "#f0c060"];

/// Одна нота на поле 512 на 384 — те же размеры, что у игры.
function makeNote(x, y, at, number, colour) {
  const roll = Math.random();
  const worth = roll < 0.12 ? 0 : roll < 0.26 ? 100 : 300;
  const put = [Math.min(462, Math.max(50, x)), Math.min(339, Math.max(45, y))];
  return {
    x: put[0],
    y: put[1],
    // Куда курсор пришёл на самом деле — иначе при промахе, чем при попадании.
    // Считается один раз, здесь же: следующий отрезок пути начнётся ровно там,
    // где кончился этот, а не там, где нота стоит «по правде». Иначе на каждом
    // промахе курсор дёргало бы.
    landX: worth === 0 ? put[0] + 70 : put[0],
    landY: worth === 0 ? put[1] + 34 : put[1],
    at,
    number,
    colour,
    worth,
  };
}

/// Ноты не сыплются по полю наугад. В карте они складываются в фигуры — поток,
/// прыжок, разворот, — и случайные точки читаются как рябь, а не как чья-то
/// игра. Каждый вызов даёт одну такую фигуру целиком, с её собственным цветом
/// комбо и нумерацией с единицы, ровно как считает их сама игра.
function makePattern(after, colour) {
  const kind = Math.random();
  const notes = [];
  let when = after + IDLE_GAP_MS + Math.random() * 240;

  if (kind < 0.4) {
    // Поток: ноты одна за другой по дуге, вплотную.
    const count = 5 + Math.floor(Math.random() * 3);
    const gap = 155 + Math.random() * 55;
    const from = [110 + Math.random() * 292, 100 + Math.random() * 184];
    const turn = Math.random() * Math.PI * 2;
    const bend = (Math.random() - 0.5) * 0.55;
    for (let i = 0; i < count; i += 1) {
      const angle = turn + bend * i;
      notes.push(makeNote(from[0] + Math.cos(angle) * 52 * i, from[1] + Math.sin(angle) * 52 * i, when, i + 1, colour));
      when += gap;
    }
  } else if (kind < 0.72) {
    // Прыжок: далеко и редко — курсору есть куда лететь, и видно, как он летит.
    const count = 3 + Math.floor(Math.random() * 2);
    let at = [140 + Math.random() * 232, 100 + Math.random() * 184];
    for (let i = 0; i < count; i += 1) {
      notes.push(makeNote(at[0], at[1], when, i + 1, colour));
      when += 385 + Math.random() * 95;
      const away = Math.random() * Math.PI * 2;
      at = [at[0] + Math.cos(away) * 205, at[1] + Math.sin(away) * 145];
    }
  } else {
    // Разворот: туда и обратно между двумя местами.
    const count = 4 + Math.floor(Math.random() * 3);
    const here = [90 + Math.random() * 130, 80 + Math.random() * 200];
    const there = [300 + Math.random() * 130, 80 + Math.random() * 200];
    for (let i = 0; i < count; i += 1) {
      const spot = i % 2 ? there : here;
      notes.push(
        makeNote(spot[0] + (Math.random() - 0.5) * 44, spot[1] + (Math.random() - 0.5) * 44, when, i + 1, colour),
      );
      when += 255 + Math.random() * 70;
    }
  }
  return notes;
}

/// Одна нота — картинками скина, если они уже загрузились, иначе тем же
/// каркасом, что рисует «Судейство» без скина.
function drawIdleNote(c, note, px, py, r, colour, alpha, ringLeft) {
  c.globalAlpha = alpha;
  const side = r * 2;
  if (pics && pics.circle) {
    c.drawImage(tinted(pics.circle.image, colour), px(note.x) - r, py(note.y) - r, side, side);
    if (pics.overlay) {
      c.drawImage(pics.overlay.image, px(note.x) - r, py(note.y) - r, side, side);
    }
    const digit = pics.digits.length === 10 ? pics.digits[note.number % 10] : null;
    if (digit) {
      const high = r * 0.85;
      const wide = (digit.image.width / digit.image.height) * high;
      c.drawImage(digit.image, px(note.x) - wide / 2, py(note.y) - high / 2, wide, high);
    }
    if (ringLeft > 0 && pics.approach) {
      const ring = r * (1 + 2.2 * ringLeft) * 2;
      c.drawImage(tinted(pics.approach.image, colour), px(note.x) - ring / 2, py(note.y) - ring / 2, ring, ring);
    }
  } else {
    // Плотнее, чем в «Судействе»: там сквозь ноту смотрят на поле, здесь на
    // неё просто смотрят.
    c.strokeStyle = colour;
    c.lineWidth = Math.max(1.5, r * 0.12);
    c.globalAlpha = alpha * 0.22;
    c.fillStyle = colour;
    c.beginPath();
    c.arc(px(note.x), py(note.y), r, 0, Math.PI * 2);
    c.fill();
    c.globalAlpha = alpha;
    c.stroke();
    c.fillStyle = "rgba(255,255,255,0.92)";
    c.font = `600 ${r * 0.85}px ui-monospace, Menlo, monospace`;
    c.textAlign = "center";
    c.textBaseline = "middle";
    c.fillText(String(note.number), px(note.x), py(note.y));
    if (ringLeft > 0) {
      c.strokeStyle = colour;
      c.beginPath();
      c.arc(px(note.x), py(note.y), r * (1 + 2.2 * ringLeft), 0, Math.PI * 2);
      c.stroke();
    }
  }
  c.globalAlpha = 1;
}

function playIdle() {
  const c = splashView.getContext("2d");
  const started = performance.now();
  // Первая — не нота, а место, откуда курсор выходит: у самого первого отрезка
  // должно быть настоящее начало с настоящим временем, иначе он считается от
  // «сейчас минус сколько-то» и ведёт себя тем страннее, чем дольше смотрят.
  let notes = [{ x: 256, y: 192, landX: 256, landY: 192, at: 0, number: 0, colour: 0, worth: 300, resting: true }];
  let colour = 0;
  let said = 0;
  let saidAt = started;

  const frame = (now) => {
    const { w, h } = sizeSplash(c);
    const t = now - started;
    // Почти во весь экран — так же, как игра занимает поле, а не квадрат
    // посередине с полями со всех сторон.
    const scale = Math.min(w / 512, h / 384) * 0.88;
    const ox = (w - 512 * scale) / 2;
    const oy = (h - 384 * scale) / 2 - Math.min(w, h) * 0.02;
    const px = (x) => ox + x * scale;
    const py = (y) => oy + y * scale;
    const r = 42 * scale;
    const colourAt = (index) =>
      pics && pics.colours && pics.colours.length
        ? pics.colours[index % pics.colours.length]
        : IDLE_PALETTE[index % IDLE_PALETTE.length];

    // Фигурами вперёд, по одной за раз: пока на ленте меньше шести нот, к ней
    // добавляется целый узор со своим цветом. Условие — про длину, а не про
    // счётчик: счётчик растёт вместе с ней, и такой цикл не кончается.
    while (notes.length < 6) {
      colour += 1;
      notes.push(...makePattern(notes[notes.length - 1].at, colour));
    }
    notes = notes.filter((note) => t < note.at + 900);
    if (!notes.length) notes.push(...makePattern(t, colour));

    // Где курсор: между прошлой нотой и следующей, с разгоном и торможением, по
    // точкам приземления — не по центрам нот, чтобы промах не телепортировал.
    // Вторая половина кривой именно `1 - 2(1-a)²`, а не `1 - (1-a)²/2`: с
    // делением половины не сходятся на середине, кривая прыгает с 0.5 на 0.875,
    // и курсор ровно посреди каждого перелёта проскакивает треть пути одним
    // кадром. Это и был тот рывок.
    const where = (moment) => {
      const next = notes.find((note) => note.at > moment) || notes[notes.length - 1];
      const index = notes.indexOf(next);
      const prev = index > 0 ? notes[index - 1] : next;
      const span = Math.max(1, next.at - prev.at);
      const along = Math.min(1, Math.max(0, (moment - prev.at) / span));
      const eased = along < 0.5 ? 2 * along * along : 1 - 2 * (1 - along) ** 2;
      return [prev.landX + (next.landX - prev.landX) * eased, prev.landY + (next.landY - prev.landY) * eased];
    };
    const cursor = where(t);

    for (const note of [...notes].reverse()) {
      if (note.resting) continue;
      const since = t - note.at;
      const appears = note.at - IDLE_APPROACH_MS;
      if (t < appears) continue;
      const alpha = since < 0 ? Math.min(1, (t - appears) / 300) : Math.max(0, 1 - since / 320);
      if (alpha <= 0) continue;
      const ringLeft = since < 0 ? -since / IDLE_APPROACH_MS : 0;
      drawIdleNote(c, note, px, py, r, colourAt(note.colour), alpha, ringLeft);

      if (since >= 0) {
        // Что из неё вышло — над самой нотой, полсекунды.
        c.globalAlpha = alpha;
        c.fillStyle = WORTH[note.worth] || WORTH[0];
        c.font = `700 ${r * 0.8}px ui-monospace, Menlo, monospace`;
        c.textAlign = "center";
        c.textBaseline = "middle";
        c.fillText(SAID[note.worth] || "×", px(note.x), py(note.y) - since * 0.03 - r * 0.1);
        c.globalAlpha = 1;
      }
    }

    // След: треть секунды пути позади курсора. Реплей и есть путь курсора —
    // без него на экране просто точка, которая оказывается то тут, то там.
    c.strokeStyle = "rgba(255,255,255,0.2)";
    c.lineWidth = 1.5;
    c.lineJoin = "round";
    c.beginPath();
    for (let back = 14; back >= 0; back -= 1) {
      const was = where(t - back * 22);
      if (back === 14) c.moveTo(px(was[0]), py(was[1]));
      else c.lineTo(px(was[0]), py(was[1]));
    }
    c.stroke();

    if (pics && pics.cursor) {
      const side = Math.min(w, h) * 0.05;
      c.drawImage(pics.cursor.image, px(cursor[0]) - side / 2, py(cursor[1]) - side / 2, side, side);
    } else {
      c.fillStyle = "rgba(255,255,255,0.85)";
      c.beginPath();
      c.arc(px(cursor[0]), py(cursor[1]), 5, 0, Math.PI * 2);
      c.fill();
      c.strokeStyle = "rgba(255,255,255,0.35)";
      c.lineWidth = 1.5;
      c.beginPath();
      c.arc(px(cursor[0]), py(cursor[1]), 11, 0, Math.PI * 2);
      c.stroke();
    }

    // Строки движка сменяют друг друга сами, не спеша — их читают, а не
    // показывают.
    if (now - saidAt > 9000) {
      saidAt = now;
      said = (said + 1) % SAYINGS.length;
      const line = byId("splash-say");
      line.style.opacity = "0";
      setTimeout(() => {
        line.textContent = SAYINGS[said];
        line.style.opacity = "";
      }, 700);
    }

    scene_run = requestAnimationFrame(frame);
  };
  scene_run = requestAnimationFrame(frame);
}

/// После молчания — не то же самое, что при запуске: та сцена говорит, что
/// это за приложение, эта занимает глаз минутами, пока к окну не вернулись.
/// Сцена запуска идёт через [`showBlackCover`] и [`playOpening`] отдельно.
function showSplash() {
  if (asleep) return;
  asleep = true;
  byId("splash-say").textContent = SAYINGS[Math.floor(Math.random() * SAYINGS.length)];
  splash.classList.add("going");
  splash.hidden = false;
  void splash.offsetWidth;
  splash.classList.remove("going");
  loadPics().catch(() => {});
  playIdle();
}

function hideSplash() {
  if (!asleep || locked) return;
  asleep = false;
  if (scene_run) cancelAnimationFrame(scene_run);
  scene_run = null;
  splash.classList.add("going");
  setTimeout(() => {
    if (!asleep) splash.hidden = true;
  }, 600);
}

function armIdle() {
  const now = Date.now();
  if (now - lastStir < 5000) return;
  lastStir = now;
  clearTimeout(idleTimer);
  if (remembered("splash", "both") === "both") idleTimer = setTimeout(showSplash, idleMs());
}

function stirred() {
  hideSplash();
  armIdle();
}

for (const kind of ["pointerdown", "pointermove", "keydown", "wheel"]) {
  document.addEventListener(kind, stirred, { passive: true });
}
document.addEventListener("visibilitychange", () => {
  document.body.classList.toggle("away", document.hidden);
  if (!document.hidden) stirred();
});

// ── жук ────────────────────────────────────────────────────────────────

const bugbox = byId("bugbox");
const bug = byId("bug");
const bugpane = byId("bugpane");
let pinned = false;

function openBug(yes, byHand = false) {
  bugpane.hidden = !yes;
  bug.setAttribute("aria-expanded", String(yes));
  if (yes && byHand) byId("bug-text").focus({ preventScroll: true });
}

// Открывается нажатием и только им. Раскрывать окно жалобы от того, что мимо
// угла провели мышью, — навязчивость: жук шевелится на наведение, и этого
// довольно, чтобы понять, что он нажимается.
bug.addEventListener("click", () => {
  pinned = bugpane.hidden;
  openBug(pinned, true);
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !bugpane.hidden) {
    pinned = false;
    openBug(false);
  }
});
document.addEventListener("pointerdown", (event) => {
  if (pinned && !bugbox.contains(event.target)) {
    pinned = false;
    openBug(false);
  }
});

/// Что уходит вместе с жалобой, кроме самой жалобы: версия и что за машина.
/// Ни путей, ни токена, ни имени в сети — на баг этого хватает, а остальное
/// не наше дело.
async function tail() {
  try {
    const about = await invoke("about");
    return `\n\n—\nDossier ${about.version} · ${about.os} · ${about.cpu} · ${about.cores} ядер`;
  } catch {
    return "";
  }
}

const ROUTES = [
  {
    key: "mail",
    label: "Почтой",
    ready: () => CONTACT.mail,
    async link(said) {
      const body = encodeURIComponent(said + (await tail()));
      return `mailto:${CONTACT.mail}?subject=${encodeURIComponent("Dossier: баг")}&body=${body}`;
    },
  },
  {
    key: "issue",
    label: "На GitHub",
    ready: () => CONTACT.repo,
    async link(said) {
      const body = encodeURIComponent(said + (await tail()));
      return `${CONTACT.repo}/issues/new?title=${encodeURIComponent("Баг из приложения")}&body=${body}`;
    },
  },
  {
    key: "telegram",
    label: "В Telegram",
    ready: () => CONTACT.telegram,
    // В личный чат текст подставить нельзя — кладём его в буфер и открываем
    // чат, чтобы оставалось только вставить.
    async link(said) {
      const whole = said + (await tail());
      try {
        await navigator.clipboard.writeText(whole);
        note("текст в буфере — вставьте в чат");
      } catch {
        note("чат открыт, текст придётся перенести руками");
      }
      return `https://t.me/${CONTACT.telegram}`;
    },
  },
];

function note(text, bad = false) {
  const said = bugpane.querySelector(".fine");
  said.textContent = text;
  said.style.color = bad ? "var(--accent)" : "";
}

for (const route of ROUTES.filter((route) => route.ready())) {
  const button = el("button", "act small", route.label);
  button.addEventListener("click", async () => {
    const said = byId("bug-text").value.trim();
    if (!said) {
      note("напишите хоть пару слов — без них починить нечего", true);
      byId("bug-text").focus();
      return;
    }
    button.disabled = true;
    try {
      await invoke("open_link", { url: await route.link(said) });
    } catch (why) {
      note(`не открылось: ${why}`, true);
    } finally {
      button.disabled = false;
    }
  });
  byId("bug-routes").append(button);
}

// ── ссылка на репозиторий ──────────────────────────────────────────────

byId("repo").addEventListener("click", (event) => {
  event.preventDefault();
  invoke("open_link", { url: CONTACT.repo }).catch(() => {});
});

// ── обновление ─────────────────────────────────────────────────────────

const upbox = byId("upbox");
const uppane = byId("uppane");
let waitingUpdate = null;

function reportWhen(how, save = true) {
  pressed(byId("seg-report"), byId("seg-report").querySelector(`[data-report="${how}"]`));
  if (save) remember("report", how);
}

for (const button of byId("seg-report").querySelectorAll("button")) {
  button.addEventListener("click", () => reportWhen(button.dataset.report));
}

/// Спросить, есть ли новее. Тихо при запуске — новость показывает сама плашка,
/// а всплывающее окно при открытии приложения никто не просил.
async function lookForUpdate(loud = false) {
  let said;
  try {
    said = await invoke("update_look");
  } catch (why) {
    if (loud) byId("up-version").textContent = `${why}`;
    return;
  }
  waitingUpdate = said;
  byId("up-version").textContent =
    said.how === "none"
      ? `${said.version} · ${said.said}`
      : `${said.version} · отстаёт на ${said.behind} ${plural(said.behind, "коммит", "коммита", "коммитов")}`;

  const there = said.how === "git" || said.how === "dirty";
  upbox.hidden = !there;
  if (!there) return;

  byId("up-say").textContent = `Обновление · ${said.behind}`;
  byId("up-parts").replaceChildren(
    ...(said.parts.length ? said.parts : ["приложение"]).map((part) => el("li", null, part)),
  );
  byId("up-note").textContent = said.said;
  if (loud) openUpdate(true);
}

function openUpdate(yes) {
  uppane.hidden = !yes;
  byId("up").setAttribute("aria-expanded", String(yes));
}

byId("up").addEventListener("click", () => openUpdate(uppane.hidden));
byId("up-check").addEventListener("click", () => {
  byId("up-version").textContent = "спрашиваю…";
  lookForUpdate(false);
});
document.addEventListener("pointerdown", (event) => {
  if (!uppane.hidden && !upbox.contains(event.target)) openUpdate(false);
});

const upLog = byId("up-log");
let logged = "";

function logLine(text) {
  logged += `${text}\n`;
  upLog.textContent = logged;
  upLog.scrollTop = upLog.scrollHeight;
  // «Compiling dossier-sim v0.11.0» — та самая строка, ради которой журнал и
  // показывается: по ней видно, что именно сейчас собирается.
  const building = /^\s*Compiling\s+(\S+)/.exec(text);
  if (!building) return;
  for (const item of byId("up-parts").children) {
    if (item.textContent === building[1]) item.classList.add("built");
  }
}

if (window.__TAURI__ && window.__TAURI__.event) {
  window.__TAURI__.event.listen("updating", ({ payload }) => logLine(String(payload)));
}

byId("up-go").addEventListener("click", async () => {
  const go = byId("up-go");
  const note = byId("up-note");
  go.disabled = true;
  logged = "";
  upLog.hidden = false;
  upLog.textContent = "";
  byId("up-after").hidden = true;
  note.textContent = "обновляю…";
  try {
    await invoke("update_run");
    note.textContent = "готово — перезапустите приложение";
    byId("up-head").textContent = "Собрано";
  } catch (why) {
    note.textContent = `не собралось: ${why}`;
    offerReport();
  } finally {
    go.disabled = false;
  }
});

/// Что делать с журналом упавшей сборки. Ничего не уходит само, пока об этом
/// не попросили в настройках, и даже тогда об отправке говорится вслух.
async function offerReport() {
  const after = byId("up-after");
  const how = remembered("report", "ask");
  if (how === "never") {
    after.replaceChildren(el("span", "muted", "журнал никуда не отправлен — так настроено"));
    after.hidden = false;
    return;
  }
  if (how === "auto") {
    after.replaceChildren(el("span", "muted", "отправляю разработчику…"));
    after.hidden = false;
    sendReport(after);
    return;
  }
  const send = el("button", "act small", "Отправить разработчику");
  send.addEventListener("click", () => sendReport(after));
  const issue = el("button", "act small", "На GitHub");
  issue.addEventListener("click", async () => {
    const body = encodeURIComponent(`Сборка обновления не прошла.\n\n\`\`\`\n${logged.slice(-3000)}\n\`\`\``);
    await invoke("open_link", {
      url: `${CONTACT.repo}/issues/new?title=${encodeURIComponent("Обновление не собралось")}&body=${body}`,
    }).catch(() => {});
  });
  after.replaceChildren(send, issue);
  after.hidden = false;
}

async function sendReport(after) {
  try {
    const said = await invoke("send_report", { log: logged });
    after.replaceChildren(el("span", "muted", `${said} — вместе с версией, системой и журналом`));
  } catch (why) {
    after.replaceChildren(el("span", "muted", `отправить не вышло: ${why}`));
  }
}

// ── готовность и машина, теперь внутри настроек ────────────────────────

async function showReady() {
  const box = byId("ready-rows");
  const verdict = byId("ready-verdict");
  let rows;
  try {
    rows = await invoke("ready");
  } catch (why) {
    verdict.textContent = `не удалось спросить: ${why.message}`;
    verdict.className = "verdict bad";
    return [];
  }
  box.replaceChildren(
    ...rows.map((row) => line({ mark: sign(row.ok), name: row.name, said: row.said, fix: row.ok === false ? row.fix : "" })),
  );
  const stopped = rows.filter((row) => row.ok === false).length;
  verdict.textContent = stopped
    ? `${stopped} ${plural(stopped, "пункт", "пункта", "пунктов")} надо поправить`
    : "готово — можно брать работу";
  verdict.className = stopped ? "verdict bad" : "verdict";

  // И отдельной строкой то, что с диска не узнать: согласен ли бот работать с
  // такой сборкой. Дописывается, когда ответит, — остальной список мгновенный
  // и ждать сети не должен.
  const asking = line({ mark: ["huh", "?"], name: "сборка", said: "спрашиваю у бота…" });
  box.append(asking);
  invoke("handshake")
    .then((row) =>
      asking.replaceWith(
        line({ mark: sign(row.ok), name: row.name, said: row.said, fix: row.ok === false ? row.fix : "" }),
      ),
    )
    .catch((why) => asking.replaceWith(line({ mark: ["no", "!"], name: "сборка", said: `${why}` })));
  return rows;
}

function round(n, places = 0) {
  return Number(n).toLocaleString("ru-RU", { maximumFractionDigits: places });
}

async function showMachine() {
  const head = byId("machine-headline");
  const box = byId("machine-rows");
  let told;
  try {
    told = await invoke("profile");
  } catch (why) {
    head.replaceChildren(el("span", null, `не удалось измерить: ${why.message}`));
    return;
  }

  // The measurement first and largest: it is the only number here that says
  // what this machine will *do* rather than what it is.
  if (told.speed) {
    head.replaceChildren(
      el("b", null, round(told.speed.estimated)),
      el("span", null, `кадров в секунду · ${told.speed.width}×${told.speed.height} · ${told.capacity.threads} ${plural(told.capacity.threads, "поток", "потока", "потоков")}`),
    );
  } else {
    head.replaceChildren(el("span", null, told.could_not_measure || "измерить не удалось"));
  }

  const hardware = told.hardware;
  const rows = [
    { mark: ["ok", "·"], name: "процессор", said: hardware.cpu },
    { mark: ["ok", "·"], name: "ядер", said: String(hardware.cores) },
    { mark: ["ok", "·"], name: "память", said: hardware.memory_gb ? `${round(hardware.memory_gb, 1)} ГБ` : "неизвестно" },
    { mark: ["ok", "·"], name: "система", said: hardware.os },
    {
      mark: hardware.encoders.length ? ["ok", "+"] : ["huh", "?"],
      name: "кодировщики",
      said: hardware.encoders.length ? hardware.encoders.join(", ") : "только программный",
    },
    {
      mark: sign(told.capacity.take),
      name: "политика",
      said: told.capacity.reason,
    },
  ];
  if (told.speed) {
    rows.push({ mark: ["ok", "·"], name: "на поток", said: `${round(told.speed.per_thread)} кадров/с` });
  }
  box.replaceChildren(...rows.map(line));
}

// ── ферма ──────────────────────────────────────────────────────────────

async function showFarm() {
  const box = byId("f-rows");
  const said = byId("f-said");
  said.textContent = "";
  said.className = "verdict";
  let farm;
  try {
    farm = await invoke("farm");
  } catch (why) {
    box.replaceChildren();
    said.className = "verdict bad";
    said.textContent = `ферму не видно: ${why}`;
    return;
  }
  byId("f-waiting").textContent = farm.waiting
    ? `${farm.waiting} ${plural(farm.waiting, "задача", "задачи", "задач")} в очереди`
    : "очередь пуста";
  if (!farm.workers.length) {
    box.replaceChildren(line({ mark: ["huh", "?"], name: "никого", said: "ни одна машина не отзывалась" }));
    return;
  }
  box.replaceChildren(
    ...farm.workers.map((worker) =>
      line({
        mark: worker.state === "rendering" ? ["ok", "▸"] : worker.state === "idle" ? ["ok", "·"] : ["huh", "?"],
        name: worker.name,
        said: [
          worker.reason || worker.state,
          `${worker.threads} ${plural(worker.threads, "поток", "потока", "потоков")}`,
          worker.build,
          `сделано ${worker.delivered}, вернул ${worker.handed_back}`,
        ].filter(Boolean).join(" · "),
      }),
    ),
  );
}

// ── настройки: одно место, откуда всё читается и куда всё пишется ──────

const FIELDS = ["server", "token", "name", "songs", "skins", "replays", "skin"];

/// Что сейчас записано в `worker.env`. Держим при себе, чтобы кнопка «Обзор»,
/// которая знает одно поле, не записала пустыми остальные пять.
let known = {};

async function loadSettings() {
  known = await invoke("settings_read").catch(() => ({}));
  for (const field of FIELDS) {
    const box = byId(`s-${field}`);
    if (box) box.value = known[field] || "";
  }
  nameChip(known);
  return known;
}

function nameChip(said) {
  // Только имя этой машины. Адрес бота отсюда убран совсем: он ничего не
  // говорит тому, кто смотрит на своё окно, а место занимает.
  byId("who").textContent = said.name || "";
}

async function saveSettings(prefix, saidId) {
  const said = { ...known };
  for (const field of FIELDS) {
    const box = byId(`${prefix}-${field}`);
    if (box) said[field] = box.value.trim();
  }
  const note = byId(saidId);
  try {
    await invoke("settings_write", { said });
    known = said;
    nameChip(said);
    // Молча. Форма пишется при каждом изменении, и «сохранено» рядом с кнопкой
    // повторяло очевидное на каждое нажатие клавиши. Слышно только когда не
    // вышло — вот это новость.
    if (note) note.textContent = "";
    return true;
  } catch (why) {
    if (note) {
      note.className = "verdict bad";
      note.textContent = `не сохранилось: ${why}`;
    }
    return false;
  }
}

/// Скорость мерится один раз на открытие окна: это маленький рендер, а
/// настройки открывают чаще, чем меняют железо.
let measured = false;

/// Поставить скин из архива. `.osk` — это zip под другим именем, и распаковать
/// его на полку значит просто получить скин, который видно во всех списках.
async function installSkin(path) {
  const note = byId("s-said");
  try {
    const name = await invoke("install_skin", { path });
    // Сначала в список, потом сохранять: запись читает поля формы, и скин,
    // которого в списке ещё нет, записался бы прежним значением.
    fillSkins(byId("s-skin"), await invoke("skins").catch(() => []), name);
    await saveSettings("s", "s-said");
    note.className = "verdict";
    note.textContent = `скин «${name}» поставлен и выбран по умолчанию`;
    showSettings();
    return true;
  } catch (why) {
    note.className = "verdict bad";
    note.textContent = `${why}`;
    return false;
  }
}

byId("s-osk").addEventListener("click", async () => {
  try {
    const path = await invoke("pick_skin", { prompt: "Скин .osk" });
    if (path) installSkin(path);
  } catch (why) {
    byId("s-said").textContent = `${why}`;
  }
});

/// Список скинов в любом выпадающем поле: встроенный первым, всегда.
function fillSkins(picker, skins, chosenName) {
  picker.replaceChildren(el("option", null, DEFAULT_SKIN));
  picker.firstChild.value = "";
  for (const name of skins) {
    const option = el("option", null, name);
    option.value = name;
    picker.append(option);
  }
  picker.value = skins.includes(chosenName) ? chosenName : "";
  dressSelect(picker);
}

const PROMPTS = {
  songs: "Папка с картами — обычно osu!/Songs",
  skins: "Папка со скинами — обычно osu!/Skins",
  replays: "Папка с реплеями — обычно osu!/Replays",
};

/// Выбрать папку системным окном и тут же записать. Путь, который выбрали, а
/// потом забыли нажать «Сохранить», — это путь, который не выбрали.
async function pickFolder(which) {
  try {
    const path = await invoke("pick_folder", { prompt: PROMPTS[which] });
    if (!path) return false;
    byId(`s-${which}`).value = path;
    return await saveSettings("s", "s-said");
  } catch (why) {
    const note = byId("s-said");
    note.className = "verdict bad";
    note.textContent = `${why}`;
    return false;
  }
}

for (const button of document.querySelectorAll("[data-pick]")) {
  button.addEventListener("click", async () => {
    if (await pickFolder(button.dataset.pick)) showSettings();
  });
}

async function showSettings() {
  await loadSettings();
  const skins = await invoke("skins").catch(() => []);
  fillSkins(byId("s-skin"), skins, known.skin);
  loadRenderSettings();
  const shelves = await invoke("shelves").catch(() => null);
  byId("s-shelves").hidden = !shelves;
  if (shelves) {
    byId("s-shelves").replaceChildren(
      ...[["карты", shelves.songs], ["скины", shelves.skins], ["реплеи", shelves.replays]].map(([name, shelf]) =>
        line({
          mark: shelf.exists ? ["ok", "+"] : ["huh", "?"],
          name,
          said: shelf.exists ? shelf.note : `${shelf.note} — ${shelf.path}`,
        }),
      ),
    );
  }
  showReady();
  // Мерить скорость — это маленький рендер. Один раз на открытие окна, дальше
  // по кнопке: настройки открывают чаще, чем железо меняется.
  if (!measured) {
    measured = true;
    showMachine();
  }
}

byId("s-save").addEventListener("click", () => saveSettings("s", "s-said"));
byId("s-skin").addEventListener("change", () => saveSettings("s", "s-said"));
byId("again").addEventListener("click", showReady);
byId("measure").addEventListener("click", () => {
  measured = true;
  showMachine();
});
byId("f-again").addEventListener("click", showFarm);

// ── библиотека ─────────────────────────────────────────────────────────

const TAGS = { builtin: "встроен", found: "на месте", missing: "нет", planned: "в планах" };

async function showLibrary() {
  const box = byId("lib-rows");
  let mods;
  try {
    mods = await invoke("modules");
  } catch (why) {
    box.replaceChildren(line({ mark: ["no", "!"], name: "библиотека", said: `${why}` }));
    return;
  }
  const groups = [
    ["Досьер", (mod) => mod.state === "builtin"],
    ["Нужно снаружи", (mod) => mod.state !== "builtin" && mod.state !== "planned"],
    ["В планах", (mod) => mod.state === "planned"],
  ];
  const rows = [];
  for (const [name, belongs] of groups) {
    const mine = mods.filter(belongs);
    if (!mine.length) continue;
    rows.push(el("div", "group", name), ...mine.map(modRow));
  }
  box.replaceChildren(...rows);

  function modRow(mod) {
      const row = el("div", "mod");
      row.append(el("span", `tag ${mod.state}`, TAGS[mod.state] || mod.state));
      const middle = el("div");
      middle.append(el("b", null, mod.name), el("div", "what", mod.what));
      row.append(middle);
      if (mod.picks === "skins") {
        const box = el("div", "pair");
        const where = el("button", "act small", "Обзор…");
        where.addEventListener("click", async () => {
          if (await pickFolder("skins")) showLibrary();
        });
        const put = el("button", "act small", "Поставить .osk…");
        put.addEventListener("click", async () => {
          const path = await invoke("pick_skin", { prompt: "Скин .osk" }).catch(() => null);
          if (path && (await installSkin(path))) showLibrary();
        });
        box.append(where, put);
        row.append(box);
      } else if (mod.picks) {
        const go = el("button", "act small", "Обзор…");
        go.addEventListener("click", async () => {
          if (await pickFolder(mod.picks)) showLibrary();
        });
        row.append(go);
      } else if (mod.state === "missing" && mod.name === "ffmpeg") {
        const go = el("button", "act small", "Где взять");
        go.addEventListener("click", () => invoke("open_link", { url: "https://ffmpeg.org/download.html" }).catch(() => {}));
        row.append(go);
      } else {
        row.append(el("span"));
      }
      row.append(el("div", "said", mod.state === "missing" ? `${mod.said} · ${mod.fix}` : mod.said));
      return row;
  }
}

// ── рендер ─────────────────────────────────────────────────────────────

let drawing = false;

/// Куда идут отчёты движка. Одна и та же полоса кормит две вкладки, и ей надо
/// знать, кто её сейчас ждёт.
let watching = { bar: "r-bar", said: "r-said", share: "r-share" };

// ── рендер и монтаж, свёрнутые в угол ────────────────────────────────────

/// Что сейчас идёт, если идёт. `home` — вкладка, на которой у процесса есть
/// своя полная панель; в углу он появляется, только когда открыта другая.
let job = null;
/// Плашка гаснет — не переключать её видимость, пока это не кончится, иначе
/// переход на другую вкладку посреди затухания обрывает его рывком.
let jobFading = false;

const jobMini = byId("job-mini");

function startJob(home, label) {
  job = { home, label };
  jobFading = false;
  jobMini.classList.remove("leaving");
  byId("job-label").textContent = label;
  paintJob(0, "");
  syncJobMini();
}

function paintJob(percent, note) {
  byId("job-percent").textContent = round(percent);
  byId("job-bar").style.width = `${Math.min(100, Math.max(0, percent))}%`;
  byId("job-note").textContent = note;
}

/// Показать или спрятать по тому, где сейчас открыто. На своей вкладке процесс
/// и так виден целиком — плашка в углу там только повторяла бы то же самое.
function syncJobMini() {
  if (jobFading) return;
  jobMini.hidden = !(job && open_tab !== job.home);
}

/// Быстро, но плавно: полоса на миг доходит до конца, а сама плашка гаснет, а
/// не пропадает разом — и переключение вкладки посреди этого её не обрывает.
/// Если она и не показывалась — работа шла на своей же вкладке всё время — ей
/// нечего гасить, и она просто остаётся спрятанной.
function endJob(ok) {
  if (!job) return;
  paintJob(100, ok ? "готово" : "не вышло");
  const wasShown = !jobMini.hidden;
  job = null;
  if (!wasShown) return;
  jobFading = true;
  jobMini.classList.add("leaving");
  setTimeout(() => {
    jobMini.hidden = true;
    jobMini.classList.remove("leaving");
    jobFading = false;
  }, 280);
}

/// Чем рисовать. Всё это — настройка, а не решение, принимаемое заново перед
/// каждым рендером: лишние шесть полей над списком реплеев стояли там ради
/// одного случая из двадцати.
function options() {
  const [width, height] = remembered("size", "1920x1080").split("x").map(Number);
  return {
    skin: known.skin || null,
    width,
    height,
    fps: Number(remembered("fps", "60")),
    background: remembered("bg", "1") === "1",
    storyboard: remembered("sb", "1") === "1",
    mute: remembered("sound", "1") !== "1",
    fine: {
      crf: Number(remembered("crf", "20")),
      preset: remembered("preset", "medium"),
      music_level: Number(remembered("music", "1")),
      hitsound_level: Number(remembered("hits", "1")),
      threads: Number(remembered("threads", "0")),
      encoder_threads: Number(remembered("enc", "0")),
      dim: Number(remembered("dim", "0")),
      blur: Number(remembered("blur", "0")),
      video: remembered("video", "0") === "1",
      bare: remembered("bare", "0") === "1",
      cursor_rotate: remembered("rotate", "0") === "1" ? true : null,
      map_hitsounds: remembered("mapsounds", "1") === "1",
      skin_hitsounds: remembered("skinsounds", "1") === "1",
      kit: remembered("kit", "click"),
      pitch: Number(remembered("pitch", "1")),
      decay: Number(remembered("decay", "1")),
      kit_level: Number(remembered("kitlevel", "1")),
    },
  };
}

/// Те же настройки, но словами — над списком, чтобы было видно, чем сейчас
/// нарисуется, не уходя за ними.
function howItDraws() {
  const said = options();
  const off = [
    said.background ? "" : "без фона",
    said.storyboard ? "" : "без сториборда",
    said.mute ? "без звука" : "",
  ].filter(Boolean);
  return [
    `${said.width}×${said.height}`,
    `${said.fps} кадров`,
    said.skin || DEFAULT_SKIN,
    ...off,
  ].join(" · ");
}

const RENDER_FIELDS = [
  ["s-size", "size", "1920x1080"],
  ["s-fps", "fps", "60"],
  ["s-bg", "bg", "1"],
  ["s-sb", "sb", "1"],
  ["s-video", "video", "0"],
  ["s-sound", "sound", "1"],
  ["s-dim", "dim", "0"],
  ["s-blur", "blur", "0"],
  ["s-crf", "crf", "20"],
  ["s-preset", "preset", "medium"],
  ["s-music", "music", "1"],
  ["s-hits", "hits", "1"],
  ["s-rotate", "rotate", "0"],
  ["s-bare", "bare", "0"],
  ["s-threads", "threads", "0"],
  ["s-enc", "enc", "0"],
  ["s-mapsounds", "mapsounds", "1"],
  ["s-skinsounds", "skinsounds", "1"],
  ["s-kit", "kit", "click"],
  ["s-pitch", "pitch", "1"],
  ["s-decay", "decay", "1"],
  ["s-kitlevel", "kitlevel", "1"],
];

function loadRenderSettings() {
  for (const [id, key, fallback] of RENDER_FIELDS) {
    const box = byId(id);
    const value = remembered(key, fallback);
    if (box.type === "checkbox") box.checked = value === "1";
    else box.value = value;
    if (box.tagName === "SELECT") dressSelect(box);
  }
}

for (const [id, key] of RENDER_FIELDS) {
  byId(id).addEventListener("change", (event) => {
    const box = event.target;
    remember(key, box.type === "checkbox" ? (box.checked ? "1" : "0") : box.value);
  });
}

function stepCard(step) {
  const card = el("li", `step${step.done ? " done" : ""}`);
  card.append(el("span", "no", step.done ? "✓" : String(step.n)));
  card.append(el("b", null, step.name));
  card.append(el("p", null, step.note));
  if (!step.done && step.act) {
    const go = el("button", "act small", step.act);
    go.addEventListener("click", step.go);
    card.append(go);
  }
  return card;
}

/// Список реплеев, пока папка не изменилась. Разбор шестидесяти `.osr` и поиск
/// их карт — это секунды, и платить их каждый раз, когда сюда заглянули,
/// незачем.
let shelfCache = null;

async function showRender(again = false) {
  const list = byId("r-list");
  const said = byId("r-done");

  if (shelfCache && !again) {
    fillPlays(shelfCache);
    return;
  }
  // Сначала показать, что читаем, и только потом читать: вкладка, которая
  // молчит и не отвечает, выглядит как повисшая, а не как занятая.
  byId("r-count").textContent = "читаю папку реплеев…";
  list.replaceChildren(line({ mark: ["huh", "·"], name: "минуту", said: "разбираю реплеи и ищу их карты" }));

  const [settings, shelves, skins, plays, rows] = await Promise.all([
    invoke("settings_read").catch(() => ({})),
    invoke("shelves").catch(() => null),
    invoke("skins").catch(() => []),
    invoke("my_replays", {}).catch(() => []),
    invoke("ready").catch(() => []),
  ]);
  known = { ...known, ...settings };
  shelfCache = { shelves, skins, plays, rows };
  fillPlays(shelfCache);
}

function fillPlays({ shelves, skins, plays, rows }) {
  const list = byId("r-list");
  const ffmpeg = rows.find((row) => row.name === "ffmpeg");
  const steps = [
    {
      n: 1,
      name: "Папка реплеев",
      note: shelves && shelves.replays.exists ? shelves.replays.note : "откуда брать, что рисовать",
      done: Boolean(shelves && shelves.replays.exists),
      act: "Обзор…",
      go: async () => (await pickFolder("replays")) && showRender(true),
    },
    {
      n: 2,
      name: "Папка карт",
      note: shelves && shelves.songs.exists ? shelves.songs.note : "по ней ищется карта, на которой играли",
      done: Boolean(shelves && shelves.songs.exists),
      act: "Обзор…",
      go: async () => (await pickFolder("songs")) && showRender(true),
    },
    {
      n: 3,
      name: "ffmpeg",
      note: ffmpeg && ffmpeg.ok ? "нашёлся" : "склеивает кадры в видео — без него рисовать некуда",
      done: Boolean(ffmpeg && ffmpeg.ok),
      act: "Где взять",
      go: () => invoke("open_link", { url: "https://ffmpeg.org/download.html" }).catch(() => {}),
    },
    {
      n: 4,
      name: "Скин",
      note: skins.length
        ? `${skins.length} ${plural(skins.length, "скин", "скина", "скинов")} на выбор`
        : `необязательно: без своего рисуется «${DEFAULT_SKIN}»`,
      done: true,
    },
  ];
  const left = steps.filter((step) => !step.done);
  byId("r-steps").replaceChildren(...(left.length ? steps.map(stepCard) : []));

  byId("r-count").textContent = plays.length
    ? `${plays.length} ${plural(plays.length, "реплей", "реплея", "реплеев")}, новые сверху`
    : "";
  if (!plays.length) {
    list.replaceChildren(line({ mark: ["huh", "?"], name: "пусто", said: "укажите папку реплеев — шаг первый" }));
    return;
  }
  list.replaceChildren(...plays.map((play) => playRow(play)));

  function playRow(play) {
    const row = el("div", "play");
    const left = el("div");
    left.append(el("b", null, play.player), el("div", "about", `${play.mods || "NM"} · ${round(play.score)} очков · комбо ${play.combo}`));
    row.append(left);
    row.append(play.have_map ? el("span", "about", "") : el("span", "nomap", "карты нет"));
    const go = el("button", "act small", "Отрендерить");
    go.disabled = !play.have_map;
    go.addEventListener("click", () => draw(play, go));
    row.append(go);
    return row;
  }

  async function draw(play, button) {
    if (drawing) return;
    drawing = true;
    for (const other of list.querySelectorAll("button")) other.disabled = true;
    watching = { bar: "r-bar", said: "r-said", share: "r-share" };
    startJob("render", `Рендер · ${play.player}`);
    const done_note = byId("r-done");
    done_note.className = "verdict";
    done_note.textContent = "";
    byId("r-busy").hidden = false;
    byId("r-bar").style.width = "0%";
    byId("r-share").textContent = "0";
    byId("r-said").textContent = `${play.player} · ${howItDraws()}`;
    list.classList.add("dimmed");
    const out = play.path.replace(/\.osr$/i, "") + ".mp4";
    try {
      const done = await invoke("draw", { replay: play.path, out, ...options() });
      done_note.textContent = `готово — ${done.path}`;
      for (const note of done.said) done_note.textContent += `\n${note}`;
      endJob(true);
    } catch (why) {
      done_note.className = "verdict bad";
      done_note.textContent = `не вышло: ${why}`;
      endJob(false);
    } finally {
      drawing = false;
      for (const other of list.querySelectorAll("button")) other.disabled = false;
      byId("r-busy").hidden = true;
      list.classList.remove("dimmed");
    }
  }
}

byId("r-refresh").addEventListener("click", () => showRender(true));

// The engine's own events, forwarded from the render — see the `draw` command.
if (window.__TAURI__ && window.__TAURI__.event) {
  window.__TAURI__.event.listen("drawing", ({ payload }) => {
    if (payload.event !== "progress" || !payload.of) return;
    const share = Math.min(100, (payload.frames / payload.of) * 100);
    byId(watching.bar).style.width = `${share}%`;
    if (watching.share) byId(watching.share).textContent = round(share);
    const note = `кадр ${round(payload.frames)} из ${round(payload.of)} · ${round(payload.per_second)} в секунду · осталось ${round(payload.left_seconds)} с`;
    byId(watching.said).textContent = note;
    if (job) paintJob(share, note);
  });
  window.__TAURI__.event.listen("reeling", ({ payload }) => {
    const note = `кусок ${payload.clip} из ${payload.of}…`;
    byId("rp-built").textContent = note;
    if (job) paintJob((payload.clip / payload.of) * 100, note);
  });
}

// ── реплей ─────────────────────────────────────────────────────────────

/// Цвета вердиктов. Те же, что рисует движок: окно и кадр не должны расходиться
/// в том, какого цвета сотка.
const WORTH = { 300: "#66ccff", 100: "#88d64c", 50: "#f0c060", 0: "#e24848" };

/// Каким рисуются ноты, пока не попросили скин. Это просмотр судейства, а не
/// показ карты: комбо-цвета здесь только отвлекают от того, что засчитано.
const PLAIN = "#c9cede";

/// Картинки скина, которым включается «Показывать со скином»: те же файлы,
/// которыми рисует движок, взятые из скина по умолчанию.
let pics = null;
const tints = new WeakMap();

async function loadPics() {
  if (pics) return pics;
  const said = await invoke("skin_pictures", {}).catch(() => null);
  if (!said) return null;
  const wait = (one) =>
    new Promise((ready) => {
      if (!one) return ready(null);
      const image = new Image();
      image.onload = () => ready({ image, scale: one.scale });
      image.onerror = () => ready(null);
      image.src = one.src;
    });
  pics = {
    circle: await wait(said.circle),
    overlay: await wait(said.overlay),
    approach: await wait(said.approach),
    cursor: await wait(said.cursor),
    digits: await Promise.all((said.digits || []).map(wait)),
  };
  return pics;
}

/// Белая картинка, покрашенная цветом комбо. Кэшируется на пару «картинка +
/// цвет»: перекрашивать её каждый кадр — это перерисовывать весь скин
/// шестьдесят раз в секунду.
function tinted(image, colour) {
  let per = tints.get(image);
  if (!per) {
    per = new Map();
    tints.set(image, per);
  }
  let made = per.get(colour);
  if (made) return made;
  made = document.createElement("canvas");
  made.width = image.width;
  made.height = image.height;
  const c = made.getContext("2d");
  c.drawImage(image, 0, 0);
  c.globalCompositeOperation = "multiply";
  c.fillStyle = colour;
  c.fillRect(0, 0, made.width, made.height);
  // Умножение красит и прозрачные места — возвращаем исходную маску.
  c.globalCompositeOperation = "destination-in";
  c.drawImage(image, 0, 0);
  per.set(colour, made);
  return made;
}
const SAID = { 300: "300", 100: "100", 50: "50", 0: "×" };

/// Сколько кадр живёт после того, как объект отыгран.
const AFTER_MS = 240;

let scene = null;
let judged = null;
let chosen = null;
let mode = null;
let head = 0;
let playing = null;
let clips = [];
let picked = -1;
let skinned = false;

const view = byId("rp-view");
const tape = byId("rp-tape");
const track = byId("rp-track");

// ── выбор режима ───────────────────────────────────────────────────────

/// Что нужно каждому режиму. Судейству — только движок, а он встроен; монтажу —
/// ffmpeg, без которого собирать нечем. Проверяется до того, как пустить, а не
/// после того, как человек нарезал ленту.
async function needs(which) {
  if (which !== "cut") return "";
  const rows = await invoke("ready").catch(() => []);
  const ffmpeg = rows.find((row) => row.name === "ffmpeg");
  return ffmpeg && ffmpeg.ok
    ? ""
    : "для монтажа нужен ffmpeg — «Библиотека» скажет, где его взять";
}

async function enter(which) {
  const gate = byId("rp-gate");
  gate.textContent = "проверяю, что для этого нужно…";
  const missing = await needs(which);
  if (missing) {
    gate.textContent = missing;
    return;
  }
  gate.textContent = "";
  mode = which;
  byId("rp-choose").hidden = true;
  byId("rp-stage").hidden = false;
  byId("rp-mode").textContent = which === "judge" ? "Судейство" : "Монтаж";
  byId("rp-what").textContent =
    which === "judge" ? "что засчитано, что нет и на сколько" : "куски игры и сборка из них";
  byId("rp-judge").hidden = which !== "judge";
  byId("rp-cut").hidden = which !== "cut";
  if (scene) showLive();
}

for (const button of document.querySelectorAll(".mode")) {
  button.addEventListener("click", () => enter(button.dataset.mode));
}

function saySkinned() {
  const button = byId("rp-skinned");
  button.hidden = !scene;
  button.textContent = skinned ? "Не показывать со скином" : "Показывать со скином";
  if (skinned) loadPics().then(() => scene && drawView());
}

byId("rp-skinned").addEventListener("click", async () => {
  skinned = !skinned;
  remember("skinned", skinned ? "1" : "0");
  saySkinned();
  if (skinned) await loadPics();
  drawView();
});

byId("rp-back").addEventListener("click", () => {
  stop();
  mode = null;
  byId("rp-stage").hidden = true;
  byId("rp-choose").hidden = false;
});

// ── открыть реплей ─────────────────────────────────────────────────────

async function openReplay(path) {
  const said = byId("rp-said");
  byId("rp-drop").hidden = false;
  said.textContent = "читаю и сужу…";
  let opened;
  try {
    opened = await invoke("judged", { replay: path });
  } catch (why) {
    said.textContent = `${why}`;
    return;
  }
  scene = opened;
  judged = opened.summary;
  density = null;
  lens = null;
  chosen = path;
  head = scene.from_ms;
  clips = [];
  picked = -1;
  said.textContent = "";
  showLive();
}

function showLive() {
  byId("rp-drop").hidden = true;
  byId("rp-live").hidden = false;
  byId("rp-what").textContent = `${judged.title} · ${judged.player} · ${judged.mods || "NM"}`;
  saySkinned();
  byId("rp-rows").replaceChildren(
    ...[
      { mark: ["ok", "·"], name: "точность", said: `${round(judged.accuracy_percent, 2)}%` },
      {
        mark: judged.combo === judged.combo_recorded ? ["ok", "·"] : ["huh", "?"],
        name: "комбо",
        said:
          judged.combo === judged.combo_recorded
            ? String(judged.combo)
            : `${judged.combo} · в реплее записано ${judged.combo_recorded}`,
      },
      { mark: ["ok", "·"], name: "300", said: String(judged.counts.great) },
      { mark: ["ok", "·"], name: "100", said: String(judged.counts.ok) },
      { mark: ["huh", "·"], name: "50", said: String(judged.counts.meh) },
      { mark: judged.counts.miss ? ["no", "!"] : ["ok", "·"], name: "промахи", said: String(judged.counts.miss) },
      {
        mark: ["ok", "·"],
        name: "разброс",
        said: judged.unstable_rate === null ? "не считается" : `${round(judged.unstable_rate, 1)} UR`,
      },
    ].map(line),
  );
  drawAll();
}

byId("rp-open").addEventListener("click", async () => {
  try {
    const path = await invoke("pick_replay", { prompt: "Реплей .osr" });
    if (path) openReplay(path);
  } catch (why) {
    byId("rp-said").textContent = `${why}`;
  }
});

// Перетаскивание — событие окна, а не страницы: у файла в webview нет пути, и
// знает его только сама Tauri. Заодно ловим и скины.
if (window.__TAURI__ && window.__TAURI__.event) {
  const zone = byId("rp-drop");
  window.__TAURI__.event.listen("tauri://drag-enter", () => zone.classList.add("over"));
  window.__TAURI__.event.listen("tauri://drag-leave", () => zone.classList.remove("over"));
  window.__TAURI__.event.listen("tauri://drag-drop", async ({ payload }) => {
    zone.classList.remove("over");
    const paths = payload.paths || [];
    const osk = paths.find((one) => /\.osk$/i.test(one));
    if (osk) {
      await installSkin(osk);
      return;
    }
    const replay = paths.find((one) => /\.osr$/i.test(one));
    if (!replay) return;
    show("replay");
    if (!mode) await enter("judge");
    openReplay(replay);
  });
}

// ── как это рисуется ───────────────────────────────────────────────────

/// Поле osu! — 512 на 384 единицы. Всё, что ниже, считает в них и переводит в
/// точки холста одним и тем же множителем, чтобы круг остался кругом.
function fit(w, h) {
  const scale = Math.min(w / 512, h / 384) * 0.9;
  return {
    scale,
    ox: (w - 512 * scale) / 2,
    oy: (h - 384 * scale) / 2,
    r: scene.radius * scale,
  };
}

/// Первый объект, который ещё может быть виден. Двоичным поиском, а не с
/// начала: на карте их бывает несколько тысяч, а кадров в секунду шестьдесят.
function firstVisible(ms) {
  let low = 0;
  let high = scene.objects.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (scene.objects[mid].end_ms + AFTER_MS < ms) low = mid + 1;
    else high = mid;
  }
  return low;
}

function cursorAt(ms) {
  const step = Math.round((ms - scene.from_ms) / scene.step_ms);
  const at = Math.min(Math.max(0, step), scene.keys.length - 1);
  return { x: scene.cursor[at * 2], y: scene.cursor[at * 2 + 1], keys: scene.keys[at], at };
}

function drawView() {
  if (!scene || view.clientWidth === 0) return;
  const dpr = window.devicePixelRatio || 1;
  const w = view.clientWidth;
  const h = view.clientHeight;
  view.width = Math.round(w * dpr);
  view.height = Math.round(h * dpr);
  const c = view.getContext("2d");
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, w, h);

  const box = fit(w, h);
  const px = (x) => box.ox + x * box.scale;
  const py = (y) => box.oy + y * box.scale;

  c.strokeStyle = "rgba(255,255,255,0.05)";
  c.strokeRect(box.ox, box.oy, 512 * box.scale, 384 * box.scale);

  // Собираем видимое, потом рисуем задом наперёд: ближайший по времени объект
  // должен лежать поверх тех, что придут после него, — так рисует и игра.
  const showing = [];
  for (let i = firstVisible(head); i < scene.objects.length; i += 1) {
    const piece = scene.objects[i];
    if (piece.start_ms - scene.preempt_ms > head) break;
    showing.push(piece);
  }
  for (let i = showing.length - 1; i >= 0; i -= 1) {
    drawPiece(c, showing[i], box, px, py);
  }

  drawPopups(c, box, px, py);
  drawCursor(c, box, px, py);
}

function drawPiece(c, piece, box, px, py) {
  const appears = piece.start_ms - scene.preempt_ms;
  const colour = skinned ? scene.colours[piece.colour] || "#e24848" : PLAIN;
  const fading = head > piece.end_ms ? 1 - (head - piece.end_ms) / AFTER_MS : 1;
  const alpha = Math.min(1, (head - appears) / Math.max(1, scene.fade_in_ms)) * Math.max(0, fading);
  if (alpha <= 0) return;
  c.globalAlpha = alpha;

  if (piece.kind === "slider" && piece.path.length >= 4) {
    c.strokeStyle = "rgba(255,255,255,0.22)";
    c.lineWidth = box.r * 2;
    c.lineJoin = "round";
    c.lineCap = "round";
    c.beginPath();
    c.moveTo(px(piece.path[0]), py(piece.path[1]));
    for (let i = 2; i < piece.path.length; i += 2) c.lineTo(px(piece.path[i]), py(piece.path[i + 1]));
    c.stroke();
    c.strokeStyle = "rgba(10,5,8,0.85)";
    c.lineWidth = Math.max(1, box.r * 2 - 5);
    c.stroke();
  }

  if (piece.kind === "spinner") {
    c.strokeStyle = colour;
    c.lineWidth = 2;
    const turning = Math.max(0, Math.min(1, (head - piece.start_ms) / Math.max(1, piece.end_ms - piece.start_ms)));
    c.beginPath();
    c.arc(px(256), py(192), 130 * box.scale * (1 - turning * 0.75), 0, Math.PI * 2);
    c.stroke();
    c.globalAlpha = 1;
    return;
  }

  const left = head < piece.start_ms ? (piece.start_ms - head) / Math.max(1, scene.preempt_ms) : 0;
  const side = box.r * 2;

  if (skinned && pics && pics.circle) {
    // Скин рисует себя сам: нота, её накладка, номер и кольцо — теми же
    // файлами, которыми это рисует движок.
    c.drawImage(tinted(pics.circle.image, colour), px(piece.x) - box.r, py(piece.y) - box.r, side, side);
    if (pics.overlay) {
      c.drawImage(pics.overlay.image, px(piece.x) - box.r, py(piece.y) - box.r, side, side);
    }
    if (pics.digits.length === 10) {
      const figures = String(piece.combo).split("");
      const high = box.r * 0.9;
      const wide = figures.map((d) => {
        const one = pics.digits[Number(d)];
        return one ? (one.image.width / one.image.height) * high : 0;
      });
      let at = px(piece.x) - wide.reduce((a, b) => a + b, 0) / 2;
      figures.forEach((d, i) => {
        const one = pics.digits[Number(d)];
        if (one) c.drawImage(one.image, at, py(piece.y) - high / 2, wide[i], high);
        at += wide[i];
      });
    }
    if (left > 0 && pics.approach) {
      const ring = box.r * (1 + 2.4 * left) * 2;
      c.drawImage(tinted(pics.approach.image, colour), px(piece.x) - ring / 2, py(piece.y) - ring / 2, ring, ring);
    }
  } else {
    // Без скина — каркас: сквозь ноту видно поле, и глаз занят тем, что
    // засчитано, а не тем, какого она цвета.
    c.fillStyle = "rgba(255,255,255,0.05)";
    c.beginPath();
    c.arc(px(piece.x), py(piece.y), box.r, 0, Math.PI * 2);
    c.fill();
    c.strokeStyle = colour;
    c.lineWidth = Math.max(1.5, box.r * 0.1);
    c.stroke();
    if (box.r > 9) {
      c.fillStyle = colour;
      c.font = `600 ${box.r * 0.9}px ui-monospace, Menlo, monospace`;
      c.textAlign = "center";
      c.textBaseline = "middle";
      c.fillText(String(piece.combo), px(piece.x), py(piece.y));
    }
    if (left > 0) {
      c.strokeStyle = colour;
      c.lineWidth = Math.max(1.2, box.r * 0.08);
      c.globalAlpha = alpha * 0.7;
      c.beginPath();
      c.arc(px(piece.x), py(piece.y), box.r * (1 + 2.4 * left), 0, Math.PI * 2);
      c.stroke();
      c.globalAlpha = alpha;
    }
  }

  // Шар слайдера — из тех же точек, что считал движок.
  if (piece.kind === "slider" && head >= piece.start_ms && head <= piece.end_ms && piece.ball.length) {
    // Шар — кружком в обоих режимах: у скина он анимированный, а анимацию
    // здесь пока никто не проигрывает, и подсунуть один её кадр было бы
    // хуже, чем не подсовывать ничего.
    const at = Math.min(
      piece.ball.length / 2 - 1,
      Math.max(0, Math.round((head - piece.start_ms) / scene.step_ms)),
    );
    c.fillStyle = colour;
    c.beginPath();
    c.arc(px(piece.ball[at * 2]), py(piece.ball[at * 2 + 1]), box.r * 0.9, 0, Math.PI * 2);
    c.fill();
    c.strokeStyle = "rgba(255,255,255,0.9)";
    c.lineWidth = 2;
    c.stroke();
  }
  c.globalAlpha = 1;
}

/// Числа, которые выскакивают на месте объекта. Полсекунды и вверх — ровно
/// столько, чтобы успеть заметить промах, и не столько, чтобы он мешал.
function drawPopups(c, box, px, py) {
  c.textAlign = "center";
  c.textBaseline = "middle";
  for (const mark of judged.marks) {
    const since = head - mark.ms;
    if (since < 0 || since > 600) continue;
    c.globalAlpha = 1 - since / 600;
    c.fillStyle = WORTH[mark.worth] || WORTH[0];
    c.font = `700 ${Math.max(11, box.r * 0.8)}px ui-monospace, Menlo, monospace`;
    c.fillText(SAID[mark.worth] || "×", px(mark.x), py(mark.y) - since * 0.02);
  }
  c.globalAlpha = 1;
}

function drawCursor(c, box, px, py) {
  const now = cursorAt(head);
  c.strokeStyle = "rgba(255,255,255,0.35)";
  c.lineWidth = 1.5;
  c.beginPath();
  for (let back = 12; back >= 0; back -= 1) {
    const at = Math.max(0, now.at - back);
    const point = [px(scene.cursor[at * 2]), py(scene.cursor[at * 2 + 1])];
    if (back === 12) c.moveTo(point[0], point[1]);
    else c.lineTo(point[0], point[1]);
  }
  c.stroke();

  const held = (now.keys & 15) !== 0;
  c.fillStyle = held ? "#fff" : "rgba(255,255,255,0.75)";
  c.beginPath();
  c.arc(px(now.x), py(now.y), held ? 6 : 4.5, 0, Math.PI * 2);
  c.fill();
  if (held) {
    c.strokeStyle = "rgba(255,255,255,0.5)";
    c.lineWidth = 2;
    c.beginPath();
    c.arc(px(now.x), py(now.y), 12, 0, Math.PI * 2);
    c.stroke();
  }
}

/// Что было к этому моменту: считается по тем же меткам, что нарисованы, —
/// второго источника правды здесь нет.
function readAt(ms) {
  let combo = 0;
  let weight = 0;
  let objects = 0;
  for (const mark of judged.marks) {
    if (mark.ms > ms) break;
    combo = mark.combo;
    weight += mark.worth;
    objects += 1;
  }
  // В процентах, как и всё остальное про точность в этом окне: две единицы
  // измерения под одним словом — это ошибка, которая ждёт своего часа.
  return { combo, percent: objects ? (weight / (objects * 300)) * 100 : 100, objects };
}

function stamp(seconds) {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

function sayAt() {
  if (!scene) return;
  const said = readAt(head);
  byId("rp-hud").replaceChildren(
    el("span", null, `${said.combo}x`),
    el("span", null, `${round(said.percent, 2)}%`),
  );
  byId("rp-at").textContent = `${stamp((head - scene.from_ms) / 1000)} / ${stamp((scene.to_ms - scene.from_ms) / 1000)}`;
}

function drawAll() {
  drawView();
  drawSeek();
  sayAt();
  if (mode === "judge") drawTape();
  if (mode === "cut") drawReel();
}

// ── ход времени ────────────────────────────────────────────────────────

function stop() {
  if (!playing) return;
  cancelAnimationFrame(playing);
  playing = null;
  byId("rp-play").textContent = "▸";
}

byId("rp-play").addEventListener("click", () => {
  if (playing) {
    stop();
    return;
  }
  if (!scene) return;
  if (head >= scene.to_ms) head = scene.from_ms;
  byId("rp-play").textContent = "▪";
  let was = performance.now();
  const walk = (now) => {
    head += (now - was) * Number(byId("rp-speed").value);
    was = now;
    if (head >= scene.to_ms) {
      head = scene.to_ms;
      stop();
    } else {
      playing = requestAnimationFrame(walk);
    }
    drawAll();
  };
  playing = requestAnimationFrame(walk);
});

// ── полоса записи ──────────────────────────────────────────────────────

const seek = byId("rp-seek");

/// Насколько густо идут ноты. Грубая мера сложности и единственная, которую
/// можно взять из самой карты: чем больше объектов в окне, тем выше столбик.
/// Не претендует на звёзды — она отвечает на «где тут плотно», а не «насколько
/// это трудно».
let density = null;

function measureDensity() {
  const span = Math.max(1, scene.to_ms - scene.from_ms);
  const buckets = 320;
  const raw = new Float32Array(buckets);
  for (const piece of scene.objects) {
    const at = Math.floor(((piece.start_ms - scene.from_ms) / span) * buckets);
    if (at >= 0 && at < buckets) raw[at] += 1;
  }
  // Сглаживание по трём соседям: иначе полоса — частокол из единиц, по
  // которому ничего не видно.
  density = new Float32Array(buckets);
  let most = 0;
  for (let i = 0; i < buckets; i += 1) {
    const around = (raw[i - 1] || 0) + raw[i] + (raw[i + 1] || 0);
    density[i] = around / 3;
    most = Math.max(most, density[i]);
  }
  if (most > 0) for (let i = 0; i < buckets; i += 1) density[i] /= most;
}

/// Какой кусок записи показывает полоса. Как и у графика попаданий: на длинной
/// карте секунда — это два пикселя, и подводить головку к нужному месту мышью
/// становится гаданием.
let strip = null;

function drawSeek() {
  if (!scene || seek.clientWidth === 0) return;
  if (!density) measureDensity();
  const from = strip ? strip.from : scene.from_ms;
  const to = strip ? strip.to : scene.to_ms;
  const dpr = window.devicePixelRatio || 1;
  const w = seek.clientWidth;
  const h = seek.clientHeight;
  seek.width = Math.round(w * dpr);
  seek.height = Math.round(h * dpr);
  const c = seek.getContext("2d");
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, w, h);
  const span = Math.max(1, to - from);
  const whole = Math.max(1, scene.to_ms - scene.from_ms);
  const atX = (ms) => ((ms - from) / span) * w;

  if (mode === "cut") {
    c.fillStyle = "rgba(226,72,72,0.28)";
    for (const clip of clips) {
      c.fillRect(atX(clip.from), 0, Math.max(2, ((clip.to - clip.from) / span) * w, 2), h);
    }
  }

  c.beginPath();
  c.moveTo(0, h);
  for (let i = 0; i < density.length; i += 1) {
    const at = scene.from_ms + (i / (density.length - 1)) * whole;
    c.lineTo(atX(at), h - density[i] * (h - 6));
  }
  c.lineTo(w, h);
  c.closePath();
  const paint = c.createLinearGradient(0, 0, 0, h);
  paint.addColorStop(0, "rgba(226,72,72,0.55)");
  paint.addColorStop(1, "rgba(226,72,72,0.08)");
  c.fillStyle = paint;
  c.fill();

  // Промахи: там, где сложное место оказалось не только плотным.
  c.fillStyle = WORTH[0];
  for (const mark of judged.marks) {
    if (mark.worth !== 0 || mark.ms < from || mark.ms > to) continue;
    c.fillRect(atX(mark.ms) - 0.75, h - 7, 1.5, 7);
  }

  const at = atX(head);
  c.strokeStyle = "#fff";
  c.lineWidth = 2;
  c.beginPath();
  c.moveTo(at, 0);
  c.lineTo(at, h);
  c.stroke();
  c.fillStyle = "#fff";
  c.beginPath();
  c.arc(at, h - 2, 3.5, 0, Math.PI * 2);
  c.fill();
}

function seekTo(ms) {
  if (!scene) return;
  head = Math.min(scene.to_ms, Math.max(scene.from_ms, ms));
  drawAll();
}

function seekFromPointer(event) {
  const box = seek.getBoundingClientRect();
  const share = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  const from = strip ? strip.from : scene.from_ms;
  const to = strip ? strip.to : scene.to_ms;
  seekTo(from + share * (to - from));
}

/// Колесо приближает вокруг того места, куда смотрят. Двойной щелчок
/// возвращает всю запись.
seek.addEventListener("wheel", (event) => {
  if (!scene) return;
  event.preventDefault();
  const box = seek.getBoundingClientRect();
  const share = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  const from = strip ? strip.from : scene.from_ms;
  const to = strip ? strip.to : scene.to_ms;
  const span = to - from;
  const at = from + share * span;
  const whole = scene.to_ms - scene.from_ms;
  const next = Math.min(whole, Math.max(3000, span * (event.deltaY > 0 ? 1.3 : 0.77)));
  let start = Math.min(Math.max(scene.from_ms, at - share * next), scene.to_ms - next);
  strip = next >= whole ? null : { from: start, to: start + next };
  drawSeek();
}, { passive: false });

seek.addEventListener("dblclick", () => {
  strip = null;
  drawSeek();
});

seek.addEventListener("pointerdown", (event) => {
  if (!scene) return;
  seek.setPointerCapture(event.pointerId);
  seekFromPointer(event);
});
seek.addEventListener("pointermove", (event) => {
  if (scene && event.buttons) seekFromPointer(event);
});

/// Клавиатура — и в судействе, и в монтаже. Мышью ставят головку примерно,
/// клавишами — точно, а между «примерно» и «точно» здесь весь смысл.
document.addEventListener("keydown", (event) => {
  if (!scene || byId("view-replay").hidden || byId("rp-stage").hidden) return;
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName);
  if (typing) return;
  const step = event.shiftKey ? 10000 : 2000;
  if (event.key === "ArrowRight") seekTo(head + step);
  else if (event.key === "ArrowLeft") seekTo(head - step);
  else if (event.key === "Home") seekTo(scene.from_ms);
  else if (event.key === "End") seekTo(scene.to_ms);
  else if (event.key === " ") byId("rp-play").click();
  else return;
  event.preventDefault();
});

// ── судейство: график ошибок ───────────────────────────────────────────

/// Ошибка каждого клика во времени: по горизонтали — карта, по вертикали —
/// насколько раньше или позже.
/// Какой кусок реплея показан на графике. Всё целиком по умолчанию: на карте
/// в три минуты одна нота — это полпикселя, и приблизить её надо уметь.
let lens = null;

function drawTape() {
  if (!scene || tape.clientWidth === 0) return;
  const from = lens ? lens.from : scene.from_ms;
  const to = lens ? lens.to : scene.to_ms;
  const dpr = window.devicePixelRatio || 1;
  const w = tape.clientWidth;
  const h = tape.clientHeight;
  tape.width = Math.round(w * dpr);
  tape.height = Math.round(h * dpr);
  const c = tape.getContext("2d");
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, w, h);

  const span = Math.max(1, to - from);
  const middle = h / 2;
  const worst = Math.max(40, ...judged.marks.map((mark) => Math.abs(mark.error_ms || 0)));
  const scale = (middle - 14) / worst;

  c.strokeStyle = "rgba(255,255,255,0.14)";
  c.beginPath();
  c.moveTo(0, middle);
  c.lineTo(w, middle);
  c.stroke();

  for (const mark of judged.marks) {
    if (mark.ms < from || mark.ms > to) continue;
    const x = ((mark.ms - from) / span) * w;
    c.fillStyle = WORTH[mark.worth] || WORTH[0];
    if (mark.error_ms === null || mark.worth === 0) {
      c.fillRect(x - 0.75, h - 11, 1.5, 9);
      continue;
    }
    c.globalAlpha = 0.8;
    c.beginPath();
    c.arc(x, middle - mark.error_ms * scale, 1.5, 0, Math.PI * 2);
    c.fill();
    c.globalAlpha = 1;
  }

  const at = ((head - from) / span) * w;
  c.strokeStyle = "#fff";
  c.beginPath();
  c.moveTo(at, 0);
  c.lineTo(at, h);
  c.stroke();

  if (lens) {
    c.fillStyle = "rgba(255,255,255,0.45)";
    c.font = "10px ui-monospace, Menlo, monospace";
    c.fillText(`${stamp((from - scene.from_ms) / 1000)} — ${stamp((to - scene.from_ms) / 1000)}`, 6, 12);
  }
}

function scrub(event) {
  if (!scene) return;
  const box = tape.getBoundingClientRect();
  const share = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  const from = lens ? lens.from : scene.from_ms;
  const to = lens ? lens.to : scene.to_ms;
  head = from + share * (to - from);
  drawAll();
}

/// Колесо приближает вокруг того места, куда смотрят, — а не вокруг середины,
/// потому что смотрят обычно не в середину.
tape.addEventListener("wheel", (event) => {
  if (!scene) return;
  event.preventDefault();
  const box = tape.getBoundingClientRect();
  const share = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  const from = lens ? lens.from : scene.from_ms;
  const to = lens ? lens.to : scene.to_ms;
  const span = to - from;
  const at = from + share * span;
  const next = Math.min(scene.to_ms - scene.from_ms, Math.max(2000, span * (event.deltaY > 0 ? 1.25 : 0.8)));
  let start = at - share * next;
  start = Math.min(Math.max(scene.from_ms, start), scene.to_ms - next);
  lens = next >= scene.to_ms - scene.from_ms ? null : { from: start, to: start + next };
  drawTape();
}, { passive: false });

tape.addEventListener("dblclick", () => {
  lens = null;
  drawTape();
});

tape.addEventListener("pointerdown", (event) => {
  tape.setPointerCapture(event.pointerId);
  scrub(event);
});
tape.addEventListener("pointermove", (event) => {
  if (event.buttons) scrub(event);
});

// ── монтаж: лента ──────────────────────────────────────────────────────

const asShare = (ms) => (ms - scene.from_ms) / Math.max(1, scene.to_ms - scene.from_ms);

function drawReel() {
  if (!scene) {
    track.replaceChildren(el("p", "empty", "Сначала откройте реплей."));
    return;
  }
  byId("rp-ruler").replaceChildren(
    ...[0, 0.2, 0.4, 0.6, 0.8].map((share) => {
      const mark = el("span", null, stamp(((scene.to_ms - scene.from_ms) * share) / 1000));
      mark.style.left = `${share * 100}%`;
      return mark;
    }),
  );

  const parts = clips.map((clip, index) => {
    const box = el("div", `clip${index === picked ? " picked" : ""}`);
    box.style.left = `${asShare(clip.from) * 100}%`;
    box.style.width = `${Math.max(0.8, (asShare(clip.to) - asShare(clip.from)) * 100)}%`;
    box.append(el("b", null, `${Math.round((clip.to - clip.from) / 1000)}с`));
    box.append(el("span", "edge a"), el("span", "edge b"));
    box.addEventListener("pointerdown", (event) => grab(event, index));
    return box;
  });
  const playhead = el("div", "head");
  playhead.style.left = `${asShare(head) * 100}%`;
  track.replaceChildren(...parts, playhead);

  const total = clips.reduce((sum, clip) => sum + (clip.to - clip.from), 0);
  byId("rp-total").textContent = clips.length
    ? `${clips.length} ${plural(clips.length, "кусок", "куска", "кусков")} · ${round(total / 1000, 1)} с`
    : "лента пуста";
}

/// Тащить целиком за середину, за края — растягивать. Границы куска не
/// пускаются за края реплея и не схлопываются в точку: лента, которая может
/// собраться в невозможный набор, соберётся в него в первый же день.
function grab(event, index) {
  event.preventDefault();
  picked = index;
  const edge = event.target.classList.contains("edge")
    ? (event.target.classList.contains("a") ? "a" : "b")
    : "move";
  const width = track.getBoundingClientRect().width;
  const span = scene.to_ms - scene.from_ms;
  const startX = event.clientX;
  const was = { ...clips[index] };
  const least = 500;

  const move = (now) => {
    const by = ((now.clientX - startX) / width) * span;
    const clip = clips[index];
    if (edge === "move") {
      const length = was.to - was.from;
      clip.from = Math.min(Math.max(scene.from_ms, was.from + by), scene.to_ms - length);
      clip.to = clip.from + length;
    } else if (edge === "a") {
      clip.from = Math.min(Math.max(scene.from_ms, was.from + by), was.to - least);
    } else {
      clip.to = Math.max(Math.min(scene.to_ms, was.to + by), was.from + least);
    }
    drawReel();
  };
  const drop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", drop);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", drop);
  drawReel();
}

byId("rp-add").addEventListener("click", () => {
  if (!scene) return;
  const from = Math.min(head, scene.to_ms - 2000);
  clips.push({ from, to: Math.min(scene.to_ms, from + 8000) });
  clips.sort((a, b) => a.from - b.from);
  picked = clips.length - 1;
  drawReel();
});

byId("rp-clear").addEventListener("click", () => {
  clips = [];
  picked = -1;
  drawReel();
});

document.addEventListener("keydown", (event) => {
  if ((event.key === "Delete" || event.key === "Backspace") && picked >= 0 && document.activeElement === document.body) {
    clips.splice(picked, 1);
    picked = -1;
    drawReel();
  }
});

byId("rp-build").addEventListener("click", async () => {
  if (!chosen || drawing) return;
  const said = byId("rp-built");
  if (!clips.length) {
    said.className = "verdict bad";
    said.textContent = "на ленте нет ни одного куска — нажмите «+ Кусок отсюда»";
    return;
  }
  drawing = true;
  watching = { bar: "rp-bar", said: "rp-built", share: null };
  startJob("replay", "Монтаж");
  said.className = "verdict";
  said.textContent = "собираю…";
  byId("rp-progress").hidden = false;
  byId("rp-bar").style.width = "0%";
  const out = chosen.replace(/\.osr$/i, "") + "-монтаж.mp4";
  const settings = options();
  try {
    const done = await invoke("build_reel", {
      replay: chosen,
      out,
      spans: clips.map((clip) => ({ from_ms: clip.from, to_ms: clip.to })),
      skin: settings.skin,
      width: settings.width,
      height: settings.height,
      fps: settings.fps,
      mute: settings.mute,
    });
    said.textContent = `готово — ${done.path}`;
    endJob(true);
  } catch (why) {
    said.className = "verdict bad";
    said.textContent = `не вышло: ${why}`;
    endJob(false);
  } finally {
    drawing = false;
    byId("rp-progress").hidden = true;
  }
});

function showReplay() {
  if (scene && mode) drawAll();
}

// ── переключение вкладок ───────────────────────────────────────────────

const views = {
  render: showRender,
  replay: showReplay,
  library: showLibrary,
  settings: showSettings,
};

let open_tab = null;

function show(which) {
  // Нажатие по разделу, который и так открыт, — это не запрос перерисовать
  // его: список, который моргает от каждого промаха мимо соседней вкладки,
  // раздражает ровно настолько, насколько это дёшево не делать.
  if (which === open_tab) return;
  open_tab = which;
  syncJobMini();
  for (const name of Object.keys(views)) {
    byId(`tab-${name}`).setAttribute("aria-selected", String(name === which));
    byId(`view-${name}`).hidden = name !== which;
  }
  // Лепёшка едет медленно и с оттяжкой, а под курсором ходит мгновенно —
  // это одно и то же движение с двумя разными характерами.
  glider.classList.add("moving");
  setTimeout(() => glider.classList.remove("moving"), 470);
  placeGlider();
  const view = byId(`view-${which}`);
  for (const child of view.children) {
    child.style.animation = "none";
    void child.offsetWidth;
    child.style.animation = "";
  }
  views[which]();
}

for (const name of Object.keys(views)) {
  byId(`tab-${name}`).addEventListener("click", () => show(name));
}

// Холст знает свою ширину только когда его видно, а лента считает проценты от
// живой ширины дорожки.
window.addEventListener("resize", () => {
  if (!byId("view-replay").hidden) drawAll();
});

// ── первый запуск ──────────────────────────────────────────────────────

byId("w-save").addEventListener("click", async () => {
  if (!byId("w-server").value.trim() || !byId("w-token").value.trim()) {
    const said = byId("w-said");
    said.className = "verdict bad";
    said.textContent = "адрес и токен — те два, без которых ничего не поедет";
    return;
  }
  if (await saveSettings("w", "w-said")) {
    byId("wizard").hidden = true;
    show("render");
  }
});

(async function open() {
  dockSide(remembered("dock", "top"), false, false);
  motion(remembered("motion", "on"), false);
  sky(remembered("sky", "live"), false);
  splashWhen(remembered("splash", "both"), false);
  skinned = remembered("skinned", "0") === "1";
  idleAfter(remembered("idle", "5"), false);
  reportWhen(remembered("report", "ask"), false);

  // Экран чёрный с первого кадра — до того, как что-либо успело измерить себя
  // или разложиться. Сцена запускается только когда всё это уже случилось:
  // раньше она стартовала тут же и её первые секунды съедала как раз та
  // работа, которую окно ещё не закончило делать.
  const opening = remembered("splash", "both") !== "never";
  if (opening) showBlackCover();
  requestAnimationFrame(() => dock.classList.remove("landing"));

  let first = false;
  try {
    first = await invoke("first_run");
  } catch {
    // Нет моста — покажем обычное окно, оно скажет об этом само.
  }
  await loadSettings();
  if (first) {
    for (const field of FIELDS) {
      const box = byId(`w-${field}`);
      if (box) box.value = known[field] || "";
    }
    // Мастеру нужно внимание сразу, а не через полторы секунды кольца —
    // первый запуск и так самый долгий разговор, который у человека будет с
    // этим окном.
    if (opening) {
      locked = false;
      asleep = false;
      splash.classList.remove("going");
      splash.hidden = true;
    }
    byId("wizard").hidden = false;
    armIdle();
    return;
  }
  show("render");

  // Мерить надо по готовым шрифтам: до них подписи другой ширины, и панель
  // разъезжается на первом же движении курсора.
  dressAll();
  measure();
  if (document.fonts && document.fonts.ready) await document.fonts.ready;
  loadPics().catch(() => {});

  // Ещё два кадра тишины: браузеру нужно успеть отрисовать всё, что только
  // что легло на макет, прежде чем часы сцены начнут отсчёт — иначе первые
  // её кадры съедает та же раскладка.
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (opening) playOpening();
  armIdle();

  // Не в первую секунду: спрашивать у origin, пока ещё идёт заставка, значит
  // тратить её на ожидание сети.
  setTimeout(() => lookForUpdate(false), 3500);
})();
