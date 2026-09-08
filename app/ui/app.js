const DEFAULT_SKIN = "Dossier Default";

const CONTACT = {
  mail: "naumredlo@icloud.com",
  telegram: "@NaumRedlo",
  repo: "https://github.com/NaumRedlo/Dossier",
};

function invoke(name, args) {
  const bridge = window.__TAURI__;
  if (!bridge) return Promise.reject(new Error("Нет моста к приложению"));
  return bridge.core.invoke(name, args);
}

function spell(seconds) {
  const whole = Math.max(0, Math.round(seconds));
  if (whole < 60) return `${whole} с`;
  const minutes = Math.floor(whole / 60);
  if (minutes < 60) {
    const left = whole % 60;
    return left ? `${minutes} м ${left} с` : `${minutes} м`;
  }
  const hours = Math.floor(minutes / 60);
  const left = minutes % 60;
  return left ? `${hours} ч ${left} м` : `${hours} ч`;
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

function line({ mark, name, said, fix }) {
  const row = el("div", "row");
  row.append(el("span", `mark-sign ${mark[0]}`, mark[1]), el("span", "name", name), el("span", "said", said));
  if (fix) row.append(el("span", "fix", fix));
  return row;
}

const sign = (ok) => (ok === null || ok === undefined ? ["huh", "?"] : ok ? ["ok", "+"] : ["no", "!"]);

const OPTIONS_WIDEST = 560;

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

  const list = el("div", "options");
  list.hidden = true;
  document.body.append(list);

  function place() {
    const rect = button.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 16;
    const above = rect.top - 16;
    const flip = below < 120 && above > below;
    list.style.width = "auto";
    list.style.minWidth = `${rect.width}px`;
    list.style.maxWidth = `${Math.min(window.innerWidth - 16, OPTIONS_WIDEST, Math.max(rect.width, 280))}px`;
    list.style.left = "0px";
    const wide = list.getBoundingClientRect().width;
    list.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - wide - 8))}px`;
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
  }
}

const still = () => document.body.classList.contains("still");

const dock = byId("dock");
const items = [...document.querySelectorAll(".item")];
const glider = byId("glider");

const GROW = 0.42;
const REACH = 78;

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
  base = items.map((item) => {
    const box = item.getBoundingClientRect();
    const glyph = item.querySelector(".glyph").getBoundingClientRect();
    return {
      start: box.left,
      size: box.width,
      centre: box.left + box.width / 2,
      glyph: glyph.width,
    };
  });
  placeGlider();
}

function magnify(pos) {
  if (!base || still()) return;
  const scale = base.map((box) => 1 + GROW * Math.exp(-(((pos - box.centre) / REACH) ** 2)));
  const span = base.map((box, i) => box.size + box.glyph * (scale[i] - 1));
  const grew = span.reduce((a, b) => a + b, 0) - base.reduce((a, box) => a + box.size, 0);

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
  item.style.setProperty("--tx", `${shift.toFixed(2)}px`);
  item.style.setProperty("--ty", "0px");
  const glyph = item.querySelector(".glyph");
  glyph.style.setProperty("--s", scale.toFixed(3));
  glyph.style.setProperty("--ly", `${(((scale - 1) / GROW) * 1.6).toFixed(2)}px`);
  glyph.style.setProperty("--lx", "0px");
}

function placeGlider(shift = 0) {
  const open = items.find((item) => item.getAttribute("aria-selected") === "true");
  if (!open) return;
  const x = open.offsetLeft + shift;
  glider.style.transform = `translate(${x.toFixed(2)}px, ${open.offsetTop.toFixed(2)}px)`;
  glider.style.width = `${open.offsetWidth}px`;
  glider.style.height = `${open.offsetHeight}px`;
}

let waiting = null;
dock.addEventListener("pointermove", (event) => {
  const pos = event.clientX;
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

const items_box = byId("items");
let easeOff = null;

function easeFor(ms) {
  clearTimeout(easeOff);
  items_box.classList.add("easing");
  easeOff = setTimeout(() => items_box.classList.remove("easing"), ms);
}

dock.addEventListener("pointerenter", () => easeFor(200));
dock.addEventListener("pointerleave", () => {
  easeFor(260);
  relax();
});
window.addEventListener("resize", measure);

function pressed(group, chosen) {
  for (const button of group.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(button === chosen));
  }
}

const SKIES = ["rich", "live", "calm", "still"];

function sky(how, save = true) {
  const wanted = SKIES.includes(how) ? how : "live";
  for (const one of SKIES) document.body.classList.toggle(`sky-${one}`, one === wanted);
  pressed(byId("seg-sky"), byId("seg-sky").querySelector(`[data-sky="${wanted}"]`));
  if (save) remember("sky", wanted);
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
  const moving = still() && how !== "off";
  document.body.classList.toggle("still", how === "off");
  pressed(byId("seg-motion"), byId("seg-motion").querySelector(`[data-motion="${how}"]`));
  if (save) remember("motion", how);
  if (how === "off") relax();
  restartLoops();
  runPreviews();
}

const looping = new WeakMap();

function loopIcon(one, frames, ms, easing, origin) {
  if (!one) return;
  const had = looping.get(one);
  if (had) had.cancel();
  looping.delete(one);
  if (still()) return;
  one.style.transformBox = "fill-box";
  one.style.transformOrigin = origin || "center";
  looping.set(one, one.animate(frames, { duration: ms, iterations: Infinity, easing }));
}

function stopLoop(what) {
  const one = document.querySelector(what);
  if (!one) return;
  const had = looping.get(one);
  if (had) had.cancel();
  looping.delete(one);
  one.style.removeProperty("opacity");
  one.style.removeProperty("transform");
}

function restartLoops() {
  const at = (what) => document.querySelector(what);
  loopIcon(
    at(".ic-scan .outer"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    2400,
    "cubic-bezier(0.5, 0, 0.5, 1)",
  );
  loopIcon(
    at(".ic-scan .inner"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(-360deg)" }],
    1700,
    "cubic-bezier(0.5, 0, 0.5, 1)",
  );
  loopIcon(
    at(".ic-scan .heart"),
    [
      { transform: "scale(0.85)", opacity: 0.75 },
      { transform: "scale(1.15)", opacity: 1, offset: 0.5 },
      { transform: "scale(0.85)", opacity: 0.75 },
    ],
    1700,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-nomap .arc"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    2600,
    "cubic-bezier(0.5, 0, 0.5, 1)",
  );
  loopIcon(
    at(".ic-nomap .one"),
    [
      { transform: "scale(0.88)", opacity: 0.7 },
      { transform: "scale(1.06)", opacity: 1, offset: 0.42 },
      { transform: "scale(0.88)", opacity: 0.7 },
    ],
    1800,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-nomap .two"),
    [
      { transform: "scale(1.06)", opacity: 1 },
      { transform: "scale(0.88)", opacity: 0.7, offset: 0.42 },
      { transform: "scale(1.06)", opacity: 1 },
    ],
    1800,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-reading .arc"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    1900,
    "cubic-bezier(0.5, 0, 0.5, 1)",
  );
  loopIcon(
    at(".ic-reading .pen"),
    [
      { strokeDashoffset: "86", opacity: 0.25 },
      { opacity: 1, offset: 0.18 },
      { strokeDashoffset: "0", opacity: 1, offset: 0.72 },
      { strokeDashoffset: "0", opacity: 0.25 },
    ],
    2200,
    "cubic-bezier(0.35, 0.6, 0.3, 1)",
  );
  loopIcon(
    at(".ic-reading .spark"),
    [
      { transform: "scale(0.2)", opacity: 0 },
      { transform: "scale(0.2)", opacity: 0, offset: 0.6 },
      { transform: "scale(1)", opacity: 0.9, offset: 0.78 },
      { transform: "scale(1.5)", opacity: 0 },
    ],
    2200,
    "ease-out",
  );
  loopIcon(
    at(".ic-done .ring"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    6000,
    "linear",
  );
  loopIcon(
    at(".ic-done .tick"),
    [{ opacity: 0.7 }, { opacity: 1, offset: 0.5 }, { opacity: 0.7 }],
    2400,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-part .ring"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    5200,
    "linear",
  );
  loopIcon(
    at(".ic-part .tick"),
    [
      { opacity: 0.4 },
      { opacity: 0.85, offset: 0.5 },
      { opacity: 0.4 },
    ],
    2200,
    "ease-in-out",
  );
  for (const what of [".pull .stem", ".pull .head"]) {
    loopIcon(
      at(what),
      [
        { transform: "translateY(0px)", opacity: 0.55 },
        { transform: "translateY(1.4px)", opacity: 1, offset: 0.5 },
        { transform: "translateY(0px)", opacity: 0.55 },
      ],
      2600,
      "ease-in-out",
    );
  }
  loopIcon(
    at(".ic-drop .arc"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    3200,
    "cubic-bezier(0.5, 0, 0.5, 1)",
  );
  loopIcon(
    at(".ic-drop .lid"),
    [
      { transform: "rotate(0deg) translateY(0)" },
      { transform: "rotate(-14deg) translateY(-3px)", offset: 0.32 },
      { transform: "rotate(-14deg) translateY(-3px)", offset: 0.6 },
      { transform: "rotate(0deg) translateY(0)" },
    ],
    2400,
    "ease-in-out",
    "right bottom",
  );
  loopIcon(
    at(".ic-drop .rib"),
    [{ opacity: 0.25 }, { opacity: 0.7, offset: 0.5 }, { opacity: 0.25 }],
    2400,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-fold .arc"),
    [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
    1100,
    "linear",
  );
  loopIcon(
    at(".ic-fold .down"),
    [
      { transform: "translateY(-2px)", opacity: 0.5 },
      { transform: "translateY(1px)", opacity: 1, offset: 0.5 },
      { transform: "translateY(-2px)", opacity: 0.5 },
    ],
    1400,
    "ease-in-out",
  );
  loopIcon(
    at(".ic-judge .ring"),
    [
      { transform: "scale(2.1)", opacity: 0 },
      { opacity: 0.85, offset: 0.18 },
      { transform: "scale(1)", opacity: 0.85, offset: 0.72 },
      { transform: "scale(1)", opacity: 0, offset: 0.86 },
      { transform: "scale(1)", opacity: 0 },
    ],
    2400,
    "cubic-bezier(0.2, 0.6, 0.35, 1)",
  );
  loopIcon(
    at(".ic-cut .head"),
    [
      { transform: "translateX(0)", opacity: 0 },
      { opacity: 0.9, offset: 0.08 },
      { opacity: 0.9, offset: 0.88 },
      { transform: "translateX(12.6px)", opacity: 0 },
    ],
    4200,
    "linear",
  );
  loopIcon(
    at(".ic-cut .clip.b"),
    [{ transform: "scaleX(1)" }, { transform: "scaleX(0.62)", offset: 0.46 }, { transform: "scaleX(1)" }],
    4200,
    "ease-in-out",
    "left center",
  );
}

for (const mode of document.querySelectorAll(".mode")) {
  mode.addEventListener("pointerenter", () => {
    for (const one of mode.querySelectorAll("*")) {
      const had = looping.get(one);
      if (had) had.updatePlaybackRate(1.6);
    }
  });
  mode.addEventListener("pointerleave", () => {
    for (const one of mode.querySelectorAll("*")) {
      const had = looping.get(one);
      if (had) had.updatePlaybackRate(1);
    }
  });
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

const idleMs = () => Number(remembered("idle", "5")) * 60 * 1000;

const splash = byId("splash");
const splashView = byId("splash-view");
let asleep = false;
let idleTimer = null;
let lastStir = 0;
let scene_run = null;
let idleRun = 0;

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

const RING_IN = 0.046 / 0.335;

const RISE_MS = 620;

const STRIKE_MS = 900;
const AFTER_HIT_MS = 640;

let locked = false;

let uncovered = Promise.resolve();
let uncover = () => {};

const SPLASH_DPR = 1.5;

function sizeSplash(c) {
  const dpr = Math.min(window.devicePixelRatio || 1, SPLASH_DPR);
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

function showBlackCover() {
  locked = true;
  asleep = true;
  uncovered = new Promise((resolve) => {
    uncover = resolve;
  });
  splash.classList.remove("going");
  splash.hidden = false;

  document.body.classList.add("covered");

  sizeSplash(splashView.getContext("2d"));
}

function letterBox(w, h) {
  const size = Math.min(w, h) * 0.17;
  const side = size * (markIsLetter ? 2.6 : 1.5);
  return { mid: [w / 2, h / 2 - 8], size, side };
}

function drawRing(c, mid, size, alpha) {
  c.globalAlpha = alpha * 0.85;
  c.strokeStyle = "#e24848";
  c.lineWidth = size * RING_IN;
  c.beginPath();
  c.arc(mid[0], mid[1], size, 0, Math.PI * 2);
  c.stroke();
  c.globalAlpha = 1;
}

let bloomFor = null;

function bloom(c, mid, size, reach) {
  const key = `${mid[0]}:${mid[1]}:${Math.round(size)}:${Math.round(reach)}`;
  if (!bloomFor || bloomFor.key !== key) {
    const paint = c.createRadialGradient(mid[0], mid[1], 0, mid[0], mid[1], reach);
    paint.addColorStop(0, "rgba(226, 72, 72, 0.9)");
    paint.addColorStop(1, "rgba(226, 72, 72, 0)");
    bloomFor = { key, paint };
  }
  return bloomFor.paint;
}

function playOpening() {
  const c = splashView.getContext("2d");
  const started = performance.now();
  let struck = false;
  locked = true;

  const frame = (now) => {
    const { w, h } = sizeSplash(c);
    const t = now - started;
    const { mid, size, side } = letterBox(w, h);

    const shown = Math.min(1, t / RISE_MS);
    const eased = 1 - (1 - shown) ** 3;
    drawRing(c, mid, size, eased);

    if (markImage.complete && markImage.naturalWidth) {
      const swell = struck ? 1 + 0.06 * Math.max(0, 1 - (t - STRIKE_MS) / 260) : 1;
      const grown = side * swell;
      c.globalAlpha = eased;
      c.drawImage(markImage, mid[0] - grown / 2, mid[1] - grown / 2, grown, grown);
      c.globalAlpha = 1;
    }

    if (struck) {
      const lit = (t - STRIKE_MS) / 560;
      if (lit < 1) {
        const reach = size * 3.2;
        c.save();
        c.globalCompositeOperation = "lighter";
        c.globalAlpha = (1 - lit) * 0.55 * Math.min(1, lit * 6);
        c.fillStyle = bloom(c, mid, size, reach);
        c.fillRect(mid[0] - reach, mid[1] - reach, reach * 2, reach * 2);
        c.restore();

        c.globalAlpha = (1 - lit) * 0.7;
        c.strokeStyle = "#fff";
        c.lineWidth = 2;
        c.beginPath();
        c.arc(mid[0], mid[1], size * (1 + lit * 1.1), 0, Math.PI * 2);
        c.stroke();
        c.globalAlpha = 1;
      }
    }

    if (!struck && t >= STRIKE_MS) {
      struck = true;
      hitSound.currentTime = 0;

      hitSound.play().catch(() => {});
    }
    if (t > STRIKE_MS + AFTER_HIT_MS) {
      locked = false;
      hideSplash();
      return;
    }
    scene_run = requestAnimationFrame(frame);
  };
  scene_run = requestAnimationFrame(frame);
}

let idlePlay = null;
let idleShelf = null;
let idleAsking = false;

const FADE_MS = 900;
let idleFade = 0;

let idleAt = 0;

async function nextIdlePlay() {
  if (idleAsking) return;
  idleAsking = true;
  try {
    if (!idleShelf) {
      const plays = await invoke("my_replays", { most: 60 });
      idleShelf = plays.filter((play) => play.have_map);
    }
    if (idleShelf.length) {
      const pick = idleShelf[idleAt % idleShelf.length];
      idleAt += 1;
      const opened = await invoke("judged", { replay: pick.path, fine: options().fine });
      idlePlay = { scene: opened, judged: opened.summary, head: opened.from_ms, show: opened.show, path: pick.path };
      idleFade = 0;
    }
  } catch {
    idleShelf = idleShelf || [];
  } finally {
    idleAsking = false;
  }
}

function restSplash(c, box) {
  const dpr = box.w / Math.max(1, splashView.clientWidth);
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, splashView.clientWidth, splashView.clientHeight);
  drawResting(c, splashView.clientWidth, splashView.clientHeight);
}

function drawResting(c, w, h) {
  if (!markImage.complete || !markImage.naturalWidth) return;
  const { mid, size, side } = letterBox(w, h);
  c.globalAlpha = 0.5;
  c.drawImage(markImage, mid[0] - side / 2, mid[1] - side / 2, side, side);
  c.globalAlpha = 0.18;
  c.strokeStyle = "#e24848";
  c.lineWidth = 2;
  c.beginPath();
  c.arc(mid[0], mid[1], size * 1.35, 0, Math.PI * 2);
  c.stroke();
  c.globalAlpha = 1;
}

function playIdle() {
  const c = splashView.getContext("2d");
  idleRun += 1;
  const mine = idleRun;
  let was = performance.now();
  nextIdlePlay();

  let missed = 0;
  (async () => {
    while (mine === idleRun) {
      await new Promise((again) => requestAnimationFrame(again));
      if (mine !== idleRun) return;
      const box = canvasPixels(splashView, SPLASH_DPR, 1_200_000);
      const now = performance.now();
      const step = Math.min(64, now - was);
      was = now;
      const one = idlePlay;
      if (!box || !one) {
        if (box) restSplash(c, box);
        continue;
      }
      one.head += step;
      const left = one.scene.to_ms - one.head;
      if (left <= 0) {
        idlePlay = null;
        idleFade = 0;
        nextIdlePlay();
        restSplash(c, box);
        continue;
      }
      const image = await frameOf(one, one.head, box.w, box.h);
      if (mine !== idleRun) return;
      if (!image) {
        missed += 1;
        if (missed > 8) {
          missed = 0;
          idlePlay = null;
          nextIdlePlay();
        }
        continue;
      }
      missed = 0;
      idleFade = Math.min(1, idleFade + step / FADE_MS);
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.clearRect(0, 0, box.w, box.h);
      c.globalAlpha = Math.min(idleFade, Math.max(0, left / FADE_MS));
      c.drawImage(image, 0, 0, box.w, box.h);
      c.globalAlpha = 1;
    }
  })();
}

function showSplash() {
  if (asleep) return;
  asleep = true;
  document.body.classList.add("covered");
  runPreviews();
  splash.classList.add("going");
  splash.hidden = false;
  void splash.offsetWidth;
  splash.classList.remove("going");
  playIdle();
}

function hideSplash() {
  if (!asleep || locked) return;
  asleep = false;
  idleRun += 1;
  if (scene_run) cancelAnimationFrame(scene_run);
  scene_run = null;
  document.body.classList.remove("covered");
  requestAnimationFrame(runPreviews);

  idlePlay = null;
  splash.classList.add("going");
  setTimeout(() => {
    if (!asleep) splash.hidden = true;
    uncover();
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
  runPreviews();
  if (!document.hidden) stirred();
});

const bugbox = byId("bugbox");
const bug = byId("bug");
const bugpane = byId("bugpane");
let pinned = false;

function openBug(yes, byHand = false) {
  bugpane.hidden = !yes;
  bug.setAttribute("aria-expanded", String(yes));
  if (yes && byHand) byId("bug-text").focus({ preventScroll: true });
}

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
    ready: () => true,

    sends: true,
    async send(said) {
      return invoke("send_report", { log: said + (await tail()), kind: "bug" });
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
      note("Напишите хотя бы пару слов...", true);
      byId("bug-text").focus();
      return;
    }
    button.disabled = true;
    try {
      if (route.sends) {
        note("Отправляю…");
        try {
          note(await route.send(said));
          byId("bug-text").value = "";
        } catch (why) {
          const kept = said + (await tail());
          const copied = await navigator.clipboard
            .writeText(kept)
            .then(() => true)
            .catch(() => false);
          note(`${why}${copied ? ". Отчёт скопирован в буфер обмена" : ""}`, true);
        }
      } else {
        await invoke("open_link", { url: await route.link(said) });
      }
    } catch (why) {
      note(`${route.sends ? "Не отправилось" : "Не открылось"}: ${why}`, true);
    } finally {
      button.disabled = false;
    }
  });
  byId("bug-routes").append(button);
}

byId("repo").addEventListener("click", (event) => {
  event.preventDefault();
  invoke("open_link", { url: CONTACT.repo }).catch(() => {});
});

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
      : `${said.version} · Отстаёт на ${said.behind} ${plural(said.behind, "коммит", "коммита", "коммитов")}`;

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
  byId("up-version").textContent = "Спрашиваю…";
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
  note.textContent = "Обновляю…";
  try {
    await invoke("update_run");
    note.textContent = "Готово — перезапустите приложение";
    byId("up-head").textContent = "Собрано";
  } catch (why) {
    note.textContent = `Не собралось: ${why}`;
    offerReport();
  } finally {
    go.disabled = false;
  }
});

async function offerReport() {
  const after = byId("up-after");
  const how = remembered("report", "ask");
  if (how === "never") {
    after.replaceChildren(el("span", "muted", "Журнал никуда не отправлен по политике приложения."));
    after.hidden = false;
    return;
  }
  if (how === "auto") {
    after.replaceChildren(el("span", "muted", "Отправляю разработчику…"));
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
    const said = await invoke("send_report", { log: logged, kind: "build" });
    after.replaceChildren(el("span", "muted", `${said} — вместе с версией, системой и журналом`));
  } catch (why) {
    after.replaceChildren(el("span", "muted", `Отправить не вышло: ${why}`));
  }
}

async function showReady() {
  const box = byId("ready-rows");
  const verdict = byId("ready-verdict");
  let rows;
  try {
    rows = await invoke("ready");
  } catch (why) {
    verdict.textContent = `Не удалось спросить: ${why.message}`;
    verdict.className = "verdict bad";
    return [];
  }
  box.replaceChildren(
    ...rows.map((row) => line({ mark: sign(row.ok), name: row.name, said: row.said, fix: row.ok === false ? row.fix : "" })),
  );
  const stopped = rows.filter((row) => row.ok === false).length;
  verdict.textContent = stopped
    ? `${stopped} ${plural(stopped, "пункт", "пункта", "пунктов")} надо поправить`
    : "Готово — можно брать работу";
  verdict.className = stopped ? "verdict bad" : "verdict";

  const asking = line({ mark: ["huh", "?"], name: "Сборка", said: "Спрашиваю у бота…" });
  box.append(asking);
  invoke("handshake")
    .then((row) =>
      asking.replaceWith(
        line({ mark: sign(row.ok), name: row.name, said: row.said, fix: row.ok === false ? row.fix : "" }),
      ),
    )
    .catch((why) => asking.replaceWith(line({ mark: ["no", "!"], name: "Сборка", said: `${why}` })));
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
    head.replaceChildren(el("span", null, `Не удалось измерить: ${why.message}`));
    return;
  }

  if (told.speed) {
    head.replaceChildren(
      el("b", null, round(told.speed.estimated)),
      el("span", null, `кадров в секунду · ${told.speed.width}×${told.speed.height} · ${told.capacity.threads} ${plural(told.capacity.threads, "поток", "потока", "потоков")}`),
    );
  } else {
    head.replaceChildren(el("span", null, told.could_not_measure || "Измерить не удалось"));
  }

  const hardware = told.hardware;
  const rows = [
    { mark: ["ok", "·"], name: "Процессор", said: hardware.cpu },
    { mark: ["ok", "·"], name: "Ядер", said: String(hardware.cores) },
    { mark: ["ok", "·"], name: "ОЗУ", said: hardware.memory_gb ? `${round(hardware.memory_gb, 1)} ГБ` : "неизвестно" },
    { mark: ["ok", "·"], name: "Система", said: hardware.os },
    {
      mark: hardware.encoders.length ? ["ok", "+"] : ["huh", "?"],
      name: "Кодировщики",
      said: hardware.encoders.length ? hardware.encoders.join(", ") : "только программный",
    },
    {
      mark: sign(told.capacity.take),
      name: "Доступность",
      said: told.capacity.reason,
    },
  ];
  if (told.speed) {
    rows.push({ mark: ["ok", "·"], name: "На поток", said: `${round(told.speed.per_thread)} кадров/с` });
  }
  box.replaceChildren(...rows.map(line));
}

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
    said.textContent = `Сеть не видно: ${why}`;
    return;
  }
  byId("f-waiting").textContent = farm.waiting
    ? `${farm.waiting} ${plural(farm.waiting, "задача", "задачи", "задач")} в очереди`
    : "Очередь пуста";
  if (!farm.workers.length) {
    box.replaceChildren(line({ mark: ["huh", "?"], name: "Никого", said: "Ни одна машина не находится в сети" }));
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

const FIELDS = ["server", "token", "name", "songs", "skins", "replays", "skin", "window", "on_close"];

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

    if (note) note.textContent = "";
    return true;
  } catch (why) {
    if (note) {
      note.className = "verdict bad";
      note.textContent = `Не сохранилось: ${why}`;
    }
    return false;
  }
}

let measured = false;

async function installSkin(path) {
  const note = byId("s-said");
  try {
    const name = await invoke("install_skin", { path });

    fillSkins(byId("s-skin"), await invoke("skins").catch(() => []), name);
    await saveSettings("s", "s-said");
    note.className = "verdict";
    note.textContent = `Скин «${name}» поставлен и выбран по умолчанию`;
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
  songs: "Папка с картами (обычно osu!/Songs)",
  skins: "Папка со скинами (обычно osu!/Skins)",
  replays: "Папка с реплеями (обычно osu!/Replays)",
};

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

let shelfCounts = null;

async function readShelves() {
  const shelves = await invoke("shelves").catch(() => null);
  byId("s-shelves").hidden = !shelves;
  if (!shelves) return "Не удалось прочитать папки";

  byId("s-shelves").replaceChildren(
    ...[["Карты", shelves.songs], ["Скины", shelves.skins], ["Реплеи", shelves.replays]].map(([name, shelf]) =>
      line({
        mark: shelf.exists ? ["ok", "+"] : ["huh", "?"],
        name,
        said: shelf.exists ? shelf.note : `${shelf.note} — ${shelf.path}`,
      }),
    ),
  );

  const now = { songs: shelves.songs.items, skins: shelves.skins.items, replays: shelves.replays.items };
  const was = shelfCounts;
  shelfCounts = now;
  if (!was) return "Что лежит в папках сейчас.";

  const grew = [
    ["карта", "карты", "карт", now.songs - was.songs],
    ["скин", "скина", "скинов", now.skins - was.skins],
    ["реплей", "реплея", "реплеев", now.replays - was.replays],
  ]
    .filter(([, , , by]) => by > 0)
    .map(([one, few, many, by]) => `${by} ${plural(by, one, few, many)}`);
  return grew.length ? `Нашлось: ${grew.join(", ")}.` : "Ничего нового не появилось.";
}

async function showSettings() {
  await loadSettings();
  const skins = await invoke("skins").catch(() => []);
  fillSkins(byId("s-skin"), skins, known.skin);
  const shape = byId("s-window");
  shape.value = known.window || "980x720";
  dressSelect(shape);
  closesInto(known.on_close || "quit", false);
  asksMaps(remembered("askmaps", "on"), false);
  for (const [id, key, fallback] of [
    ["s-askfrom", "askfrom", "3"],
    ["s-atonce", "atonce", "1"],
    ["s-holdfor", "holdfor", "12"],
    ["s-mirror", "mirror", ""],
  ]) {
    const box = byId(id);
    box.value = remembered(key, fallback);
    dressSelect(box);
  }
  loadRenderSettings();
  byId("s-found").textContent = await readShelves();
  if (!document.querySelector('#view-settings .page[data-page="skins"]').hidden) runFitting();
  showReady();

  if (!measured) {
    measured = true;
    showMachine();
  }
}

byId("s-rescan").addEventListener("click", async () => {
  const button = byId("s-rescan");
  const said = byId("s-found");
  button.disabled = true;
  said.textContent = "Смотрю…";

  shelfCache = null;
  try {
    const skins = await invoke("skins").catch(() => []);
    fillSkins(byId("s-skin"), skins, known.skin);
    said.textContent = await readShelves();
  } finally {
    button.disabled = false;
  }
});

function asksMaps(how, save = true) {
  const box = byId("seg-ask");
  pressed(box, box.querySelector(`[data-ask="${how}"]`));
  if (save) remember("askmaps", how);
}

for (const button of byId("seg-ask").querySelectorAll("button")) {
  button.addEventListener("click", () => asksMaps(button.dataset.ask));
}

for (const [id, key, fallback] of [
  ["s-askfrom", "askfrom", "3"],
  ["s-atonce", "atonce", "1"],
  ["s-holdfor", "holdfor", "12"],
  ["s-mirror", "mirror", ""],
]) {
  byId(id).addEventListener("change", () => remember(key, byId(id).value));
  void fallback;
}

function closesInto(how, save = true) {
  const box = byId("seg-close");
  pressed(box, box.querySelector(`[data-close="${how}"]`));
  known.on_close = how;
  if (save) saveSettings("s", "s-said");
}

for (const button of byId("seg-close").querySelectorAll("button")) {
  button.addEventListener("click", () => closesInto(button.dataset.close));
}

byId("s-window").addEventListener("change", () => {
  known.window = byId("s-window").value;
  saveSettings("s", "s-said");
});

byId("s-loud").addEventListener("change", () => {
  remember("loud", byId("s-loud").value);
  loudness();
});

function loudness() {
  const level = Number(remembered("loud", "0.35"));
  hitSound.volume = Math.min(1, Math.max(0, level));
  return hitSound.volume;
}

byId("s-save").addEventListener("click", () => saveSettings("s", "s-said"));
byId("s-skin").addEventListener("change", async () => {
  await saveSettings("s", "s-said");
  await freshSkin();
});

const FIT_PARTS = 3;
const FIT_PIECE_MS = 5200;
const FIT_BLEND_MS = 900;

let fitRun = 0;
let fitParts = null;
let fitAt = 0;
let fitLoading = false;

async function loadFitting(again = false) {
  if (fitLoading) return;
  if (fitParts && !again) return;
  fitLoading = true;
  try {
    const plays = await invoke("my_replays", { most: 40 }).catch(() => []);
    const withMap = plays.filter((play) => play.have_map);
    if (!withMap.length) {
      fitParts = [];
      byId("s-skinsaid").textContent = "Примерять не на чем: нет ни одного реплея с картой.";
      return;
    }
    const pool = withMap.slice();
    const picked = [];
    while (picked.length < Math.min(FIT_PARTS, pool.length)) {
      picked.push(pool.splice(Math.floor(Math.random() * pool.length), 1)[0]);
    }
    const made = await Promise.all(
      picked.map((play) =>
        invoke("preview", { replay: play.path, fine: options().fine })
          .then((scene) => {
            const one = { scene, head: scene.from_ms, at: 0, show: 0, path: play.path, who: play.player };
            rollPart(one, 0);
            return one;
          })
          .catch(() => null),
      ),
    );
    fitParts = made.filter(Boolean);
    fitAt = 0;
    if (!fitParts.length) byId("s-skinsaid").textContent = "Ни один реплей не открылся.";
    await Promise.all(fitParts.map((one) => frameOf(one, one.head, 64, 48)));
  } finally {
    fitLoading = false;
  }
}

function nextFitPiece() {
  if (!fitParts || !fitParts.length) return;
  fitAt = (fitAt + 1) % fitParts.length;
  const one = fitParts[fitAt];
  const parts = one.scene.parts;
  if (parts && parts.length) {
    one.at = Math.floor(Math.random() * parts.length);
    one.head = parts[one.at][0];
  } else {
    one.head = one.scene.from_ms;
  }
}

function runFitting() {
  fitRun += 1;
  const view = byId("s-fitview");
  const page = document.querySelector('#view-settings .page[data-page="skins"]');
  if (!view || !page || page.hidden) return;
  const mine = fitRun;
  let was = performance.now();
  let shown = 0;
  (async () => {
    while (mine === fitRun && !fitParts) {
      await loadFitting();
      if (mine !== fitRun) return;
    }
    if (!fitParts || !fitParts.length) return;

    let seat = canvasPixels(view, 1.5);
    if (!seat) return;
    let ahead = frameOf(fitParts[fitAt], fitParts[fitAt].head, seat.w, seat.h);
    while (mine === fitRun && !page.hidden) {
      const image = await ahead;
      if (mine !== fitRun) return;
      seat = canvasPixels(view, 1.5) || seat;
      const one = fitParts[fitAt];
      const now = performance.now();
      const step = Math.min(120, now - was);
      was = now;

      if (!image) {
        one.misses = (one.misses || 0) + 1;
        if (one.misses > 3) {
          fitParts = fitParts.filter((other) => other !== one);
          fitAt = 0;
          if (!fitParts.length) {
            byId("s-skinsaid").textContent = "Реплеи для примерки не открылись.";
            return;
          }
        }
        shown = 0;
        nextFitPiece();
        ahead = frameOf(fitParts[fitAt], fitParts[fitAt].head, seat.w, seat.h);
        continue;
      }
      one.misses = 0;
      const soft = clamp01(shown / FIT_BLEND_MS);

      if (motionOn()) rollPart(one, step);
      shown += step;
      if (shown >= FIT_PIECE_MS) {
        shown = 0;
        nextFitPiece();
      }
      const next = fitParts[fitAt];
      ahead = frameOf(next, next.head, seat.w, seat.h);
      await new Promise((again) => requestAnimationFrame(again));
      if (mine !== fitRun || page.hidden) return;
      paintFrame(seat, image, soft);
    }
  })();
}

function stopFitting() {
  fitRun += 1;
}

byId("s-clone").addEventListener("click", async () => {
  const name = byId("s-skin").value;
  const said = byId("s-skinsaid");
  if (!name) {
    said.textContent = "«" + DEFAULT_SKIN + "» не лежит в папке — клонировать нечего.";
    return;
  }
  said.textContent = "Копирую…";
  try {
    const made = await invoke("clone_skin", { name });
    const skins = await invoke("skins").catch(() => []);
    fillSkins(byId("s-skin"), skins, made);
    known.skin = made;
    await saveSettings("s", "s-said");
    await freshSkin();
    said.textContent = `Скопирован как «${made}».`;
  } catch (why) {
    said.textContent = `${why}`;
  }
});

byId("s-drop").addEventListener("click", () => {
  const name = byId("s-skin").value;
  const said = byId("s-skinsaid");
  if (!name) {
    said.textContent = "«" + DEFAULT_SKIN + "» не лежит в папке — удалять нечего.";
    return;
  }
  holdWall({
    head: `Удалить скин «${name}»?`,
    why: "Действие необратимо удалит папку скина с Вашего устройства",
    doing: "Да, я хочу удалить!",
    nope: "Нет, я передумал",
    yes: async () => {
      try {
        await invoke("remove_skin", { name });
        const skins = await invoke("skins").catch(() => []);
        known.skin = "";
        fillSkins(byId("s-skin"), skins, "");
        await saveSettings("s", "s-said");
        await freshSkin();
        said.textContent = `Скин «${name}» удалён.`;
        tellWall("Скин удалён", "Папки больше нет на этом устройстве.");
        wallOk.textContent = "Хорошо";
        wallFace("done");
      } catch (why) {
        said.textContent = `${why}`;
        tellWall("Скин не удалился", `${why}`);
        wallFace("part");
      }
    },
  });
});

byId("s-export").addEventListener("click", async () => {
  const name = byId("s-skin").value;
  const said = byId("s-skinsaid");
  if (!name) {
    said.textContent = "«" + DEFAULT_SKIN + "» не лежит в папке — выгружать нечего.";
    return;
  }
  try {
    const into = await invoke("pick_folder", { prompt: "Куда положить .osk" });
    if (!into) return;
    said.textContent = "Собираю архив…";
    const file = await invoke("export_skin", { name, into });
    said.textContent = `Готово: ${file}`;
  } catch (why) {
    said.textContent = `${why}`;
  }
});

async function freshSkin() {
  cardTicket += 1;
  stopFitting();
  for (const made of previews.values()) {
    if (made && made.show) invoke("show_shut", { show: made.show }).catch(() => {});
  }
  previews.clear();
  fitParts = null;
  runFitting();
  for (const card of document.querySelectorAll("#r-list .rep")) {
    card.classList.remove("read", "plain");
    const where = card.querySelector(".map");
    if (where) where.textContent = "…";
    askPreview(card);
  }
  if (scene) drawView();
}
byId("again").addEventListener("click", showReady);
byId("measure").addEventListener("click", () => {
  measured = true;
  showMachine();
});
byId("f-again").addEventListener("click", showFarm);

const TAGS = { builtin: "встроен", found: "на месте", missing: "отсутствует", planned: "в планах" };

async function showLibrary() {
  const box = byId("lib-rows");
  let mods;
  try {
    mods = await invoke("modules");
  } catch (why) {
    box.replaceChildren(line({ mark: ["no", "!"], name: "Библиотека", said: `${why}` }));
    return;
  }
  const groups = [
    ["Ядро Dossier", "Наши собственные части. Живут внутри приложения и обновляются вместе с ним.", (mod) => mod.state === "builtin"],
    ["Сторонние зависимости", "Чужое, без чего не обойтись. Ставится отдельно с указанием источников для установки.", (mod) => mod.state !== "builtin" && mod.state !== "planned"],
    ["В разработке", "Задумано, но ещё не сделано. Стоит здесь, чтобы не выглядеть пропажей.", (mod) => mod.state === "planned"],
  ];

  const parts = [];
  let at = 0;
  for (const [name, about, belongs] of groups) {
    const mine = mods.filter(belongs);
    if (!mine.length) continue;
    const part = el("section", "part");
    part.style.animationDelay = `${at * 0.07}s`;
    at += 1;
    const head = el("header");
    head.append(el("h3", null, name), el("p", null, about));
    part.append(head);
    const card = el("div", "card");
    card.append(...mine.map(modRow));
    part.append(card);
    parts.push(part);
  }
  box.replaceChildren(...parts);

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

let drawing = false;
let drawFile = null;

let watching = { bar: "r-bar", said: "r-said", share: "r-share" };

let job = null;


const jobBox = byId("jobbox");
const jobMini = byId("job-mini");

const DONE_HOLD_MS = 12000;
let jobHold = null;

const works = new Map();
let workSeq = 0;
let openWork = null;

const LOG_KEPT = 400;

function startWork({ kind, label, home }) {
  workSeq += 1;
  const id = `w${workSeq}`;
  works.set(id, { id, kind, label, home, share: 0, note: "Начинаю…", done: false, ok: true, lines: [] });
  openWork = id;
  paintWorks();
  return id;
}

function stepWork(id, share, note) {
  const one = works.get(id);
  if (!one) return;
  if (typeof share === "number") one.share = Math.min(100, Math.max(0, share));
  if (note) one.note = note;
  paintWorks();
}

function logWork(id, line) {
  const one = works.get(id);
  if (!one) return;
  one.lines.push(line);
  if (one.lines.length > LOG_KEPT) one.lines.splice(0, one.lines.length - LOG_KEPT);
  paintWorks();
}

function endWork(id, ok, note) {
  const one = works.get(id);
  if (!one) return;
  one.done = true;
  one.ok = ok;
  one.share = 100;
  if (note) one.note = note;
  paintWorks();
  const holdFor = Number(remembered("holdfor", "12")) * 1000;
  clearTimeout(jobHold);
  if (holdFor <= 0) return;
  const sweep = () => {
    if (workHover) {
      jobHold = setTimeout(sweep, 1200);
      return;
    }
    works.delete(id);
    if (openWork === id) openWork = null;
    paintWorks();
  };
  jobHold = setTimeout(sweep, holdFor);
}

const alive = () => [...works.values()].filter((one) => !one.done);

let workHover = false;

for (const kind of ["pointerenter", "pointerleave"]) {
  jobBox.addEventListener(kind, () => {
    workHover = kind === "pointerenter";
  });
}

function paintWorks() {
  const all = [...works.values()];
  jobBox.hidden = false;
  const list = byId("job-works");
  if (!all.length) {
    jobMini.classList.add("idle");
    jobMini.classList.remove("done", "failed");
    if (!looping.has(document.querySelector(".pull .stem"))) restartLoops();
    byId("job-label").textContent = "";
    byId("job-percent").textContent = "0";
    list.replaceChildren(el("p", "fine", "Сейчас ничего не идёт."));
    byId("job-hint").textContent = "Здесь видно всё, что приложение качает и рисует.";
    return;
  }
  jobMini.classList.remove("idle");
  byId("job-hint").textContent = "Нажмите на работу, чтобы открыть её журнал.";
  const running = alive();
  const share = running.length
    ? running.reduce((sum, one) => sum + one.share, 0) / running.length
    : 100;
  const lead = running[0] || all[all.length - 1];

  const resting = !running.length;
  jobMini.classList.toggle("done", resting && all.every((one) => one.ok));
  jobMini.classList.toggle("failed", resting && all.some((one) => !one.ok));
  if (resting) {
    for (const what of [".pull .stem", ".pull .head", ".pull .tray"]) stopLoop(what);
  } else if (!looping.has(document.querySelector(".pull .stem"))) {
    restartLoops();
  }
  byId("job-label").textContent = running.length > 1 ? `${running.length} работы` : lead.label;
  byId("job-percent").textContent = round(share);
  list.replaceChildren(
    ...all.map((one) => {
      const row = el("div", `work${one.done ? (one.ok ? " done" : " failed") : ""}${openWork === one.id ? " open" : ""}`);
      const head = el("button", "workhead");
      head.append(
        el("b", null, one.label),
        el("span", "pct", one.done && !one.ok ? "сбой" : `${round(one.share)}%`),
      );
      head.addEventListener("click", () => {
        openWork = openWork === one.id ? null : one.id;
        paintWorks();
      });
      row.append(head);
      if (one.done) {
        const drop = el("button", "workshut", "");
        drop.setAttribute("aria-label", "Убрать");
        drop.innerHTML = '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M7 7 17 17M17 7 7 17"/></svg>';
        drop.addEventListener("click", (event) => {
          event.stopPropagation();
          works.delete(one.id);
          if (openWork === one.id) openWork = null;
          paintWorks();
        });
        row.append(drop);
      }
      const bar = el("div", "workbar");
      bar.append(el("i", null, ""));
      bar.firstChild.style.width = `${one.share}%`;
      row.append(bar);
      row.append(el("p", "worknote", one.note));
      if (openWork === one.id && one.lines.length) {
        const log = el("pre", "worklog", one.lines.join("\n"));
        row.append(log);
        requestAnimationFrame(() => {
          log.scrollTop = log.scrollHeight;
        });
      }
      if (one.home && !one.done) {
        const go = el("button", "act small", "Открыть");
        go.addEventListener("click", () => show(one.home));
        row.append(go);
      }
      return row;
    }),
  );
}

function startJob(home, label) {
  job = { home, label, done: false, id: startWork({ kind: "render", label: label.split(" · ")[0], home }) };
  return job.id;
}

function paintJob(percent, note) {
  if (job && job.id) stepWork(job.id, percent, note);
}

function syncJobMini() {
  paintWorks();
}

jobMini.addEventListener("click", () => {
  const running = alive();
  const home = (running[0] && running[0].home) || (job && job.home);
  if (home) show(home);
});

function dismissJob() {
  if (job && job.id) works.delete(job.id);
  job = null;
  paintWorks();
}

function endJob(ok) {
  if (!job) return;
  endWork(job.id, ok, ok ? "Готово" : "Не вышло");
  job.done = true;
}

function showFinished(slot, { path, notes, bad, head }) {
  const card = el("div", bad ? "finished bad" : "finished");

  const seal = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  seal.setAttribute("class", "seal");
  seal.setAttribute("viewBox", "0 0 24 24");
  seal.innerHTML = bad
    ? '<circle cx="12" cy="12" r="10"/><path d="M8.4 8.4 15.6 15.6M15.6 8.4 8.4 15.6"/>'
    : '<circle cx="12" cy="12" r="10"/><path d="M7.4 12.4 10.5 15.4 16.6 9.1"/>';
  card.append(seal);

  const said = el("div", "said");
  said.append(el("b", null, head));
  if (bad) {
    if (notes && notes.length) said.append(el("p", "facts", notes.join(" · ")));
  } else {
    const facts = el("p", "facts");
    facts.append("Техническая информация ");
    const where = el("button", "asknote", "в логах");
    where.addEventListener("click", () => {
      invoke("log_open").catch((why) => {
        facts.replaceChildren(`Лог не открылся: ${why}`);
      });
    });
    facts.append(where);
    said.append(facts);
  }
  card.append(said);

  const shut = el("button", "shut small", "");
  shut.setAttribute("aria-label", "Скрыть");
  shut.innerHTML = '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M7 7 17 17M17 7 7 17"/></svg>';
  shut.addEventListener("click", () => {
    card.classList.add("leaving");
    card.addEventListener("transitionend", () => slot.replaceChildren(), { once: true });
    setTimeout(() => slot.replaceChildren(), 700);
  });

  const routes = el("div", "routes");
  if (path && !bad) {
    const open = el("button", "act small primary", "Открыть");
    const where = el("button", "act small", "Показать в папке");
    open.addEventListener("click", () => {
      invoke("play_file", { path }).catch((why) => {
        said.querySelector(".facts")?.remove();
        said.append(el("p", "facts", String(why)));
      });
    });
    where.addEventListener("click", () => {
      invoke("reveal_file", { path }).catch((why) => {
        said.querySelector(".facts")?.remove();
        said.append(el("p", "facts", String(why)));
      });
    });
    routes.append(open, where);
  }
  routes.append(shut);
  card.append(routes);
  slot.replaceChildren(card);
}

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
      hit_lighting: remembered("lighting", "1") === "1",
      snake_in: remembered("snakein", "0") === "1",
      snake: remembered("snake", "0") === "1",
      cursor_expand: remembered("expand", "1") === "1",
      map_hitsounds: remembered("mapsounds", "1") === "1",
      skin_hitsounds: remembered("skinsounds", "1") === "1",
      kit: remembered("kit", "click"),
      pitch: Number(remembered("pitch", "1")),
      decay: Number(remembered("decay", "1")),
      kit_level: Number(remembered("kitlevel", "1")),
    },
  };
}

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
  ["s-expand", "expand", "1"],
  ["s-lighting", "lighting", "1"],
  ["s-snakein", "snakein", "0"],
  ["s-snake", "snake", "0"],
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
    refreshShows();
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

let shelfCache = null;

function scanning(on) {
  byId("r-scan").hidden = !on;
  byId("r-bar-line").hidden = on;
  byId("r-steps").hidden = on;
  byId("r-list").hidden = on;
  if (on) {
    byId("r-busy").hidden = true;
    byId("r-done").textContent = "";
    byId("r-doneslot").replaceChildren();
  }
  document.body.classList.toggle("scanning-now", on);
  restartLoops();
}

async function showRender(again = false) {
  const list = byId("r-list");
  const said = byId("r-done");

  if (shelfCache && !again) {
    scanning(false);
    fillPlays(shelfCache);
    return;
  }

  scanning(true);
  list.replaceChildren();

  const [settings, shelves, skins, plays, rows] = await Promise.all([
    invoke("settings_read").catch(() => ({})),
    invoke("shelves").catch(() => null),
    invoke("skins").catch(() => []),
    invoke("my_replays", {}).catch(() => []),
    invoke("ready").catch(() => []),
  ]);
  known = { ...known, ...settings };
  shelfCache = { shelves, skins, plays, rows };
  await uncovered;
  scanning(false);
  fillPlays(shelfCache);
}

const SORTS = {
  new: (a, b) => b.played_at - a.played_at,
  old: (a, b) => a.played_at - b.played_at,
  score: (a, b) => b.score - a.score,
  combo: (a, b) => b.combo - a.combo,
  player: (a, b) => a.player.localeCompare(b.player, "ru"),
};

const ONLY = {
  all: () => true,
  map: (play) => play.have_map,
  nomap: (play) => !play.have_map,
};

let finding = "";

function matches(play, words) {
  if (!words.length) return true;
  const made = previews.get(play.path);
  const title = made && made.judged ? made.judged.title : "";
  const hay = `${play.player} ${play.file} ${play.mods || "NM"} ${title}`.toLowerCase();
  return words.every((word) => hay.includes(word));
}

function chosenPlays(plays) {
  const how = SORTS[remembered("sort", "new")] || SORTS.new;
  const which = ONLY[remembered("only", "all")] || ONLY.all;
  const words = finding.toLowerCase().split(/\s+/).filter(Boolean);
  return plays
    .filter(which)
    .filter((play) => matches(play, words))
    .slice()
    .sort(how);
}

for (const [id, key] of [
  ["r-sort", "sort"],
  ["r-only", "only"],
]) {
  byId(id).addEventListener("change", () => {
    remember(key, byId(id).value);
    if (shelfCache) fillPlays(shelfCache);
  });
}

let findSoon = null;
byId("r-find").addEventListener("input", () => {
  clearTimeout(findSoon);
  findSoon = setTimeout(() => {
    finding = byId("r-find").value.trim();
    if (shelfCache) fillPlays(shelfCache);
  }, 160);
});

function fillPlays({ shelves, skins, plays, rows }) {
  const list = byId("r-list");
  const ffmpeg = rows.find((row) => row.name === "ffmpeg");
  const steps = [
    {
      n: 1,
      name: "Папка реплеев",
      note: shelves && shelves.replays.exists ? shelves.replays.note : "Откуда брать, что рендерить",
      done: Boolean(shelves && shelves.replays.exists),
      act: "Обзор…",
      go: async () => (await pickFolder("replays")) && showRender(true),
    },
    {
      n: 2,
      name: "Папка карт",
      note: shelves && shelves.songs.exists ? shelves.songs.note : "По ней ищется карта, на которой играли",
      done: Boolean(shelves && shelves.songs.exists),
      act: "Обзор…",
      go: async () => (await pickFolder("songs")) && showRender(true),
    },
    {
      n: 3,
      name: "ffmpeg",
      note: ffmpeg && ffmpeg.ok ? "нашёлся" : "Склеивает кадры в видео.",
      done: Boolean(ffmpeg && ffmpeg.ok),
      act: "Где взять",
      go: () => invoke("open_link", { url: "https://ffmpeg.org/download.html" }).catch(() => {}),
    },
    {
      n: 4,
      name: "Скин",
      note: skins.length
        ? `${skins.length} ${plural(skins.length, "скин", "скина", "скинов")} на выбор`
        : `Необязательно: без своего рисуется «${DEFAULT_SKIN}»`,
      done: true,
    },
  ];
  const left = steps.filter((step) => !step.done);
  byId("r-steps").replaceChildren(...(left.length ? steps.map(stepCard) : []));

  for (const [id, key, fallback] of [
    ["r-sort", "sort", "new"],
    ["r-only", "only", "all"],
  ]) {
    const box = byId(id);
    box.value = remembered(key, fallback);
    dressSelect(box);
  }

  const showing = chosenPlays(plays);
  if (!plays.length) {
    list.classList.add("waiting");
    list.replaceChildren(line({ mark: ["huh", "?"], name: "пусто", said: "Укажите папку реплеев." }));
    return;
  }
  list.classList.remove("waiting");
  forgetCards();
  requestAnimationFrame(shelfEdges);
  if (!showing.length) {
    list.replaceChildren(
      line({
        mark: ["huh", "?"],
        name: "пусто",
        said: finding ? `По «${finding}» ничего не нашлось.` : "Под этот отбор ничего не подошло.",
      }),
    );
  } else {
    list.replaceChildren(...showing.map((play) => playCard(play)));
  }
  offerMissingMaps(plays);

  function playCard(play) {
    const card = el("article", "rep");
    if (!play.have_map) card.classList.add("nomap");

    const stage = el("div", "stage");
    const canvas = document.createElement("canvas");
    stage.append(canvas);
    stage.append(el("div", "hush", play.have_map ? "" : "Карты нет"));

    const from = el("span", "from");
    from.hidden = true;
    const acc = el("span", "acc");
    acc.hidden = true;
    stage.append(from, acc);
    const scrim = el("div", "scrim");
    const go = el("button", "act small primary", play.have_map ? "Отрендерить" : "Скачать карту");
    go.addEventListener("click", (event) => {
      event.stopPropagation();
      if (play.have_map) draw(play);
      else showWall(play);
    });
    scrim.append(go);
    stage.append(scrim);
    card.append(stage);

    const said = el("div", "who");
    said.append(el("b", null, play.player));
    said.append(el("p", "map", "…"));
    const about = el("p", "about");
    about.append(modBadges(play.mods));
    said.append(about);
    card.append(said);

    card.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      openMenu(event.clientX, event.clientY, menuFor(play, card));
    });

    card.addEventListener("click", () => {
      if (play.have_map) openSheet(play, card);
      else showWall(play);
    });

    card.dataset.path = play.path;
    card.previewOf = play;
    card.previewOn = canvas;

    if (play.have_map) watchCard(card);
    else card.classList.add("plain");
    return card;
  }

  drawFile = draw;

  async function draw(play) {
    if (drawing) return;
    drawing = true;
    for (const other of list.querySelectorAll("button")) other.disabled = true;
    watching = { bar: "r-bar", said: "r-said", share: "r-share" };
    startJob("render", `Рендер · ${play.player}`);
    byId("r-done").textContent = "";
    byId("r-busy").hidden = false;
    byId("r-bar").style.width = "0%";
    byId("r-share").textContent = "0";
    byId("r-said").textContent = watchFailed
      ? `идёт, но без счётчика: окно не подписалось на события (${watchFailed})`
      : "Подготовка к рендеру…";
    list.classList.add("dimmed");
    const out = play.path.replace(/\.osr$/i, "") + ".mp4";
    const slot = byId("r-doneslot");
    slot.replaceChildren();
    try {
      const done = await invoke("draw", { replay: play.path, out, ...options() });
      showFinished(slot, {
        head: `Реплей ${play.player} отрисован`,
        path: done.path,
        notes: done.said,
      });
      endJob(true);
    } catch (why) {
      showFinished(slot, { head: "Рендер не вышел", notes: [String(why)], bad: true });
      endJob(false);
    } finally {
      drawing = false;

      for (const other of list.querySelectorAll("button")) other.disabled = false;
      byId("r-busy").hidden = true;
      list.classList.remove("dimmed");
    }
  }

  function menuFor(play, card) {
    const rows = [
      { name: "Отрендерить", off: !play.have_map || drawing, go: () => draw(play) },
      { name: "Посмотреть в Судействе", go: () => takeTo("judge", play.path) },
      { name: "Добавить в Студию", go: () => takeTo("cut", play.path) },
      { name: "Подробнее…", go: () => openSheet(play, card) },
    ];
    if (!play.have_map) rows.push({ line: true }, { name: "Скачать карту", go: () => showWall(play) });
    rows.push({ line: true }, {
      name: "Показать в папке",
      go: () => invoke("reveal_file", { path: play.path }).catch(() => {}),
    });
    rows.push({ name: "Удалить реплей", go: () => askToDrop(play) });
    return rows;
  }
}

const clamp01 = (t) => (t < 0 ? 0 : t > 1 ? 1 : t);

let modArt = null;

async function loadModArt() {
  if (modArt) return modArt;
  const said = await invoke("mod_icons", { high: 48 }).catch(() => null);
  modArt = new Map(said || []);
  return modArt;
}

function modsOf(text) {
  const clean = String(text || "").trim().toUpperCase();
  if (!clean || clean === "NM") return ["NM"];
  const out = [];
  for (let at = 0; at + 1 < clean.length; at += 2) out.push(clean.slice(at, at + 2));
  return out.length ? out : ["NM"];
}

function modBadges(text) {
  const box = el("span", "mods");
  for (const name of modsOf(text)) {
    const art = modArt && modArt.get(name);
    if (!art) {
      box.append(el("span", "modtext", name));
      continue;
    }
    const one = document.createElement("img");
    one.className = "modicon";
    one.src = art;
    one.alt = name;
    one.title = name;
    box.append(one);
  }
  return box;
}

function askToDrop(play) {
  holdWall({
    head: "Удалить реплей?",
    why: "Действие необратимо удалит файл с Вашего устройства",
    doing: "Да, я хочу удалить!",
    nope: "Нет, я передумал",
    yes: async () => {
      try {
        await invoke("drop_replay", { path: play.path });
        tellWall("Реплей удалён", "Файла больше нет на этом устройстве.");
        wallOk.textContent = "Хорошо";
        wallFace("done");
        shelfCache = null;
        showRender(true);
      } catch (why) {
        tellWall("Реплей не удалился", `${why}`);
        wallFace("part");
      }
    },
  });
}

const wall = byId("nomap");
const wallGet = byId("nomap-get");
const wallOk = byId("nomap-ok");
let wallFor = null;
let wallGot = false;
let wallAsk = null;

const WALL_WHY = "Скачай карту для реплея, чтобы взаимодействовать с реплеем.";

function showWall(play) {
  wallFor = play;
  wallGot = false;
  wallAsk = null;
  wallGet.onclick = null;
  holdDone = null;
  wallHold.hidden = true;
  byId("nomap-head").textContent = "Нет карты для реплея!";
  byId("nomap-why").textContent = WALL_WHY;
  wallGet.hidden = false;
  wallGet.disabled = false;
  wallGet.textContent = "Скачать карту";
  wallOk.textContent = "Понял!";
  document.querySelector(".walldo").classList.remove("stacked");
  wallWorking(false);
  wallFace("ask");
  wall.hidden = false;
  restartLoops();
  wallOk.focus();
}

function askWall({ head, why, yes, no, doing, nope }) {
  wallFor = null;
  wallGot = false;
  wallBulk = null;
  holdDone = null;
  wallHold.hidden = true;
  wallGet.onclick = null;
  wallAsk = { yes, no };
  byId("nomap-head").textContent = head;
  byId("nomap-why").textContent = why;
  wallGet.hidden = false;
  wallGet.disabled = false;
  wallGet.textContent = doing;
  wallOk.textContent = nope;
  document.querySelector(".walldo").classList.remove("stacked");
  wallWorking(false);
  wallFace("ask");
  wall.hidden = false;
  restartLoops();
  wallOk.focus();
}

const wallHold = byId("nomap-hold");
let holdRun = null;
let holdTicket = 0;
let holdDone = null;

function holdWall({ head, why, doing, nope, yes }) {
  wallFor = null;
  wallGot = false;
  wallAsk = null;
  wallBulk = null;
  wallGet.onclick = null;
  holdDone = yes;
  byId("nomap-head").textContent = head;
  byId("nomap-why").textContent = why;
  wallGet.hidden = true;
  wallOk.hidden = false;
  wallOk.textContent = nope;
  byId("nomap-fold").hidden = true;
  wallHold.hidden = false;
  document.querySelector(".walldo").classList.add("stacked");
  wallHold.querySelector("span").textContent = doing;
  wallHold.querySelector(".fill").style.width = "0%";
  wallFace("drop");
  wall.hidden = false;
  restartLoops();
  wallOk.focus();
}

const HOLD_MS = 2000;

function stopHold() {
  holdTicket += 1;
  if (holdRun) cancelAnimationFrame(holdRun);
  holdRun = null;
  wallHold.querySelector(".fill").style.width = "0%";
  wallHold.classList.remove("holding");
}

wallHold.addEventListener("pointerdown", (event) => {
  event.preventDefault();
  if (holdRun) return;
  if (wallHold.setPointerCapture) {
    try {
      wallHold.setPointerCapture(event.pointerId);
    } catch {
      void 0;
    }
  }
  wallHold.classList.add("holding");
  holdTicket += 1;
  const mine = holdTicket;
  const from = performance.now();
  const step = (now) => {
    if (mine !== holdTicket) return;
    const share = clamp01((now - from) / HOLD_MS);
    wallHold.querySelector(".fill").style.width = `${share * 100}%`;
    if (share >= 1) {
      const go = holdDone;
      stopHold();
      wallHold.hidden = true;
      holdDone = null;
      if (go) go();
      return;
    }
    holdRun = requestAnimationFrame(step);
  };
  holdRun = requestAnimationFrame(step);
});

for (const kind of ["pointerup", "pointercancel", "lostpointercapture", "blur"]) {
  wallHold.addEventListener(kind, stopHold);
}

function tellWall(head, why) {
  byId("nomap-head").textContent = head;
  byId("nomap-why").textContent = why;
}

async function shutWall() {
  if (wall.hidden) return;
  wall.hidden = true;
  const grabbed = wallGot;
  const asked = wallAsk;
  wallFor = null;
  wallGot = false;
  wallAsk = null;
  if (asked) return;
  if (!grabbed) return;
  shelfCache = null;
  await showRender(true);
}

wallOk.addEventListener("click", () => {
  if (wallAsk) {
    const no = wallAsk.no;
    wallAsk = null;
    wall.hidden = true;
    wallFor = null;
    if (no) no();
    return;
  }
  shutWall();
});
wall.addEventListener("pointerdown", (event) => {
  if (event.target === wall) shutWall();
});

wallGet.addEventListener("click", async () => {
  if (wallAsk) {
    const yes = wallAsk.yes;
    wallAsk = null;
    if (yes) yes();
    return;
  }
  const play = wallFor;
  if (!play || wallGet.disabled || wallGet.onclick) return;
  bulkDone = 0;
  bulkTotal = 1;
  lastStep = "";
  mapWork = startWork({ kind: "maps", label: "Карта", home: "render" });
  logWork(mapWork, play.file);
  wallWorking(true);
  byId("nomap-head").textContent = "Ищу карту…";
  byId("nomap-why").textContent = "Спрашиваю зеркало про эту карту.";
  try {
    const found = await invoke("fetch_map", { replay: play.path, mirror: remembered("mirror", "") || null });
    wallGot = true;
    logWork(mapWork, `  ✓ ${found.artist} — ${found.title} [${found.version}]`);
    endWork(mapWork, true, `${found.artist} — ${found.title}`);
    mapWork = null;
    wallWorking(false);
    wallFace("done");
    byId("nomap-head").textContent = "Карта на месте!";
    byId("nomap-why").textContent = `${found.artist} — ${found.title} [${found.version}]`;
    wallGet.disabled = false;
    wallGet.textContent = "Открыть подробности";
    wallGet.onclick = async () => {
      const path = play.path;
      await shutWall();
      const fresh = (shelfCache && shelfCache.plays) || [];
      const one = fresh.find((row) => row.path === path);
      if (one) openSheet(one, null);
    };
    wallOk.focus();
  } catch (why) {
    logWork(mapWork, `  × ${why}`);
    endWork(mapWork, false, `${why}`);
    mapWork = null;
    wallWorking(false);
    byId("nomap-head").textContent = "Карта не скачалась";
    byId("nomap-why").textContent = `${why}`;
    wallGet.disabled = false;
    wallGet.textContent = "Попробовать снова";
  }
});

const mb = (n) => (n / (1024 * 1024)).toFixed(1);

let mapWork = null;
let bulkDone = 0;
let bulkTotal = 0;
let lastStep = "";

function saySteps(payload) {
  if (!mapWork) return;
  const of = Number(payload.of) || 0;
  const got = Number(payload.got) || 0;
  const note = of && got ? `${mb(got)} из ${mb(of)} МБ · ${Math.round((got / of) * 100)}%` : payload.step;

  if (mapWork) {
    const part = of ? got / of : 0;
    const share = bulkTotal ? ((bulkDone + part) / bulkTotal) * 100 : part * 100;
    stepWork(mapWork, share, note);
    if (payload.step !== lastStep) {
      lastStep = payload.step;
      logWork(mapWork, `  ${payload.step}`);
    }
  }
  if (wall.hidden || holdDone || !wallFor || payload.replay !== wallFor.path) return;
  byId("nomap-head").textContent = wallBulk || "Качаю карту…";
  byId("nomap-why").textContent = note;
}

function wallFace(which) {
  for (const face of document.querySelectorAll(".wallface")) {
    if (face.dataset.face === which) face.removeAttribute("hidden");
    else face.setAttribute("hidden", "");
  }
  restartLoops();
}

function wallWorking(on) {
  wallGet.hidden = on;
  wallOk.hidden = on;
  if (on) wallHold.hidden = true;
  byId("nomap-fold").hidden = !on;
  if (on) restartLoops();
}

byId("nomap-fold").addEventListener("click", () => {
  wall.hidden = true;
});

const MISSING_ENOUGH = 3;
let askedAboutMaps = false;
let wallBulk = null;

function offerMissingMaps(plays) {
  if (askedAboutMaps || !wall.hidden) return;
  if (remembered("askmaps", "on") !== "on") return;
  const missing = plays.filter((play) => !play.have_map);
  if (missing.length < Number(remembered("askfrom", String(MISSING_ENOUGH)))) return;
  askedAboutMaps = true;
  askWall({
    head: "Не хотите ли вы скачать все недостающие карты для реплеев?",
    why: `${missing.length} ${plural(missing.length, "реплей ждёт свою карту", "реплея ждут свои карты", "реплеев ждут свои карты")}.`,
    doing: "Да",
    nope: "Нет, спасибо",
    yes: () => grabAllMaps(missing),
  });
}

async function grabAllMaps(missing) {
  let done = 0;
  let failed = 0;
  bulkDone = 0;
  bulkTotal = missing.length;
  mapWork = startWork({ kind: "maps", label: `Карты · ${missing.length}`, home: "render" });
  logWork(mapWork, `Не хватает карт: ${missing.length}`);
  wallWorking(true);

  const atOnce = Math.max(1, Math.min(3, Number(remembered("atonce", "1"))));
  const queue = missing.slice();
  const grab = async () => {
    while (queue.length) {
      const play = queue.shift();
      const at = done + failed + 1;
      if (atOnce === 1) {
        wallFor = play;
        wallBulk = `Карта ${at} из ${missing.length}`;
        lastStep = "";
        if (!wall.hidden) tellWall(wallBulk, play.file);
      }
      logWork(mapWork, `${at}. ${play.file}`);
      try {
        const found = await invoke("fetch_map", { replay: play.path, mirror: remembered("mirror", "") || null });
        done += 1;
        logWork(mapWork, `  ✓ ${found.artist} — ${found.title} [${found.version}]`);
      } catch (why) {
        failed += 1;
        logWork(mapWork, `  × ${why}`);
      }
      bulkDone += 1;
      stepWork(mapWork, (bulkDone / bulkTotal) * 100, `Скачано ${done}, не нашлось ${failed}`);
      if (atOnce > 1 && !wall.hidden) tellWall(`Карта ${bulkDone} из ${missing.length}`, `Скачано ${done}, не нашлось ${failed}`);
    }
  };
  await Promise.all(Array.from({ length: Math.min(atOnce, queue.length) }, grab));

  logWork(mapWork, failed ? `Готово: ${done} скачано, ${failed} не нашлось.` : `Готово: ${done} скачано.`);
  endWork(mapWork, failed === 0, failed ? `Скачано ${done}, не нашлось ${failed}` : `Скачано ${done}`);
  mapWork = null;
  bulkTotal = 0;
  wallBulk = null;
  wallFor = null;
  wallGot = done > 0;
  wallWorking(false);
  wallGet.hidden = true;
  wallOk.hidden = false;
  wallOk.textContent = "Понял!";
  wallFace(failed ? "part" : "done");
  tellWall(
    done ? (failed ? "Не всё нашлось" : "Все карты на месте!") : "Ничего не скачалось",
    failed ? `Скачано ${done}, не нашлось ${failed}.` : `Скачано ${done}.`,
  );
  if (wall.hidden && done) {
    shelfCache = null;
    showRender(true);
  }
}

function refreshShows() {
  for (const made of previews.values()) if (made) made.show = 0;
  if (live) live.show = 0;
  if (sheetPlay) sheetPlay.show = 0;
  if (idlePlay) idlePlay.show = 0;
  if (hovered) runPreviews();
  if (scene) drawView();
}

const FRAMES = navigator.userAgent.includes("Windows") ? "http://frame.localhost/" : "frame://localhost/";

function loadImage(src) {
  return new Promise((ready) => {
    const image = new Image();
    image.onload = () => ready(image);
    image.onerror = () => ready(null);
    image.src = src;
  });
}

function askFrame(show, ms, wide, high) {
  return new Promise((ready) => {
    if (!show || !wide || !high) return ready(null);
    const image = new Image();
    image.onload = () => ready(image);
    image.onerror = () => ready(null);
    image.src = `${FRAMES}?show=${show}&ms=${ms.toFixed(2)}&w=${Math.round(wide)}&h=${Math.round(high)}`;
  });
}

async function frameOf(made, ms, wide, high, still) {
  const first = await askFrame(made.show, ms, wide, high);
  if (first) return first;
  if (!made.path) return null;
  if (still && !still()) return null;
  try {
    made.show = await invoke("show_open", { replay: made.path, skin: made.skin, fine: options().fine });
  } catch {
    made.show = 0;
    return null;
  }
  return askFrame(made.show, ms, wide, high);
}

const MOST_PIXELS = 1_600_000;

function canvasPixels(canvas, cap, most = MOST_PIXELS) {
  const wide = canvas.clientWidth;
  const high = canvas.clientHeight;
  if (!wide || !high) return null;
  let dpr = Math.min(window.devicePixelRatio || 1, cap);
  const room = Math.sqrt(most / (wide * high * dpr * dpr));
  if (room < 1) dpr *= room;
  const w = Math.max(1, Math.round(wide * dpr));
  const h = Math.max(1, Math.round(high * dpr));
  const seat = { ctx: canvas.getContext("2d"), w, h };
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
    const before = lastShown.get(canvas);
    if (before) {
      seat.ctx.setTransform(1, 0, 0, 1, 0, 0);
      seat.ctx.drawImage(before, 0, 0, w, h);
    }
  }
  return seat;
}

function rollPart(one, step) {
  const parts = one.scene.parts && one.scene.parts.length ? one.scene.parts : null;
  if (!parts) {
    one.head += step;
    if (one.head > one.scene.to_ms) one.head = one.scene.from_ms;
    return;
  }
  if (one.at === undefined || one.at >= parts.length) {
    one.at = 0;
    one.head = parts[0][0];
  }
  one.head += step;
  if (one.head <= parts[one.at][1]) return;
  one.at = (one.at + 1) % parts.length;
  one.head = parts[one.at][0];
}

function partEdges(one) {
  const parts = one.scene.parts && one.scene.parts.length ? one.scene.parts : null;
  const from = parts ? parts[one.at || 0][0] : one.scene.from_ms;
  const to = parts ? parts[one.at || 0][1] : one.scene.to_ms;
  return { from_ms: from, to_ms: to };
}

const lastShown = new WeakMap();

function paintFrame(c, image, blend) {
  const canvas = c.ctx.canvas;
  const before = lastShown.get(canvas);
  const share = blend === undefined || !before ? 1 : clamp01(blend);
  c.ctx.setTransform(1, 0, 0, 1, 0, 0);
  c.ctx.clearRect(0, 0, c.w, c.h);
  if (share < 1) c.ctx.drawImage(before, 0, 0, c.w, c.h);
  c.ctx.globalAlpha = share;
  c.ctx.drawImage(image, 0, 0, c.w, c.h);
  c.ctx.globalAlpha = 1;
  lastShown.set(canvas, image);
}

function blendOf(one) {
  const edges = partEdges(one);
  return clamp01((one.head - edges.from_ms) / PREVIEW_EDGE_MS);
}

const PREVIEWS_KEPT = 60;

const PREVIEWS_AT_ONCE = 2;

const PREVIEW_EDGE_MS = 420;

const WAKE_MS = 520;

const previews = new Map();
const showing = new Set();
const asking = new Set();
const waitingFor = [];

let hovered = null;
let cardTicket = 0;
let previewSeen = null;

function forgetCards() {
  if (previewSeen) previewSeen.disconnect();
  showing.clear();
  waitingFor.length = 0;
  hovered = null;
  cardTicket += 1;
}

function watchCard(card) {
  if (!previewSeen) {
    previewSeen = new IntersectionObserver(
      (rows) => {
        for (const row of rows) {
          const one = row.target;
          if (row.isIntersecting) {
            showing.add(one);
            askPreview(one);
          } else {
            showing.delete(one);
          }
        }
      },
      { rootMargin: "160px" },
    );
  }
  card.addEventListener("pointerenter", () => {
    hovered = card;
    runPreviews();
  });
  card.addEventListener("pointerleave", () => {
    if (hovered !== card) return;
    hovered = null;
    cardTicket += 1;

    const made = previews.get(card.previewOf.path);
    if (made) {
      made.at = 0;
      rollPart(made, 0);
    }
    stillCard(card);
  });

  previewSeen.observe(card);
}

function askPreview(card) {
  const path = card.previewOf.path;
  if (previews.has(path)) {
    applyPreview(card, previews.get(path));
    return;
  }
  if (asking.has(path)) return;
  if (!waitingFor.includes(card)) waitingFor.push(card);
  pumpPreviews();
}

function applyPreview(card, made) {
  if (!made) {
    card.classList.add("plain");
    return;
  }
  dressCard(card, made.judged);
  stillCard(card);
}

function fetchPreview(path, wide, high) {
  if (asking.has(path)) return Promise.resolve(previews.get(path) || null);
  asking.add(path);
  return invoke("preview", {
    replay: path,
    fine: options().fine,
    width: Math.max(160, Math.round(wide)) || 480,
    height: Math.max(120, Math.round(high)) || 360,
  })
    .then((scene) => {
      const made = { scene, judged: scene.summary, head: scene.from_ms, at: 0, show: 0, still: scene.still || null, path };
      rollPart(made, 0);
      keepPreview(path, made);
      return made;
    })
    .catch(() => {
      keepPreview(path, null);
      return null;
    })
    .finally(() => {
      asking.delete(path);
    });
}

function pumpPreviews() {
  while (asking.size < PREVIEWS_AT_ONCE && waitingFor.length) {
    const card = waitingFor.shift();

    if (!showing.has(card)) continue;
    const path = card.previewOf.path;
    if (previews.has(path)) continue;
    const box = card.previewOn.getBoundingClientRect();
    fetchPreview(path, box.width * 1.5, box.height * 1.5)
      .then((made) => applyPreview(card, made))
      .finally(pumpPreviews);
  }
}

function outcomeOf(said) {
  const out = said.outcome || { kind: "miss", misses: said.counts.miss, share: 100 };
  if (out.kind === "fail") return { text: `Fail · ${round(out.share, 1)}%`, tone: "bad" };
  if (out.kind === "fc") return { text: "FC", tone: "good" };
  if (out.kind === "break") return { text: "SB", tone: "meh" };
  return { text: `×${round(out.misses)}`, tone: "bad" };
}

function dressCard(card, said) {
  const where = card.querySelector(".map");
  if (where) where.textContent = said.title;

  const mark = card.querySelector(".acc");
  if (mark) {
    const out = outcomeOf(said);
    mark.textContent = out.text;
    mark.className = `acc ${out.tone}`;
    mark.hidden = false;
  }

  const from = card.querySelector(".from");
  if (from && said.client) {
    from.textContent = said.client.name;
    from.hidden = false;
  }
  card.classList.add("read");
}

function keepPreview(path, made) {
  previews.set(path, made);
  while (previews.size > PREVIEWS_KEPT) {
    const oldest = previews.keys().next().value;
    if (oldest === path) break;
    previews.delete(oldest);

    const card = byId("r-list").querySelector(`.rep[data-path="${CSS.escape(oldest)}"]`);
    if (card) card.classList.remove("read");
  }
}

function stillOf(made) {
  if (made.stillImage) return Promise.resolve(made.stillImage);
  if (!made.still) return Promise.resolve(null);
  return loadImage(made.still).then((image) => {
    made.stillImage = image;
    return image;
  });
}

async function stillCard(card) {
  const made = previews.get(card.previewOf.path);
  if (!made) return;
  const c = canvasPixels(card.previewOn, 1.5);
  if (!c) return;
  const image = await stillOf(made);
  if (!image) {
    card.classList.add("plain");
    return;
  }
  card.classList.remove("plain");
  if (hovered === card) return;
  paintFrame(c, image);
}

function runPreviews() {
  cardTicket += 1;
  const one = hovered;
  const made = one && previews.get(one.previewOf.path);
  if (!made || !motionOn() || document.hidden || asleep) return;
  const mine = cardTicket;
  const alive = () => mine === cardTicket && hovered === one;
  let was = performance.now();
  let woke = 0;
  (async () => {
    let seat = canvasPixels(one.previewOn, 1);
    if (!seat) return;
    let ahead = frameOf(made, made.head, seat.w, seat.h, alive);
    while (alive()) {
      const image = await ahead;
      if (!alive()) return;
      seat = canvasPixels(one.previewOn, 1) || seat;
      const now = performance.now();
      const step = Math.min(120, now - was);
      was = now;
      woke += step;
      rollPart(made, step);
      const soft = Math.min(blendOf(made), clamp01(woke / WAKE_MS));
      ahead = frameOf(made, made.head, seat.w, seat.h, alive);
      await new Promise((again) => requestAnimationFrame(again));
      if (!alive()) return;
      if (image) paintFrame(seat, image, soft);
    }
  })();
}

const motionOn = () => remembered("motion", "on") === "on";

async function takeTo(which, path) {
  show("replay");
  if (await enter(which)) await openReplay(path);
}

const menuBox = byId("r-menu");

function openMenu(x, y, items) {
  menuBox.replaceChildren(
    ...items.map((item) => {
      if (item.line) return el("div", "sep");
      const button = el("button", null, item.name);
      button.disabled = Boolean(item.off);
      button.addEventListener("click", () => {
        shutMenu();
        item.go();
      });
      return button;
    }),
  );
  menuBox.hidden = false;

  const box = menuBox.getBoundingClientRect();
  const left = x + box.width > window.innerWidth - 8 ? x - box.width : x;
  const top = y + box.height > window.innerHeight - 8 ? y - box.height : y;
  menuBox.style.left = `${Math.max(8, left)}px`;
  menuBox.style.top = `${Math.max(8, top)}px`;
}

function shutMenu() {
  menuBox.hidden = true;
}

window.addEventListener(
  "pointerdown",
  (event) => {
    const where = event.target;
    if (where && where.closest && where.closest(".menu")) return;
    shutMenu();
  },
  { passive: true, capture: true },
);
for (const kind of ["wheel", "blur"]) {
  window.addEventListener(kind, shutMenu, { passive: true, capture: true });
}

document.addEventListener("contextmenu", (event) => {
  const where = event.target;
  if (where && where.closest && where.closest("input, textarea, [contenteditable]")) return;
  event.preventDefault();
});

document.addEventListener("dragstart", (event) => event.preventDefault());
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") shutMenu();
});

const sheet = byId("r-sheet");

function openSheet(play, card) {
  const made = previews.get(play.path);
  const body = byId("r-sheet-body");
  const said = made && made.judged;

  const head = el("div", "sheethead");
  head.append(el("h3", null, said ? said.title : shortFile(play.file)));
  const chips = el("div", "chips");
  chips.append(el("span", "chip who", play.player));
  chips.append(modBadges(play.mods));
  if (said) {
    const out = outcomeOf(said);
    chips.append(el("span", `chip ${out.tone}`, out.text));
  }
  head.append(chips);
  body.replaceChildren(head);

  if (!said) {
    body.append(play.have_map ? reading() : el("p", "fine", "Карты этого реплея нет на этом устройстве."));
    if (play.have_map) waitForRead(play, card);
  } else {
    const top = el("div", "sheettop");
    if (made) {
      const stage = el("div", "sheetstage");
      sheetView = document.createElement("canvas");
      stage.append(sheetView);
      top.append(stage);
    }
    const side = el("div", "sheetside");
    side.append(mainFigure(said, play), tally(said));
    top.append(side);
    body.append(top);
    if (made) {
      sheetPlay = {
        scene: made.scene,
        judged: made.judged,
        head: made.scene.from_ms,
        at: 0,
        show: made.show,
        still: made.still,
        stillImage: made.stillImage,
        path: made.path,
      };
      rollPart(sheetPlay, 0);
    }
    const spread = errorBars(said);
    if (spread) {
      body.append(el("h4", "sheeth", "Куда ложились нажатия"));
      body.append(spread);
    }
  }

  if (said && said.client) {
    body.append(el("h4", "sheeth", "Где записано"));
    const from = el("div", "written");
    const client = el("div", "one");
    client.append(
      el("b", null, said.client.name),
      el("span", null, said.client.build ? `Сборка ${said.client.build} · ${said.client.version}` : `Версия ${said.client.version}`),
    );
    from.append(client);
    if (said.client.played_at) {
      const when = el("div", "one");
      when.append(el("b", null, said.client.played_at.split(" ")[0]), el("span", null, `${said.client.played_at.split(" ")[1]} UTC`));
      from.append(when);
    }
    body.append(from);
  }

  const routes = el("div", "routes");
  const render = el("button", "act small primary", "Отрендерить");
  render.disabled = !play.have_map || drawing || !drawFile;
  render.addEventListener("click", () => {
    shutSheet();
    if (drawFile) drawFile(play);
  });
  routes.append(render);
  const judge = el("button", "act small", "Посмотреть в Судействе");
  judge.addEventListener("click", () => {
    shutSheet();
    takeTo("judge", play.path);
  });
  const studio = el("button", "act small", "Добавить в Студию");
  studio.addEventListener("click", () => {
    shutSheet();
    takeTo("cut", play.path);
  });
  routes.append(judge, studio);
  if (!play.have_map) {
    const grab = el("button", "act small", "Скачать карту");
    grab.addEventListener("click", () => {
      shutSheet();
      showWall(play);
    });
    routes.append(grab);
  }
  const folder = el("button", "act small", "Показать в папке");
  folder.addEventListener("click", () => invoke("reveal_file", { path: play.path }).catch(() => {}));
  routes.append(folder);
  body.append(routes);

  sheet.hidden = false;
  byId("r-sheet-shut").focus();
  if (sheetPlay) runSheetPreview();

  const spread = body.querySelector(".spread .bars");
  if (spread && spread.repaint) requestAnimationFrame(spread.repaint);
  void card;
}

let readingFor = 0;

function waitForRead(play, card) {
  readingFor += 1;
  const mine = readingFor;
  const ready = () => {
    if (mine !== readingFor || sheet.hidden) return;
    const made = previews.get(play.path);
    if (!made || !made.judged) return;
    shutSheet();
    openSheet(play, card);
  };
  if (previews.has(play.path)) {
    setTimeout(ready, 200);
    return;
  }
  fetchPreview(play.path, 720, 540).then(ready);
}

function shortFile(name) {
  const bare = name.replace(/\.osr$/i, "");
  return bare.length > 46 ? `${bare.slice(0, 44)}…` : bare;
}

function reading() {
  const box = el("div", "reading");
  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.setAttribute("viewBox", "0 0 120 120");
  icon.setAttribute("fill", "none");
  icon.setAttribute("aria-hidden", "true");
  icon.setAttribute("class", "ic-reading");
  icon.innerHTML =
    '<circle class="track" cx="60" cy="60" r="42"/>' +
    '<circle class="arc" cx="60" cy="60" r="42"/>' +
    '<path class="pen" d="M40 74 L54 56 L66 66 L82 44"/>' +
    '<circle class="spark" cx="82" cy="44" r="6"/>';
  box.append(icon);
  box.append(el("b", null, "Реплей в процессе анализа…"));
  box.append(el("span", null, "В ближайший момент он станет вот-вот доступен"));
  requestAnimationFrame(restartLoops);
  return box;
}

function mainFigure(said, play) {
  const box = el("div", "crown");

  const big = el("div", "big");
  const whole = Math.floor(said.accuracy_percent);
  const part = Math.round((said.accuracy_percent - whole) * 100);
  const number = el("div", "n");
  number.append(el("span", "w", String(whole)), el("span", "p", `,${String(part).padStart(2, "0")}%`));
  big.append(number, el("span", "cap", "Точность"));
  box.append(big);

  const side = el("div", "aside");
  if (play && play.score) {
    const points = el("div", "one");
    points.append(el("b", null, round(play.score)), el("span", null, "Очки"));
    side.append(points);
  }
  const combo = el("div", "one");
  combo.append(
    el("b", null, `${round(said.combo)}×`),
    el("span", null, said.combo_possible ? `из ${round(said.combo_possible)} возможных` : "Наше комбо"),
  );
  side.append(combo);
  if (said.combo !== said.combo_recorded) {
    const recorded = el("div", "one huh");
    recorded.append(el("b", null, `${round(said.combo_recorded)}×`), el("span", null, "записано в реплее"));
    side.append(recorded);
  }
  const ur = el("div", "one");
  ur.append(
    el("b", null, said.unstable_rate === null ? "—" : round(said.unstable_rate, 1)),
    el("span", null, "Unstable Rate"),
  );
  side.append(ur);
  box.append(side);
  return box;
}

function tally(said) {
  const rows = [
    ["300", said.counts.great, "great"],
    ["100", said.counts.ok, "ok"],
    ["50", said.counts.meh, "meh"],
    ["×", said.counts.miss, "miss"],
  ];
  const total = rows.reduce((sum, [, n]) => sum + n, 0) || 1;
  const box = el("div", "tally");
  const bar = el("div", "band");
  for (const [, n, kind] of rows) {
    if (!n) continue;
    const part = el("span", kind);
    part.style.flexGrow = String(n);
    bar.append(part);
  }
  box.append(bar);
  const keys = el("div", "keys");
  for (const [name, n, kind] of rows) {
    const one = el("div", `key ${kind}${n ? "" : " none"}`);
    one.append(el("i", null, ""), el("b", null, round(n)), el("span", null, name));
    keys.append(one);
  }
  box.append(keys);
  void total;
  return box;
}

function errorBars(said) {
  const every = said.presses && said.presses.length ? said.presses : null;
  const errors = every || said.marks.map((mark) => mark.error_ms).filter((one) => one !== null && one !== undefined);
  if (errors.length < 4) return null;

  const box = el("div", "spread");
  const canvas = document.createElement("canvas");
  canvas.className = "bars";
  const hint = el("div", "hint");
  hint.hidden = true;
  box.append(canvas, hint);

  const widest = Math.max(...errors.map(Math.abs));
  const reach = Math.max(20, Math.ceil(widest / 10) * 10);

  const bins = 33;
  const counts = new Array(bins).fill(0);
  for (const one of errors) {
    const at = Math.min(bins - 1, Math.max(0, Math.floor(((one + reach) / (reach * 2)) * bins)));
    counts[at] += 1;
  }

  const paint = () => paintSpread(canvas, counts);
  requestAnimationFrame(paint);
  canvas.repaint = paint;

  const step = (reach * 2) / bins;
  const edge = (i) => -reach + i * step;
  const ms = (n) => (Math.abs(n) < 0.05 ? "0" : `${n > 0 ? "+" : ""}${round(n, 1)}`);

  canvas.addEventListener("pointermove", (event) => {
    const box_ = canvas.getBoundingClientRect();
    const at = Math.floor(((event.clientX - box_.left) / box_.width) * bins);
    if (at < 0 || at >= bins) {
      hint.hidden = true;
      return;
    }
    const n = counts[at];
    const from = edge(at);
    const to = edge(at + 1);
    const when = at < Math.floor(bins / 2) ? "рано" : at > Math.floor(bins / 2) ? "поздно" : "в точку";
    hint.replaceChildren(
      el("b", null, `${n} ${plural(n, "нажатие", "нажатия", "нажатий")}`),
      el("span", null, `${ms(from)}…${ms(to)} мс · ${when} · ${round((n / errors.length) * 100, 1)}%`),
    );
    hint.hidden = false;
    const wide = hint.offsetWidth || 150;
    const x = ((at + 0.5) / bins) * box_.width;
    hint.style.left = `${Math.min(Math.max(x - wide / 2, 0), box_.width - wide)}px`;
  });
  canvas.addEventListener("pointerleave", () => {
    hint.hidden = true;
  });

  const early = errors.filter((one) => one < 0).length;
  const late = errors.length - early;
  const scale = el("div", "scale");
  scale.append(
    el("span", null, `рано · ${round((early / errors.length) * 100)}%`),
    el("span", "mono", `±${round(reach)} мс`),
    el("span", null, `${round((late / errors.length) * 100)}% · поздно`),
  );
  box.append(scale);
  return box;
}

function paintSpread(canvas, counts) {
  const wide = canvas.clientWidth;
  const high = canvas.clientHeight;
  if (!wide || !high) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = Math.round(wide * dpr);
  canvas.height = Math.round(high * dpr);
  const c = canvas.getContext("2d");
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, wide, high);

  const floor = high - 1;
  const middle = Math.floor(counts.length / 2);
  const step = wide / counts.length;
  const gap = Math.min(2, step * 0.2);
  const tallest = Math.max(...counts) || 1;

  c.fillStyle = "rgba(255,255,255,0.14)";
  c.fillRect(Math.round(wide / 2), 0, 1, floor);
  c.fillRect(0, floor, wide, 1);

  counts.forEach((n, i) => {
    if (!n) return;
    const tall = Math.max(2, (n / tallest) * (floor - 2));
    c.fillStyle = i === middle ? "#5fd694" : i < middle ? "rgba(122,167,216,0.75)" : "rgba(216,136,122,0.75)";
    const w = Math.max(1, step - gap);
    const x = i * step + (step - w) / 2;

    const r = Math.min(2, w / 2, tall / 2);
    c.beginPath();
    c.moveTo(x, floor);
    c.lineTo(x, floor - tall + r);
    c.quadraticCurveTo(x, floor - tall, x + r, floor - tall);
    c.lineTo(x + w - r, floor - tall);
    c.quadraticCurveTo(x + w, floor - tall, x + w, floor - tall + r);
    c.lineTo(x + w, floor);
    c.closePath();
    c.fill();
  });
}

let sheetPlay = null;
let sheetView = null;
let sheetRun = 0;

function runSheetPreview() {
  sheetRun += 1;
  if (!sheetPlay || !sheetView) return;
  const mine = sheetRun;
  const one = sheetPlay;
  const still = !motionOn();
  let was = performance.now();
  (async () => {
    const first = await stillOf(one);
    if (mine === sheetRun && sheetPlay === one && first) {
      const seat = canvasPixels(sheetView, 2);
      if (seat) paintFrame(seat, first);
    }
    while (mine === sheetRun && sheetPlay === one && !sheet.hidden) {
      await new Promise((again) => requestAnimationFrame(again));
      if (mine !== sheetRun || sheetPlay !== one || sheet.hidden) return;
      const c = canvasPixels(sheetView, 2);
      if (!c) return;
      const now = performance.now();
      if (!still) rollPart(one, Math.min(120, now - was));
      was = now;
      const image = await frameOf(one, one.head, c.w, c.h);
      if (mine !== sheetRun || sheetPlay !== one) return;
      if (!image) continue;
      paintFrame(c, image, blendOf(one));
      if (still) return;
    }
  })();
}

function shutSheet() {
  sheet.hidden = true;
  sheetRun += 1;
  sheetPlay = null;
  sheetView = null;
}

byId("r-sheet-shut").addEventListener("click", shutSheet);
sheet.addEventListener("pointerdown", (event) => {
  if (event.target === sheet) shutSheet();
});
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (!wall.hidden) shutWall();
  else if (!sheet.hidden) shutSheet();
});

byId("r-refresh").addEventListener("click", () => showRender(true));

function shelfEdges() {
  const list = byId("r-list");
  const top = list.scrollTop > 4;
  const bottom = list.scrollTop + list.clientHeight < list.scrollHeight - 4;
  list.classList.toggle("over-top", top);
  list.classList.toggle("over-bottom", bottom);
}

byId("r-list").addEventListener("scroll", shelfEdges, { passive: true });
window.addEventListener("resize", shelfEdges);

window.addEventListener(
  "wheel",
  (event) => {
    if (open_tab !== "render" || asleep) return;
    const list = byId("r-list");
    if (list.hidden || list.scrollHeight <= list.clientHeight) return;
    const over = event.target.closest && event.target.closest("#r-list, .options, .sheetbox, .menu, .popover");
    if (over) return;
    list.scrollTop += event.deltaY;
    event.preventDefault();
  },
  { passive: false },
);

function subscribe(name, take) {
  window.__TAURI__.event.listen(name, take).catch((why) => {
    console.error(`Не подписаться на «${name}»:`, why);
    watchFailed = String(why);
  });
}
let watchFailed = null;

if (window.__TAURI__ && window.__TAURI__.event) {
  subscribe("drawing", ({ payload }) => {
    if (payload.event !== "progress" || !payload.of) return;
    const share = Math.min(100, (payload.frames / payload.of) * 100);
    byId(watching.bar).style.width = `${share}%`;
    if (watching.share) byId(watching.share).textContent = round(share);
    const note = `Отрендерено ${round(payload.frames)} кадров из ${round(payload.of)} · ${round(payload.per_second)} в секунду · осталось ${spell(payload.left_seconds)}`;
    byId(watching.said).textContent = note;
    if (job) paintJob(share, note);
  });
  subscribe("fetching", ({ payload }) => saySteps(payload));
  subscribe("reeling", ({ payload }) => {
    const note = `кусок ${payload.clip} из ${payload.of}…`;
    byId("rp-built").textContent = note;
    if (job) paintJob((payload.clip / payload.of) * 100, note);
  });
}

const WORTH = { 300: "#66ccff", 100: "#88d64c", 50: "#f0c060", 0: "#e24848" };

let scene = null;
let live = null;
let viewTicket = 0;
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

async function needs(which) {
  if (which !== "cut") return "";
  const rows = await invoke("ready").catch(() => []);
  const ffmpeg = rows.find((row) => row.name === "ffmpeg");
  return ffmpeg && ffmpeg.ok
    ? ""
    : "Для Студии нужен ffmpeg. «Библиотека» скажет, где его взять";
}

async function enter(which) {
  const gate = byId("rp-gate");
  gate.textContent = "Проверяю, что для этого нужно…";
  const missing = await needs(which);
  if (missing) {
    gate.textContent = missing;
    return false;
  }
  gate.textContent = "";
  mode = which;
  byId("rp-choose").hidden = true;
  byId("rp-stage").hidden = false;
  byId("rp-mode").textContent = which === "judge" ? "Судейство" : "Студия";
  byId("rp-what").textContent =
    which === "judge" ? "Что засчитано, что нет и на сколько" : "Куски игры и сборка из них";
  byId("rp-judge").hidden = which !== "judge";
  byId("rp-cut").hidden = which !== "cut";
  if (scene) showLive();
  return true;
}

for (const button of document.querySelectorAll(".mode")) {
  button.addEventListener("click", () => enter(button.dataset.mode));
}

function saySkinned() {
  const button = byId("rp-skinned");
  button.hidden = !scene;
  button.textContent = skinned ? "Не показывать со скином" : "Показывать со скином";
}

byId("rp-skinned").addEventListener("click", () => {
  skinned = !skinned;
  remember("skinned", skinned ? "1" : "0");
  saySkinned();
  if (!live) return;
  live.skin = skinned ? null : "";
  live.show = 0;
  drawView();
});

byId("rp-back").addEventListener("click", () => {
  stop();
  mode = null;
  byId("rp-stage").hidden = true;
  byId("rp-choose").hidden = false;
});

async function openReplay(path) {
  const said = byId("rp-said");
  byId("rp-drop").hidden = false;
  said.textContent = "Смотрю, есть ли карта…";
  const here = await invoke("map_here", { replay: path }).catch(() => null);
  if (here && !here.here) {
    said.textContent = "";
    showWall({ path, have_map: false });
    return;
  }
  said.textContent = "Читаю и сужу…";
  let opened;
  try {
    opened = await invoke("judged", { replay: path, skin: skinned ? null : "", fine: options().fine });
  } catch (why) {
    said.textContent = `${why}`;
    return;
  }
  scene = opened;
  live = { show: opened.show, path, skin: skinned ? null : "" };
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

function openSettings(which, save = true) {
  const rail = byId("s-rail");
  for (const button of rail.querySelectorAll("button")) {
    const mine = button.dataset.page === which;
    if (mine) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  }
  let found = false;
  for (const page of document.querySelectorAll("#view-settings .page")) {
    const mine = page.dataset.page === which;
    page.hidden = !mine;
    found = found || mine;
  }
  if (!found) return openSettings("link", save);
  if (save) remember("spage", which);
  if (which === "skins") runFitting();
  else stopFitting();
  return which;
}

for (const button of byId("s-rail").querySelectorAll("button")) {
  button.addEventListener("click", () => openSettings(button.dataset.page));
}

function showLive() {
  byId("rp-drop").hidden = true;
  byId("rp-live").hidden = false;
  byId("rp-what").replaceChildren(`${judged.title} · ${judged.player} `, modBadges(judged.mods));
  saySkinned();
  byId("rp-rows").replaceChildren(
    ...[
      { mark: ["ok", "·"], name: "Точность", said: `${round(judged.accuracy_percent, 2)}%` },
      {
        mark: judged.combo === judged.combo_recorded ? ["ok", "·"] : ["huh", "?"],
        name: "Комбо",
        said:
          judged.combo === judged.combo_recorded
            ? String(judged.combo)
            : `${judged.combo} · в реплее записано ${judged.combo_recorded}`,
      },
      { mark: ["ok", "·"], name: "300", said: String(judged.counts.great) },
      { mark: ["ok", "·"], name: "100", said: String(judged.counts.ok) },
      { mark: ["huh", "·"], name: "50", said: String(judged.counts.meh) },
      { mark: judged.counts.miss ? ["no", "!"] : ["ok", "·"], name: "Промахи", said: String(judged.counts.miss) },
      {
        mark: ["ok", "·"],
        name: "Разброс",
        said: judged.unstable_rate === null ? "Без данных" : `${round(judged.unstable_rate, 1)} UR`,
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

let viewBusy = false;
let viewAgain = false;

function drawView() {
  if (!scene || !live) return;
  if (viewBusy) {
    viewAgain = true;
    return;
  }
  const c = canvasPixels(view, 1.5, 1_200_000);
  if (!c) return;
  viewBusy = true;
  viewTicket += 1;
  const mine = viewTicket;
  frameOf(live, head, c.w, c.h).then((image) => {
    viewBusy = false;
    if (mine !== viewTicket) return;
    if (image) paintFrame(c, image);
    else byId("rp-said").textContent = "Движок не отдал кадр — посмотрите в логах";
    if (viewAgain) {
      viewAgain = false;
      drawView();
    }
  });
}

function readAt(ms) {
  return readMarks(judged.marks, ms);
}

function readMarks(marks, ms) {
  let combo = 0;
  let weight = 0;
  let objects = 0;
  for (const mark of marks) {
    if (mark.ms > ms) break;
    combo = mark.combo;
    weight += mark.worth;
    objects += 1;
  }

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

const seek = byId("rp-seek");

let density = null;

function measureDensity() {
  const span = Math.max(1, scene.to_ms - scene.from_ms);
  const buckets = 320;
  const raw = new Float32Array(buckets);
  for (const start of scene.starts) {
    const at = Math.floor(((start - scene.from_ms) / span) * buckets);
    if (at >= 0 && at < buckets) raw[at] += 1;
  }

  density = new Float32Array(buckets);
  let most = 0;
  for (let i = 0; i < buckets; i += 1) {
    const around = (raw[i - 1] || 0) + raw[i] + (raw[i + 1] || 0);
    density[i] = around / 3;
    most = Math.max(most, density[i]);
  }
  if (most > 0) for (let i = 0; i < buckets; i += 1) density[i] /= most;
}

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
    : "Лента пуста";
}

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
    said.textContent = "На ленте нет ни одного куска. Нажмите «Выделить кусок отсюда»";
    return;
  }
  drawing = true;
  watching = { bar: "rp-bar", said: "rp-built", share: null };
  startJob("replay", "Студия");
  said.className = "verdict";
  said.textContent = "Собираю…";
  byId("rp-progress").hidden = false;
  byId("rp-doneslot").replaceChildren();
  byId("rp-bar").style.width = "0%";
  const out = chosen.replace(/\.osr$/i, "") + "-студия.mp4";
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
    said.textContent = "";
    showFinished(byId("rp-doneslot"), {
      head: `Собрано из ${clips.length} ${plural(clips.length, "куска", "кусков", "кусков")}`,
      path: done.path,
      notes: done.said,
    });
    endJob(true);
  } catch (why) {
    said.textContent = "";
    showFinished(byId("rp-doneslot"), { head: "Не собралось", notes: [String(why)], bad: true });
    endJob(false);
  } finally {
    drawing = false;
    byId("rp-progress").hidden = true;
  }
});

function showReplay() {
  if (scene && mode) drawAll();
}

const views = {
  render: showRender,
  replay: showReplay,
  library: showLibrary,
  settings: showSettings,
};

let open_tab = null;

function show(which) {
  if (which === open_tab) return;
  if (open_tab === "settings") stopFitting();
  open_tab = which;
  syncJobMini();
  restartLoops();

  requestAnimationFrame(runPreviews);
  for (const name of Object.keys(views)) {
    byId(`tab-${name}`).setAttribute("aria-selected", String(name === which));
    byId(`view-${name}`).hidden = name !== which;
  }

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

window.addEventListener("resize", () => {
  if (!byId("view-replay").hidden) drawAll();
});

byId("w-save").addEventListener("click", async () => {
  if (!byId("w-server").value.trim() || !byId("w-token").value.trim()) {
    const said = byId("w-said");
    said.className = "verdict bad";
    said.textContent = "Необходим адрес и токен. Без них нельзя стать воркером.";
    return;
  }
  if (await saveSettings("w", "w-said")) {
    byId("wizard").hidden = true;
    show("render");
  }
});

(async function open() {
  motion(remembered("motion", "on"), false);
  sky(remembered("sky", "live"), false);
  splashWhen(remembered("splash", "both"), false);
  skinned = remembered("skinned", "0") === "1";
  idleAfter(remembered("idle", "5"), false);
  reportWhen(remembered("report", "ask"), false);
  openSettings(remembered("spage", "link"), false);
  byId("s-loud").value = remembered("loud", "0.35");
  loudness();
  restartLoops();

  let opening = remembered("splash", "both") !== "never";
  if (opening) showBlackCover();
  requestAnimationFrame(() => dock.classList.remove("landing"));

  const giveUpOnScene = () => {
    if (!opening) return;
    opening = false;
    locked = false;
    asleep = false;
    splash.classList.remove("going");
    splash.hidden = true;
    uncover();
  };

  let first = false;
  let broken = false;
  try {
    first = await invoke("first_run");
  } catch {
  }
  try {
    await loadSettings();
  } catch {
    broken = true;
  }

  if (first || broken) giveUpOnScene();
  if (first) {
    for (const field of FIELDS) {
      const box = byId(`w-${field}`);
      if (box) box.value = known[field] || "";
    }
    byId("wizard").hidden = false;
    armIdle();
    return;
  }
  show("render");

  try {
    dressAll();
    measure();
    if (document.fonts && document.fonts.ready) await document.fonts.ready;
    await loadModArt();
  } catch (why) {
    console.error(why);
  }

  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (opening) playOpening();
  armIdle();

  setTimeout(() => lookForUpdate(false), 3500);
})();
