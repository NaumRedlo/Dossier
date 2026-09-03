// In its own file, not in a `<script>` tag: the window's content policy is
// `default-src 'self'`, which blocks an inline script — a page that worked
// perfectly in a browser and showed nothing at all in the application.

/// Куда идёт жалоба на баг. Почта и репозиторий известны; ник в Telegram знаете
/// только вы — впишите его сюда без «собаки», и кнопка появится сама.
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

dock.addEventListener("pointermove", (event) => magnify(vertical() ? event.clientY : event.clientX));
dock.addEventListener("pointerleave", relax);
window.addEventListener("resize", measure);

// ── готовность ─────────────────────────────────────────────────────────

async function showReady() {
  const box = byId("ready-rows");
  const verdict = byId("ready-verdict");
  let rows;
  try {
    rows = await invoke("ready");
  } catch (why) {
    verdict.textContent = `не удалось спросить: ${why.message}`;
    verdict.className = "verdict bad";
    return;
  }
  box.replaceChildren(
    ...rows.map((row) => line({ mark: sign(row.ok), name: row.name, said: row.said, fix: row.ok === false ? row.fix : "" })),
  );
  const stopped = rows.filter((row) => row.ok === false).length;
  verdict.textContent = stopped
    ? `${stopped} ${plural(stopped, "пункт", "пункта", "пунктов")} надо поправить`
    : "готово — можно брать работу";
  verdict.className = stopped ? "verdict bad" : "verdict";
}

// ── машина ─────────────────────────────────────────────────────────────

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
      fix: hardware.encoders.length ? "" : "движок и так кодирует x264 — это про будущее",
    },
    {
      mark: sign(told.capacity.take),
      name: "политика",
      said: told.capacity.reason,
      fix: told.capacity.take ? "" : "работа не берётся, пока это так",
    },
  ];
  if (told.speed) {
    rows.push({ mark: ["ok", "·"], name: "на один поток", said: `${round(told.speed.per_thread)} кадров в секунду` });
  }
  box.replaceChildren(...rows.map(line));
}

// ── рендер своего реплея ───────────────────────────────────────────────

let drawing = false;

function playRow(play, onDraw) {
  const row = el("div", "play");
  const left = el("div");
  left.append(el("b", null, play.player), el("div", "about", `${play.mods || "NM"} · ${round(play.score)} очков · комбо ${play.combo}`));
  row.append(left);
  row.append(play.have_map ? el("span", "about", "") : el("span", "nomap", "карты нет"));
  const go = el("button", "act small", "Нарисовать");
  go.disabled = !play.have_map;
  go.addEventListener("click", () => onDraw(play, go));
  row.append(go);
  return row;
}

async function showRender() {
  const list = byId("r-list");
  const said = byId("r-said");
  const [plays, skins] = await Promise.all([invoke("my_replays", {}), invoke("skins")]);

  const picker = byId("r-skin");
  picker.replaceChildren(el("option", null, "как рисует бот"));
  picker.firstChild.value = "";
  for (const name of skins) {
    const option = el("option", null, name);
    option.value = name;
    picker.append(option);
  }
  byId("r-count").textContent = plays.length
    ? `${plays.length} ${plural(plays.length, "реплей", "реплея", "реплеев")}, новые сверху`
    : "";

  if (!plays.length) {
    list.replaceChildren(line({ mark: ["huh", "?"], name: "пусто", said: "укажите папку реплеев в настройках" }));
    return;
  }
  list.replaceChildren(...plays.map((play) => playRow(play, draw)));

  async function draw(play, button) {
    if (drawing) return;
    drawing = true;
    button.disabled = true;
    said.className = "verdict";
    said.textContent = "рисую…";
    const bar = byId("r-progress");
    bar.hidden = false;
    byId("r-bar").style.width = "0%";
    const out = play.path.replace(/\.osr$/i, "") + ".mp4";
    try {
      const done = await invoke("draw", { replay: play.path, out, skin: picker.value || null });
      said.textContent = `готово — ${done.path}`;
      for (const note of done.said) said.textContent += `\n${note}`;
    } catch (why) {
      said.className = "verdict bad";
      said.textContent = `не вышло: ${why}`;
    } finally {
      drawing = false;
      button.disabled = false;
      bar.hidden = true;
    }
  }
}

// The engine's own events, forwarded from the render — see the `draw` command.
if (window.__TAURI__ && window.__TAURI__.event) {
  window.__TAURI__.event.listen("drawing", ({ payload }) => {
    if (payload.event !== "progress" || !payload.of) return;
    const share = Math.min(100, (payload.frames / payload.of) * 100);
    byId("r-bar").style.width = `${share}%`;
    byId("r-said").textContent =
      `рисую: ${round(share, 1)}% · ${round(payload.per_second)} кадров в секунду · осталось ${round(payload.left_seconds)} с`;
  });
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

// ── настройки ──────────────────────────────────────────────────────────

const FIELDS = ["server", "token", "name", "songs", "skins", "replays"];

async function showSettings() {
  const said = await invoke("settings_read");
  for (const field of FIELDS) byId(`s-${field}`).value = said[field] || "";
  nameChip(said);
  const shelves = await invoke("shelves");
  byId("s-shelves").replaceChildren(
    ...[["карты", shelves.songs], ["скины", shelves.skins], ["реплеи", shelves.replays]].map(([name, shelf]) =>
      line({
        mark: shelf.exists ? ["ok", "+"] : ["huh", "?"],
        name,
        said: shelf.exists ? `${shelf.note} · ${shelf.path}` : shelf.note,
        fix: shelf.exists ? "" : shelf.path,
      }),
    ),
  );
}

function nameChip(said) {
  // Только имя: подпись стоит в углу рядом с панелью, и в узком окне длинная
  // строка с адресом наезжает на вкладки. Адрес — во всплывающей подсказке и в
  // настройках, где ему и место.
  const chip = byId("who");
  chip.textContent = said.name || "Dossier";
  chip.title = `${said.name} · ${said.server || "адрес не задан"}`;
}

async function saveSettings(prefix, saidId) {
  const said = {};
  for (const field of FIELDS) said[field] = byId(`${prefix}-${field}`).value.trim();
  const note = byId(saidId);
  try {
    await invoke("settings_write", { said });
    note.className = "verdict";
    note.textContent = "сохранено";
    return true;
  } catch (why) {
    note.className = "verdict bad";
    note.textContent = `не сохранилось: ${why}`;
    return false;
  }
}

// ── переключатели вида ─────────────────────────────────────────────────

function pressed(group, chosen) {
  for (const button of group.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(button === chosen));
  }
}

function dockSide(side, save = true) {
  document.body.classList.toggle("dock-left", side === "left");
  document.body.classList.toggle("dock-top", side !== "left");
  pressed(byId("seg-dock"), byId("seg-dock").querySelector(`[data-side="${side}"]`));
  if (save) remember("dock", side);
  // Панель поехала на другую сторону — все замеры теперь не про неё.
  requestAnimationFrame(measure);
}

function motion(how, save = true) {
  document.body.classList.toggle("still", how === "off");
  pressed(byId("seg-motion"), byId("seg-motion").querySelector(`[data-motion="${how}"]`));
  if (save) remember("motion", how);
  if (how === "off") relax();
}

for (const button of byId("seg-dock").querySelectorAll("button")) {
  button.addEventListener("click", () => dockSide(button.dataset.side));
}
for (const button of byId("seg-motion").querySelectorAll("button")) {
  button.addEventListener("click", () => motion(button.dataset.motion));
}

// ── жук ────────────────────────────────────────────────────────────────

const bugbox = byId("bugbox");
const bug = byId("bug");
const bugpane = byId("bugpane");
let bugTimer = null;
let pinned = false;

/// Открыть или закрыть. Курсор в поле отдаётся только тому, кто нажал: панель
/// раскрывается и от наведения, а забирать клавиатуру у того, кто просто провёл
/// мышью мимо угла, — худшее, что она может сделать.
function openBug(yes, byHand = false) {
  clearTimeout(bugTimer);
  bugpane.hidden = !yes;
  bug.setAttribute("aria-expanded", String(yes));
  if (yes && byHand) byId("bug-text").focus({ preventScroll: true });
}

bugbox.addEventListener("pointerenter", () => {
  clearTimeout(bugTimer);
  bugTimer = setTimeout(() => openBug(true), 140);
});
bugbox.addEventListener("pointerleave", () => {
  clearTimeout(bugTimer);
  if (pinned || bugbox.contains(document.activeElement)) return;
  bugTimer = setTimeout(() => openBug(false), 280);
});
bug.addEventListener("click", () => {
  pinned = bugpane.hidden ? true : !pinned;
  openBug(pinned || bugpane.hidden, true);
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !bugpane.hidden) {
    pinned = false;
    openBug(false);
  }
});
// Прикреплённая панель закрывается щелчком мимо: иначе она остаётся висеть над
// вкладкой, на которую в этот момент нажали.
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

// ── переключение вкладок ───────────────────────────────────────────────

const views = {
  ready: showReady,
  machine: showMachine,
  render: showRender,
  farm: showFarm,
  settings: showSettings,
};

function show(which) {
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
byId("again").addEventListener("click", showReady);
byId("measure").addEventListener("click", showMachine);
byId("f-again").addEventListener("click", showFarm);
byId("s-save").addEventListener("click", () => saveSettings("s", "s-said").then(showSettings));

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
    show("ready");
  }
});

(async function open() {
  dockSide(remembered("dock", "top"), false);
  motion(remembered("motion", "on"), false);

  let first = false;
  try {
    first = await invoke("first_run");
  } catch {
    // Нет моста — покажем обычное окно, оно скажет об этом само.
  }
  if (first) {
    const said = await invoke("settings_read").catch(() => ({}));
    for (const field of FIELDS) {
      const box = byId(`w-${field}`);
      if (box) box.value = said[field] || "";
    }
    byId("wizard").hidden = false;
    return;
  }
  show("ready");
  invoke("settings_read").then(nameChip).catch(() => {});

  // Мерить надо по готовым шрифтам: до них подписи другой ширины, и панель
  // разъезжается на первом же движении курсора.
  measure();
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(measure);
})();
