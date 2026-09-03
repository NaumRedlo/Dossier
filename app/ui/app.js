// In its own file, not in a `<script>` tag: the window's content policy is
// `default-src 'self'`, which blocks an inline script — a page that worked
// perfectly in a browser and showed nothing at all in the application.

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

/// One line of a card: a sign, a name, what was said, and a remedy when it is
/// the sort of thing somebody can do something about.
function line({ mark, name, said, fix }) {
  const row = el("div", "row");
  row.append(el("span", `mark-sign ${mark[0]}`, mark[1]), el("span", "name", name), el("span", "said", said));
  if (fix) row.append(el("span", "fix", fix));
  return row;
}

const sign = (ok) => (ok === null || ok === undefined ? ["huh", "?"] : ok ? ["ok", "+"] : ["no", "!"]);

// ── готовность ─────────────────────────────────────────────────────────

async function showReady() {
  const box = document.getElementById("ready-rows");
  const verdict = document.getElementById("ready-verdict");
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
  const head = document.getElementById("machine-headline");
  const box = document.getElementById("machine-rows");
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
  const list = document.getElementById("r-list");
  const said = document.getElementById("r-said");
  const [plays, skins] = await Promise.all([invoke("my_replays", {}), invoke("skins")]);

  const picker = document.getElementById("r-skin");
  picker.replaceChildren(el("option", null, "как рисует бот"));
  picker.firstChild.value = "";
  for (const name of skins) {
    const option = el("option", null, name);
    option.value = name;
    picker.append(option);
  }
  document.getElementById("r-count").textContent = plays.length
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
    const bar = document.getElementById("r-progress");
    bar.hidden = false;
    document.getElementById("r-bar").style.width = "0%";
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
    document.getElementById("r-bar").style.width = `${share}%`;
    document.getElementById("r-said").textContent =
      `рисую: ${round(share, 1)}% · ${round(payload.per_second)} кадров в секунду · осталось ${round(payload.left_seconds)} с`;
  });
}

// ── ферма ──────────────────────────────────────────────────────────────

async function showFarm() {
  const box = document.getElementById("f-rows");
  const said = document.getElementById("f-said");
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
  document.getElementById("f-waiting").textContent = farm.waiting
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
  for (const field of FIELDS) document.getElementById(`s-${field}`).value = said[field] || "";
  document.getElementById("who").textContent = `${said.name} · ${said.server || "адрес не задан"}`;
  const shelves = await invoke("shelves");
  document.getElementById("s-shelves").replaceChildren(
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

async function saveSettings(prefix, saidId) {
  const said = {};
  for (const field of FIELDS) said[field] = document.getElementById(`${prefix}-${field}`).value.trim();
  const note = document.getElementById(saidId);
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

// ── переключение ───────────────────────────────────────────────────────

const views = {
  ready: showReady,
  machine: showMachine,
  render: showRender,
  farm: showFarm,
  settings: showSettings,
};

function show(which) {
  for (const name of Object.keys(views)) {
    document.getElementById(`tab-${name}`).setAttribute("aria-selected", String(name === which));
    document.getElementById(`view-${name}`).hidden = name !== which;
  }
  views[which]();
}

for (const name of Object.keys(views)) {
  document.getElementById(`tab-${name}`).addEventListener("click", () => show(name));
}
document.getElementById("again").addEventListener("click", showReady);
document.getElementById("measure").addEventListener("click", showMachine);
document.getElementById("f-again").addEventListener("click", showFarm);
document.getElementById("s-save").addEventListener("click", () => saveSettings("s", "s-said").then(showSettings));

// ── первый запуск ──────────────────────────────────────────────────────

document.getElementById("w-save").addEventListener("click", async () => {
  if (!document.getElementById("w-server").value.trim() || !document.getElementById("w-token").value.trim()) {
    const note = document.getElementById("w-said");
    note.className = "verdict bad";
    note.textContent = "адрес и токен — те два, без которых ничего не поедет";
    return;
  }
  if (await saveSettings("w", "w-said")) {
    document.getElementById("wizard").hidden = true;
    show("ready");
  }
});

(async function open() {
  let first = false;
  try {
    first = await invoke("first_run");
  } catch {
    // Нет моста — покажем обычное окно, оно скажет об этом само.
  }
  if (first) {
    const said = await invoke("settings_read").catch(() => ({}));
    for (const field of FIELDS) {
      const box = document.getElementById(`w-${field}`);
      if (box) box.value = said[field] || "";
    }
    document.getElementById("wizard").hidden = false;
    return;
  }
  show("ready");
  invoke("settings_read")
    .then((said) => {
      document.getElementById("who").textContent = `${said.name} · ${said.server || "адрес не задан"}`;
    })
    .catch(() => {});
})();
