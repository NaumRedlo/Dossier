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
    list.style.maxWidth = `${Math.min(window.innerWidth - 16, OPTIONS_WIDEST)}px`;
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
      const opened = await invoke("judged", { replay: pick.path });
      idlePlay = { scene: opened, judged: opened.summary, head: opened.from_ms };
      idleFade = 0;
    }
  } catch {
    idleShelf = idleShelf || [];
  } finally {
    idleAsking = false;
  }
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
  let was = performance.now();
  nextIdlePlay();

  const frame = (now) => {
    const { w, h } = sizeSplash(c);
    const step = Math.min(64, now - was);
    was = now;

    if (idlePlay) {
      idlePlay.head += step;
      const left = idlePlay.scene.to_ms - idlePlay.head;
      if (left <= 0) {
        idlePlay = null;
        idleFade = 0;
        nextIdlePlay();
        drawResting(c, w, h);
      } else {
        idleFade = Math.min(1, idleFade + step / FADE_MS);
        c.globalAlpha = Math.min(idleFade, Math.max(0, left / FADE_MS));
        drawPlay(
          c,
          {
            scene: idlePlay.scene,
            judged: idlePlay.judged,
            head: idlePlay.head,
            skinned: true,

            popups: true,
            frame: false,
          },
          w,
          h,
        );
        c.globalAlpha = 1;
      }
    } else {
      drawResting(c, w, h);
    }

    scene_run = requestAnimationFrame(frame);
  };
  scene_run = requestAnimationFrame(frame);
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
  loadPics().catch(() => {});
  playIdle();
}

function hideSplash() {
  if (!asleep || locked) return;
  asleep = false;
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
  loadRenderSettings();
  byId("s-found").textContent = await readShelves();
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

async function freshSkin() {
  pics = null;
  await loadPics();
  previews.clear();
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

let watching = { bar: "r-bar", said: "r-said", share: "r-share" };

let job = null;

let jobFading = false;

const jobBox = byId("jobbox");
const jobMini = byId("job-mini");

const DIAL_ROUND = 2 * Math.PI * 9;

const DONE_HOLD_MS = 12000;
let jobHold = null;

function startJob(home, label) {
  job = { home, label, done: false };
  jobFading = false;
  clearTimeout(jobHold);
  jobMini.classList.remove("leaving", "done", "failed");

  byId("job-label").textContent = label.split(" · ")[0];
  byId("job-full").textContent = label;
  byId("job-hint").textContent = "Нажмите, чтобы вернуться к этой работе.";
  paintJob(0, "Начинаю…");
  syncJobMini();
}

function paintJob(percent, note) {
  const share = Math.min(100, Math.max(0, percent));
  byId("job-percent").textContent = round(share);
  byId("job-dial").style.strokeDashoffset = String(DIAL_ROUND * (1 - share / 100));
  if (note) byId("job-note").textContent = note;
}

function syncJobMini() {
  if (jobFading) return;
  jobBox.hidden = !(job && (job.done || open_tab !== job.home));
}

jobMini.addEventListener("click", () => {
  const home = job && job.home;
  if (job && job.done) dismissJob();
  if (home) show(home);
});

function dismissJob() {
  clearTimeout(jobHold);
  job = null;
  if (jobBox.hidden) return;
  jobFading = true;
  jobMini.classList.add("leaving");
  setTimeout(() => {
    jobBox.hidden = true;
    jobMini.classList.remove("leaving", "done", "failed");
    jobFading = false;
  }, 280);
}

function endJob(ok) {
  if (!job) return;
  paintJob(100, ok ? "Готово" : "Не вышло");
  job.done = true;
  jobFading = false;
  jobMini.classList.remove("leaving");
  jobMini.classList.add(ok ? "done" : "failed");
  byId("job-label").textContent = ok ? "Готово" : "Не вышло";
  byId("job-hint").textContent = ok
    ? "Нажмите, чтобы открыть вкладку с готовым файлом."
    : "Нажмите, чтобы посмотреть, на чём оно встало.";
  jobBox.hidden = false;
  jobHold = setTimeout(dismissJob, DONE_HOLD_MS);
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
  if (path) said.append(el("p", "where", path));
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
    setTimeout(() => slot.replaceChildren(), 300);
  });

  const routes = el("div", "routes");
  if (path && !bad) {
    const open = el("button", "act small primary", "Открыть");
    const where = el("button", "act small", "В папке");
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
    readEffects();
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

  byId("r-count").textContent = plays.length
    ? `${plays.length} ${plural(plays.length, "реплей", "реплея", "реплеев")}, новейшие находятся наверху`
    : "";
  if (!plays.length) {
    list.classList.add("waiting");
    list.replaceChildren(line({ mark: ["huh", "?"], name: "пусто", said: "Укажите папку реплеев." }));
    return;
  }
  list.classList.remove("waiting");
  forgetCards();
  requestAnimationFrame(shelfEdges);
  list.replaceChildren(...plays.map((play) => playCard(play)));

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
    const go = el("button", "act small primary", "Отрендерить");
    go.disabled = !play.have_map;
    go.addEventListener("click", (event) => {
      event.stopPropagation();
      draw(play);
    });
    scrim.append(go);
    stage.append(scrim);
    card.append(stage);

    const said = el("div", "who");
    said.append(el("b", null, play.player));
    said.append(el("p", "map", "…"));
    said.append(el("p", "about", `${play.mods || "NM"} · ${round(play.score)} · комбо ${play.combo}`));
    card.append(said);

    card.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      openMenu(event.clientX, event.clientY, menuFor(play, card));
    });

    card.addEventListener("click", () => openSheet(play, card));

    card.dataset.path = play.path;
    card.previewOf = play;
    card.previewOn = canvas;

    if (play.have_map) watchCard(card);
    else card.classList.add("plain");
    return card;
  }

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
      : `${play.player} · ${howItDraws()}`;
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

      for (const other of list.querySelectorAll("button")) other.disabled = Boolean(other.closest(".rep.nomap"));
      byId("r-busy").hidden = true;
      list.classList.remove("dimmed");
    }
  }

  function menuFor(play, card) {
    return [
      { name: "Отрендерить", off: !play.have_map || drawing, go: () => draw(play) },
      { name: "Посмотреть в Судействе", off: !play.have_map, go: () => takeTo("judge", play.path) },
      { name: "Добавить в Студию", off: !play.have_map, go: () => takeTo("cut", play.path) },
      { name: "Подробнее…", off: !play.have_map, go: () => openSheet(play, card) },
      { line: true },
      { name: "Показать в папке", go: () => invoke("reveal_file", { path: play.path }).catch(() => {}) },
    ];
  }
}

const PREVIEWS_KEPT = 60;

const PREVIEWS_AT_ONCE = 2;

const PREVIEW_EDGE_MS = 420;

const previews = new Map();
const showing = new Set();
const asking = new Set();
const waitingFor = [];

let hovered = null;
let previewRun = null;
let previewSeen = null;

function forgetCards() {
  if (previewSeen) previewSeen.disconnect();
  showing.clear();
  waitingFor.length = 0;
  hovered = null;
  if (previewRun) cancelAnimationFrame(previewRun);
  previewRun = null;
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
    if (previewRun) cancelAnimationFrame(previewRun);
    previewRun = null;

    const made = previews.get(card.previewOf.path);
    if (made) made.head = made.scene.from_ms;
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

function pumpPreviews() {
  while (asking.size < PREVIEWS_AT_ONCE && waitingFor.length) {
    const card = waitingFor.shift();

    if (!showing.has(card)) continue;
    const path = card.previewOf.path;
    if (previews.has(path)) continue;
    asking.add(path);
    invoke("preview", { replay: path })
      .then((scene) => {
        const made = { scene, judged: scene.summary, head: scene.from_ms };
        keepPreview(path, made);
        applyPreview(card, made);
      })
      .catch(() => {
        keepPreview(path, null);
        applyPreview(card, null);
      })
      .finally(() => {
        asking.delete(path);
        pumpPreviews();
      });
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

function stillCard(card) {
  const made = previews.get(card.previewOf.path);
  if (!made) return;
  const c = readyCanvas(card);
  if (!c) return;
  const { ctx, wide, high } = c;
  const play = made.scene;
  const middle = (play.from_ms + play.to_ms) / 2;
  drawPlay(ctx, { scene: play, judged: made.judged, head: middle, skinned: true, popups: true, frame: false }, wide, high);
}

function readyCanvas(card) {
  const canvas = card.previewOn;
  const wide = canvas.clientWidth;
  const high = canvas.clientHeight;
  if (!wide || !high) return null;

  const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
  if (canvas.width !== Math.round(wide * dpr)) {
    canvas.width = Math.round(wide * dpr);
    canvas.height = Math.round(high * dpr);
  }
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, wide, high);
  return { ctx, wide, high };
}

function runPreviews() {
  const one = hovered;
  const wanted = one && motionOn() && !document.hidden && !asleep && previews.get(one.previewOf.path);
  if (!wanted) {
    if (previewRun) cancelAnimationFrame(previewRun);
    previewRun = null;
    return;
  }
  if (previewRun) return;
  let was = performance.now();
  const frame = (now) => {
    if (hovered !== one) {
      previewRun = null;
      return;
    }

    const step = Math.min(120, now - was);
    was = now;
    paintCard(one, step);
    previewRun = requestAnimationFrame(frame);
  };
  previewRun = requestAnimationFrame(frame);
}

function paintCard(card, step) {
  const made = previews.get(card.previewOf.path);
  if (!made) return;
  const c = readyCanvas(card);
  if (!c) return;
  const play = made.scene;
  made.head += step;
  if (made.head > play.to_ms) made.head = play.from_ms;

  drawPlay(
    c.ctx,
    { scene: play, judged: made.judged, head: made.head, skinned: true, popups: true, frame: false },
    c.wide,
    c.high,
  );
  const into = clamp01((made.head - play.from_ms) / PREVIEW_EDGE_MS);
  const away = clamp01((play.to_ms - made.head) / PREVIEW_EDGE_MS);
  const shown = Math.min(into, away);
  if (shown >= 1) return;

  c.ctx.save();
  c.ctx.globalCompositeOperation = "destination-out";
  c.ctx.globalAlpha = 1 - shown;
  c.ctx.fillRect(0, 0, c.wide, c.high);
  c.ctx.restore();
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

for (const kind of ["pointerdown", "wheel", "blur"]) {
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
  head.append(el("h3", null, said ? said.title : play.file));
  const chips = el("div", "chips");
  chips.append(el("span", "chip who", play.player));
  chips.append(el("span", "chip", play.mods || "NM"));
  chips.append(el("span", "chip", `${round(play.score)} очк.`));
  if (said) {
    const out = outcomeOf(said);
    chips.append(el("span", `chip ${out.tone}`, out.text));
  }
  head.append(chips);
  body.replaceChildren(head);

  if (made) {
    const stage = el("div", "sheetstage");
    sheetView = document.createElement("canvas");
    stage.append(sheetView);
    body.append(stage);
    sheetPlay = { scene: made.scene, judged: made.judged, head: made.scene.from_ms };
    runSheetPreview();
  }

  if (!said) {
    body.append(
      el("p", "fine", play.have_map ? "Ещё читаю этот реплей." : "Карты этого реплея нет на этом устройстве."),
    );
  } else {
    body.append(mainFigure(said));
    body.append(tally(said));
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

  body.append(el("h4", "sheeth", "Файл"));
  body.append(el("p", "where", play.path));

  const routes = el("div", "routes");
  const judge = el("button", "act small primary", "Посмотреть в Судействе");
  judge.disabled = !play.have_map;
  judge.addEventListener("click", () => {
    shutSheet();
    takeTo("judge", play.path);
  });
  const studio = el("button", "act small", "Добавить в Студию");
  studio.disabled = !play.have_map;
  studio.addEventListener("click", () => {
    shutSheet();
    takeTo("cut", play.path);
  });
  const folder = el("button", "act small", "Показать в папке");
  folder.addEventListener("click", () => invoke("reveal_file", { path: play.path }).catch(() => {}));
  routes.append(judge, studio, folder);
  body.append(routes);

  sheet.hidden = false;
  byId("r-sheet-shut").focus();

  const spread = body.querySelector(".spread .bars");
  if (spread && spread.repaint) requestAnimationFrame(spread.repaint);
  void card;
}

function mainFigure(said) {
  const box = el("div", "crown");

  const big = el("div", "big");
  const whole = Math.floor(said.accuracy_percent);
  const part = Math.round((said.accuracy_percent - whole) * 100);
  const number = el("div", "n");
  number.append(el("span", "w", String(whole)), el("span", "p", `,${String(part).padStart(2, "0")}%`));
  big.append(number, el("span", "cap", "Точность"));
  box.append(big);

  const side = el("div", "aside");
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
  const errors = said.marks.map((mark) => mark.error_ms).filter((one) => one !== null && one !== undefined);
  if (errors.length < 4) return null;

  const box = el("div", "spread");
  const canvas = document.createElement("canvas");
  canvas.className = "bars";
  box.append(canvas);

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
    const x = i * step;
    const w = Math.max(1, step - gap);

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
let sheetRun = null;

function runSheetPreview() {
  if (sheetRun) cancelAnimationFrame(sheetRun);
  if (!sheetPlay || !sheetView) return;
  if (!motionOn()) {
    paintSheet(0);
    return;
  }
  let was = performance.now();
  const frame = (now) => {
    if (!sheetPlay || sheet.hidden) {
      sheetRun = null;
      return;
    }
    const step = Math.min(120, now - was);
    was = now;
    paintSheet(step);
    sheetRun = requestAnimationFrame(frame);
  };
  sheetRun = requestAnimationFrame(frame);
}

function paintSheet(step) {
  const wide = sheetView.clientWidth;
  const high = sheetView.clientHeight;
  if (!wide || !high) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  if (sheetView.width !== Math.round(wide * dpr)) {
    sheetView.width = Math.round(wide * dpr);
    sheetView.height = Math.round(high * dpr);
  }
  const c = sheetView.getContext("2d");
  c.setTransform(dpr, 0, 0, dpr, 0, 0);
  c.clearRect(0, 0, wide, high);
  const play = sheetPlay.scene;
  sheetPlay.head += step;
  if (sheetPlay.head > play.to_ms) sheetPlay.head = play.from_ms;
  drawPlay(
    c,
    { scene: play, judged: sheetPlay.judged, head: sheetPlay.head, skinned: true, popups: true, frame: false },
    wide,
    high,
  );
  const into = clamp01((sheetPlay.head - play.from_ms) / PREVIEW_EDGE_MS);
  const away = clamp01((play.to_ms - sheetPlay.head) / PREVIEW_EDGE_MS);
  const shown = Math.min(into, away);
  if (shown >= 1) return;
  c.save();
  c.globalCompositeOperation = "destination-out";
  c.globalAlpha = 1 - shown;
  c.fillRect(0, 0, wide, high);
  c.restore();
}

function shutSheet() {
  sheet.hidden = true;
  if (sheetRun) cancelAnimationFrame(sheetRun);
  sheetRun = null;
  sheetPlay = null;
  sheetView = null;
}

byId("r-sheet-shut").addEventListener("click", shutSheet);
sheet.addEventListener("pointerdown", (event) => {
  if (event.target === sheet) shutSheet();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !sheet.hidden) shutSheet();
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
    const note = `Отрендерено ${round(payload.frames)} кадров из ${round(payload.of)} · ${round(payload.per_second)} в секунду · осталось ${round(payload.left_seconds)} с`;
    byId(watching.said).textContent = note;
    if (job) paintJob(share, note);
  });
  subscribe("reeling", ({ payload }) => {
    const note = `кусок ${payload.clip} из ${payload.of}…`;
    byId("rp-built").textContent = note;
    if (job) paintJob((payload.clip / payload.of) * 100, note);
  });
}

const WORTH = { 300: "#66ccff", 100: "#88d64c", 50: "#f0c060", 0: "#e24848" };

const PLAIN = "#c9cede";

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

  c.globalCompositeOperation = "destination-in";
  c.drawImage(image, 0, 0);
  per.set(colour, made);
  return made;
}
const SAID = { 300: "300", 100: "100", 50: "50", 0: "×" };

const AFTER_MS = 1400;

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

async function openReplay(path) {
  const said = byId("rp-said");
  byId("rp-drop").hidden = false;
  said.textContent = "Читаю и сужу…";
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
  return which;
}

for (const button of byId("s-rail").querySelectorAll("button")) {
  button.addEventListener("click", () => openSettings(button.dataset.page));
}

function showLive() {
  byId("rp-drop").hidden = true;
  byId("rp-live").hidden = false;
  byId("rp-what").textContent = `${judged.title} · ${judged.player} · ${judged.mods || "NM"}`;
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

function fit(w, h, radius) {
  const scale = Math.min(w / 512, h / 384) * 0.9;
  return {
    scale,
    ox: (w - 512 * scale) / 2,
    oy: (h - 384 * scale) / 2,
    r: radius * scale,
  };
}

function firstVisible(objects, ms) {
  let low = 0;
  let high = objects.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (objects[mid].end_ms + AFTER_MS < ms) low = mid + 1;
    else high = mid;
  }
  return low;
}

function cursorAt(play, ms) {
  const step = Math.round((ms - play.from_ms) / play.step_ms);
  const at = Math.min(Math.max(0, step), play.keys.length - 1);
  return { x: play.cursor[at * 2], y: play.cursor[at * 2 + 1], keys: play.keys[at], at };
}

const HIT_FADE_MS = 240;
const MISS_FADE_MS = 100;
const NUMBER_FADE_MS = HIT_FADE_MS / 4;
const HIT_SWELL = 0.4;
const APPROACH_REACH = 3.0;
const BALL_CORE = 0.34;
const ARROW_SCALE = 0.52;
const ARROW_LOOP_MS = 300;
const ARROW_LOOP_FROM = 1.3;

const TICK_FADE_MS = 150;

const TICK_FIRST_LEAD = 0.66;
const TICK_REPEAT_LEAD_MS = 200;
const LIGHT_IN_MS = 200;
const LIGHT_HOLD_MS = 400;
const LIGHT_OUT_MS = 1000;
const LIGHT_MS = LIGHT_HOLD_MS + LIGHT_OUT_MS;
const LIGHT_GROW_MS = 600;
const LIGHT_FROM = 0.8;
const LIGHT_TO = 1.2;
const FOLLOW_SPACING = 32;
const FOLLOW_PREEMPT_MS = 800;
const FOLLOW_ENTRY_SCALE = 1.5;
const FOLLOW_APPROACH = 0.1;

const TRAIL_STEP_MS = 1000 / 60;
const TRAIL_DISJOINT_MS = 150;
const TRAIL_CONTINUOUS_MS = 500;
const TRAIL_INTERVAL_SHARE = 1 / 2.5;

const NOTE_BORDER = 0.11;

const CURSOR_TURN_MS = 10000;

const easeOut = (t) => 1 - (1 - t) * (1 - t);
const clamp01 = (t) => (t < 0 ? 0 : t > 1 ? 1 : t);

const effects = { lighting: true, expand: true };

function readEffects() {
  effects.lighting = remembered("lighting", "1") === "1";
  effects.expand = remembered("expand", "1") === "1";
}
readEffects();

function colourOf(piece, show) {
  if (!show.skinned) return PLAIN;
  const own = pics && pics.colours && pics.colours.length ? pics.colours : null;
  const list = own || (show.scene.colours && show.scene.colours.length ? show.scene.colours : null);
  if (!list || !list.length) return "#e24848";
  return list[piece.run % list.length];
}

function shade(colour, part) {
  const hex = colour.replace("#", "");
  const n = parseInt(hex.length === 3 ? hex.replace(/./g, (d) => d + d) : hex, 16);
  const at = (shift) => Math.round(((n >> shift) & 255) * part);
  return `rgb(${at(16)}, ${at(8)}, ${at(0)})`;
}

function verdictsOf(show) {
  if (show.scene.by_object) return show.scene.by_object;
  const by = new Map();
  for (const mark of show.judged.marks) by.set(mark.object_index, mark);
  show.scene.by_object = by;
  return by;
}

function endingOf(entry) {
  const { piece, mark } = entry;
  const missed = mark ? mark.worth === 0 : false;
  const resolved = mark ? mark.ms : piece.end_ms;
  return {
    missed,
    resolved,
    fade: missed ? MISS_FADE_MS : HIT_FADE_MS,
    leaves: Math.max(resolved, piece.end_ms),
  };
}

function alphaOf(entry, show) {
  const play = show.scene;
  const spawn = entry.piece.start_ms - play.preempt_ms;
  const ends = endingOf(entry);
  const now = show.head;
  if (now < spawn || now > ends.leaves + ends.fade) return 0;
  const appearing = clamp01((now - spawn) / Math.max(1, play.fade_in_ms));

  return appearing * (1 - clamp01((now - ends.leaves) / ends.fade));
}

function drawPlay(c, show, w, h) {
  const box = fit(w, h, show.scene.radius);
  const px = (x) => box.ox + x * box.scale;
  const py = (y) => box.oy + y * box.scale;
  const marks = verdictsOf(show);

  if (show.frame) {
    c.strokeStyle = "rgba(255,255,255,0.05)";
    c.strokeRect(box.ox, box.oy, 512 * box.scale, 384 * box.scale);
  }

  const showing = [];
  for (let i = firstVisible(show.scene.objects, show.head); i < show.scene.objects.length; i += 1) {
    const piece = show.scene.objects[i];
    if (piece.start_ms - show.scene.preempt_ms > show.head) break;
    showing.push({ piece, index: i, mark: marks.get(i) || null });
  }

  drawFollowPoints(c, box, px, py, show);
  drawLighting(c, box, px, py, show, showing);
  for (let i = showing.length - 1; i >= 0; i -= 1) drawBody(c, showing[i], box, px, py, show);
  for (let i = showing.length - 1; i >= 0; i -= 1) drawNote(c, showing[i], box, px, py, show);
  for (let i = showing.length - 1; i >= 0; i -= 1) drawApproach(c, showing[i], box, px, py, show);

  if (show.popups) drawPopups(c, box, px, py, show);
  drawCursor(c, box, px, py, show);
}

function drawFollowPoints(c, box, px, py, show) {
  const shot = show.skinned && pics ? pics.follow_point : null;
  if (!shot) return;
  const play = show.scene;
  const now = show.head;
  const fade = Math.max(1, play.fade_in_ms);
  const objects = play.objects;

  for (let i = Math.max(1, firstVisible(objects, now)); i < objects.length; i += 1) {
    const to = objects[i];
    if (to.start_ms - play.preempt_ms - FOLLOW_PREEMPT_MS > now) break;
    const from = objects[i - 1];
    if (to.combo === 1 || from.kind === "spinner" || to.kind === "spinner") continue;
    const leaves = endPointOf(from);
    const span = to.start_ms - from.end_ms;
    if (span <= 0) continue;
    const dx = to.x - leaves[0];
    const dy = to.y - leaves[1];
    const distance = Math.hypot(dx, dy);
    if (distance <= FOLLOW_SPACING * 2.5) continue;
    const turn = Math.atan2(dy, dx);

    for (let walked = FOLLOW_SPACING * 1.5; walked < distance - FOLLOW_SPACING; walked += FOLLOW_SPACING) {
      const fraction = walked / distance;
      const leaves_at = from.end_ms + fraction * span;
      const arrives_at = leaves_at - FOLLOW_PREEMPT_MS;
      if (now < arrives_at) continue;
      const arriving = clamp01((now - arrives_at) / fade);
      const leaving = now > leaves_at ? clamp01((now - leaves_at) / fade) : 0;
      const alpha = arriving * (1 - leaving);
      if (alpha <= 0) continue;

      const along = fraction - FOLLOW_APPROACH * (1 - easeOut(arriving));
      const scale = FOLLOW_ENTRY_SCALE + (1 - FOLLOW_ENTRY_SCALE) * easeOut(arriving);
      const side = box.r * scale;
      c.save();
      c.globalAlpha = alpha;
      c.translate(px(leaves[0] + dx * along), py(leaves[1] + dy * along));
      c.rotate(turn);
      c.drawImage(shot.image, -side / 2, -side / 2, side, side);
      c.restore();
    }
  }
  c.globalAlpha = 1;
}

function drawLighting(c, box, px, py, show, showing) {
  const shot = effects.lighting && show.skinned && pics ? pics.lighting : null;
  if (!shot) return;
  c.save();
  c.globalCompositeOperation = "lighter";
  for (const entry of showing) {
    const mark = entry.mark;
    if (!mark || mark.worth === 0) continue;
    const age = show.head - mark.ms;
    if (age < 0 || age >= LIGHT_MS) continue;
    const alpha =
      age < LIGHT_IN_MS
        ? age / LIGHT_IN_MS
        : age < LIGHT_HOLD_MS
          ? 1
          : clamp01(1 - (age - LIGHT_HOLD_MS) / LIGHT_OUT_MS);
    if (alpha <= 0) continue;
    const scale = LIGHT_FROM + (LIGHT_TO - LIGHT_FROM) * easeOut(clamp01(age / LIGHT_GROW_MS));
    const side = box.r * 2 * scale;
    c.globalAlpha = alpha;

    c.drawImage(tinted(shot.image, colourOf(entry.piece, show)), px(mark.x) - side / 2, py(mark.y) - side / 2, side, side);
  }
  c.restore();
  c.globalAlpha = 1;
}

const TUBE_SHADOW = 1 - 59 / 64;
const TUBE_BORDER = 0.1875;
const TUBE_SHADOW_ALPHA = 0.25;
const TUBE_ALPHA = 0.7;

let tubeCanvas = null;

function drawBody(c, entry, box, px, py, show) {
  const piece = entry.piece;
  if (piece.kind !== "slider" || piece.path.length < 4) return;
  const alpha = alphaOf(entry, show);
  if (alpha <= 0 || box.r < 0.5) return;
  const rules = show.skinned && pics ? pics.rules : null;
  const colour = colourOf(piece, show);
  const track = (rules && rules.slider_track) || colour;
  const border = (rules && rules.slider_border) || "#ffffff";
  const outer = scaled(track, 1 / 1.1);
  const inner = lifted(track, 1.125, 0.25);

  if (!tubeCanvas) tubeCanvas = document.createElement("canvas");
  const wide = c.canvas.width;
  const high = c.canvas.height;
  if (tubeCanvas.width !== wide || tubeCanvas.height !== high) {
    tubeCanvas.width = wide;
    tubeCanvas.height = high;
  }
  const t = tubeCanvas.getContext("2d");
  t.setTransform(c.getTransform());
  t.clearRect(0, 0, wide, high);
  t.lineCap = "round";
  t.lineJoin = "round";

  const line = () => {
    t.beginPath();
    t.moveTo(px(piece.path[0]), py(piece.path[1]));
    for (let i = 2; i < piece.path.length; i += 2) t.lineTo(px(piece.path[i]), py(piece.path[i + 1]));
  };

  const steps = Math.min(48, Math.max(8, Math.ceil(box.r / 2)));
  for (let step = steps; step >= 0; step -= 1) {
    const towards = 1 - step / steps;
    const width = Math.max(0.01, box.r * 2 * (1 - towards));
    let paint;
    if (towards <= TUBE_SHADOW) {
      paint = `rgba(0,0,0,${(TUBE_SHADOW_ALPHA * towards) / TUBE_SHADOW})`;
    } else if (towards <= TUBE_BORDER) {
      paint = border;
    } else {
      const along = (towards - TUBE_BORDER) / (1 - TUBE_BORDER);
      paint = mixed(outer, inner, along, TUBE_ALPHA);
    }

    t.globalCompositeOperation = "destination-out";
    t.lineWidth = width;
    line();
    t.stroke();
    t.globalCompositeOperation = "source-over";
    t.strokeStyle = paint;
    line();
    t.stroke();
  }

  c.save();
  c.setTransform(1, 0, 0, 1, 0, 0);
  c.globalAlpha = alpha;
  c.drawImage(tubeCanvas, 0, 0);
  c.restore();
}

function parts(colour) {
  if (colour.startsWith("#")) {
    const hex = colour.slice(1);
    const n = parseInt(hex.length === 3 ? hex.replace(/./g, (d) => d + d) : hex, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const found = colour.match(/\d+(\.\d+)?/g) || [0, 0, 0];
  return [Number(found[0]), Number(found[1]), Number(found[2])];
}

function scaled(colour, by) {
  const [r, g, b] = parts(colour);
  return `rgb(${Math.round(r * by)}, ${Math.round(g * by)}, ${Math.round(b * by)})`;
}

function lifted(colour, by, add) {
  const [r, g, b] = parts(colour);
  const up = (one) => Math.round(Math.min(255, one * by + add * 255));
  return `rgb(${up(r)}, ${up(g)}, ${up(b)})`;
}

function mixed(from, to, along, alpha) {
  const a = parts(from);
  const b = parts(to);
  const at = (i) => Math.round(a[i] + (b[i] - a[i]) * along);
  return `rgba(${at(0)}, ${at(1)}, ${at(2)}, ${alpha})`;
}

function endPointOf(piece) {
  if (piece.kind === "slider" && piece.ball.length >= 2) {
    const last = piece.ball.length;
    return [piece.ball[last - 2], piece.ball[last - 1]];
  }
  return [piece.x, piece.y];
}

function farEndOf(piece) {
  const last = piece.path.length;
  return last >= 2 ? [piece.path[last - 2], piece.path[last - 1]] : [piece.x, piece.y];
}

function drawNote(c, entry, box, px, py, show) {
  const { piece } = entry;
  const play = show.scene;
  const now = show.head;
  const alpha = alphaOf(entry, show);
  if (alpha <= 0) return;
  const colour = colourOf(piece, show);

  if (piece.kind === "spinner") {
    c.globalAlpha = alpha;
    c.strokeStyle = colour;
    c.lineWidth = 2;
    const turning = clamp01((now - piece.start_ms) / Math.max(1, piece.end_ms - piece.start_ms));
    c.beginPath();
    c.arc(px(256), py(192), 130 * box.scale * (1 - turning * 0.75), 0, Math.PI * 2);
    c.stroke();
    c.globalAlpha = 1;
    return;
  }

  const ends = endingOf(entry);
  const exit = clamp01((now - ends.leaves) / ends.fade);

  const grown = box.r * (ends.missed ? 1 : 1 + HIT_SWELL * easeOut(exit));

  c.globalAlpha = alpha;
  if (piece.kind === "slider") {
    const tail = farEndOf(piece);
    drawFace(c, px(tail[0]), py(tail[1]), grown, colour, show, "tail");
  }
  drawFace(c, px(piece.x), py(piece.y), grown, colour, show, piece.kind === "slider" ? "head" : "note");

  const swells = !!(show.skinned && pics && pics.rules && pics.rules.number_swells);
  const numberShare = ends.missed || now < ends.resolved ? 1 : clamp01(1 - (now - ends.resolved) / NUMBER_FADE_MS);
  if (numberShare > 0) {
    c.globalAlpha = alpha * (swells ? 1 : numberShare);
    drawNumber(c, px(piece.x), py(piece.y), swells ? grown : box.r, piece.combo, colour, show);
    c.globalAlpha = alpha;
  }
  drawRim(c, px(piece.x), py(piece.y), grown, show, piece.kind === "slider" ? "head" : "note");

  if (piece.kind === "slider") drawSlide(c, entry, box, px, py, show, colour, alpha);
  c.globalAlpha = 1;
}

function faceOf(which) {
  if (!pics) return null;
  if (which === "head" && pics.slider_head) return [pics.slider_head, pics.slider_head_overlay];
  if (which === "tail" && pics.slider_tail) return [pics.slider_tail, pics.slider_tail_overlay];
  if (which === "tail") return null;
  return pics.circle ? [pics.circle, pics.overlay] : null;
}

function drawFace(c, x, y, radius, colour, show, which) {
  const pair = show.skinned ? faceOf(which) : null;
  if (pair) {
    const side = radius * 2;
    c.drawImage(tinted(pair[0].image, colour), x - radius, y - radius, side, side);

    const above = pics.rules && pics.rules.overlay_above_number;
    if (!above && pair[1]) c.drawImage(pair[1].image, x - radius, y - radius, side, side);
    return;
  }
  if (which === "tail") return;
  if (show.skinned) {
    const border = radius * NOTE_BORDER;
    c.fillStyle = shade(colour, 0.75);
    c.beginPath();
    c.arc(x, y, radius, 0, Math.PI * 2);
    c.fill();
    c.fillStyle = colour;
    c.beginPath();
    c.arc(x, y, radius - border, 0, Math.PI * 2);
    c.fill();
    c.strokeStyle = "#fff";
    c.lineWidth = border;
    c.beginPath();
    c.arc(x, y, radius - border / 2, 0, Math.PI * 2);
    c.stroke();
    return;
  }

  c.fillStyle = "rgba(255,255,255,0.05)";
  c.beginPath();
  c.arc(x, y, radius, 0, Math.PI * 2);
  c.fill();
  c.strokeStyle = colour;
  c.lineWidth = Math.max(1.5, radius * 0.1);
  c.stroke();
}

function drawRim(c, x, y, radius, show, which) {
  if (!show.skinned || !pics || !pics.rules || !pics.rules.overlay_above_number) return;
  const pair = faceOf(which);
  if (!pair || !pair[1]) return;
  c.drawImage(pair[1].image, x - radius, y - radius, radius * 2, radius * 2);
}

function drawNumber(c, x, y, radius, combo, colour, show) {
  const figures = String(combo).split("");
  if (show.skinned && pics && pics.digits.length === 10) {
    const high = radius * 0.9;
    const overlap = pics.rules ? pics.rules.hit_circle_overlap : 0;
    const wide = figures.map((d) => {
      const one = pics.digits[Number(d)];
      return one ? (one.image.width / one.image.height) * high : 0;
    });

    const pull = pics.digits[0] ? (overlap / pics.digits[0].image.height) * high : 0;
    const total = wide.reduce((a, b) => a + b, 0) - pull * (figures.length - 1);
    let at = x - total / 2;
    figures.forEach((d, i) => {
      const one = pics.digits[Number(d)];
      if (one) c.drawImage(one.image, at, y - high / 2, wide[i], high);
      at += wide[i] - pull;
    });
    return;
  }
  if (radius <= 9) return;
  c.fillStyle = show.skinned ? "#fff" : colour;
  c.font = `600 ${radius * 0.9}px ui-monospace, Menlo, monospace`;
  c.textAlign = "center";
  c.textBaseline = "middle";
  c.fillText(String(combo), x, y);
}

function drawSlide(c, entry, box, px, py, show, colour, alpha) {
  const { piece } = entry;
  const play = show.scene;
  const now = show.head;
  const slides = piece.slides || 1;
  const span = Math.max(1, piece.end_ms - piece.start_ms);
  const slide = span / slides;

  const ticks = piece.ticks || [];
  for (let i = 0; i + 2 < ticks.length; i += 3) {
    const at = ticks[i];
    if (at <= now) continue;
    const step = slide > 0 ? Math.floor((at - piece.start_ms) / slide) : 0;
    const lead = step > 0 ? TICK_REPEAT_LEAD_MS : play.preempt_ms * TICK_FIRST_LEAD;
    const live = at - ((at - (piece.start_ms + step * slide)) / 2 + lead);
    const arriving = clamp01((now - live) / TICK_FADE_MS);
    if (arriving <= 0) continue;
    const grown = 0.5 + 0.5 * easeOut(clamp01((now - live) / (TICK_FADE_MS * 4)));
    const x = px(ticks[i + 1]);
    const y = py(ticks[i + 2]);
    c.globalAlpha = alpha * arriving;
    const dot = show.skinned && pics ? pics.score_point : null;
    if (dot) {
      const side = box.r * 2 * grown * 0.4;
      c.drawImage(dot.image, x - side / 2, y - side / 2, side, side);
    } else {
      c.fillStyle = "rgba(255,255,255,0.85)";
      c.beginPath();
      c.arc(x, y, Math.max(1.5, box.r * 0.13 * grown), 0, Math.PI * 2);
      c.fill();
    }
  }
  c.globalAlpha = alpha;

  if (slides > 1 && now < piece.end_ms) {
    const at = Math.floor(Math.max(0, now - piece.start_ms) / slide);
    if (at < slides - 1) {
      const near = at % 2 === 1;
      const spot = near ? [piece.x, piece.y] : farEndOf(piece);
      const other = near ? farEndOf(piece) : [piece.x, piece.y];
      const turn = Math.atan2(other[1] - spot[1], other[0] - spot[0]);

      const breath = ARROW_LOOP_FROM + (1 - ARROW_LOOP_FROM) * easeOut(((now % ARROW_LOOP_MS) / ARROW_LOOP_MS));

      const side = box.r * 2 * ARROW_SCALE * breath;
      const shot = show.skinned && pics ? pics.reverse_arrow : null;
      c.save();
      c.globalAlpha = alpha;
      c.translate(px(spot[0]), py(spot[1]));
      c.rotate(turn);
      if (shot) {
        c.drawImage(shot.image, -side / 2, -side / 2, side, side);
      } else {
        c.strokeStyle = "#fff";
        c.lineWidth = Math.max(1.5, box.r * 0.14);
        c.lineCap = "round";
        c.lineJoin = "round";
        const reach = box.r * 0.5 * breath;
        c.beginPath();
        c.moveTo(-reach, -reach);
        c.lineTo(reach * 0.6, 0);
        c.lineTo(-reach, reach);
        c.stroke();
      }
      c.restore();
    }
  }

  if (now < piece.start_ms || now > piece.end_ms || !piece.ball.length) return;
  const at = Math.min(piece.ball.length / 2 - 1, Math.max(0, Math.round((now - piece.start_ms) / play.step_ms)));
  const bx = px(piece.ball[at * 2]);
  const by = py(piece.ball[at * 2 + 1]);

  c.globalAlpha = 1;
  const ring = show.skinned && pics ? pics.follow_circle : null;
  if (ring) {
    const side = box.r * 2 * 2;
    c.drawImage(ring.image, bx - side / 2, by - side / 2, side, side);
  } else {
    c.strokeStyle = "rgba(255,255,255,0.5)";
    c.lineWidth = Math.max(1.5, box.r * 0.1);
    c.beginPath();
    c.arc(bx, by, box.r * 1.9, 0, Math.PI * 2);
    c.stroke();
  }

  const ball = show.skinned && pics ? pics.slider_ball : null;
  if (ball) {
    const side = box.r * 2;

    const tint = pics.rules && pics.rules.slider_ball_tint ? tinted(ball.image, colour) : ball.image;
    c.drawImage(tint, bx - side / 2, by - side / 2, side, side);
    return;
  }

  const grown = BALL_CORE + (1 - BALL_CORE) * clamp01((now - piece.start_ms) / span);
  c.fillStyle = colour;
  c.beginPath();
  c.arc(bx, by, box.r * 0.9 * grown, 0, Math.PI * 2);
  c.fill();
  c.strokeStyle = "rgba(255,255,255,0.9)";
  c.lineWidth = 2;
  c.stroke();
}

function drawApproach(c, entry, box, px, py, show) {
  const { piece } = entry;
  if (piece.kind === "spinner" || show.head >= piece.start_ms) return;
  const alpha = alphaOf(entry, show);
  if (alpha <= 0) return;
  const play = show.scene;
  const progress = clamp01(1 - (piece.start_ms - show.head) / Math.max(1, play.preempt_ms));
  const scale = 1 + APPROACH_REACH * (1 - progress);
  const colour = colourOf(piece, show);
  c.globalAlpha = alpha;
  const shot = show.skinned && pics ? pics.approach : null;
  if (shot) {
    const side = box.r * scale * 2;
    c.drawImage(tinted(shot.image, colour), px(piece.x) - side / 2, py(piece.y) - side / 2, side, side);
  } else {
    c.strokeStyle = colour;
    c.lineWidth = Math.max(1, box.r * 0.09);
    c.beginPath();
    c.arc(px(piece.x), py(piece.y), box.r * scale, 0, Math.PI * 2);
    c.stroke();
  }
  c.globalAlpha = 1;
}

const VERDICT_OF = { 300: "three", 100: "hundred", 50: "fifty", 0: "miss" };

function drawPopups(c, box, px, py, show) {
  c.textAlign = "center";
  c.textBaseline = "middle";
  for (const mark of show.judged.marks) {
    const since = show.head - mark.ms;
    if (since < 0 || since > 600) continue;
    c.globalAlpha = 1 - since / 600;
    const shot = show.skinned && pics && pics.verdicts ? pics.verdicts[VERDICT_OF[mark.worth]] : null;
    if (shot) {
      const high = box.r * 1.5;
      const wide = (shot.image.width / shot.image.height) * high;
      c.drawImage(shot.image, px(mark.x) - wide / 2, py(mark.y) - high / 2 - since * 0.02, wide, high);
      continue;
    }

    if (show.skinned) continue;
    c.fillStyle = WORTH[mark.worth] || WORTH[0];
    c.font = `700 ${Math.max(11, box.r * 0.8)}px ui-monospace, Menlo, monospace`;
    c.fillText(SAID[mark.worth] || "×", px(mark.x), py(mark.y) - since * 0.02);
  }
  c.globalAlpha = 1;
}

function drawTrail(c, box, px, py, show, shot) {
  const play = show.scene;
  const now = cursorAt(play, show.head);

  const side = shot ? box.r * 2 : box.r * 0.8 * 2;
  const mark = (at, alpha) => {
    if (alpha <= 0) return;
    c.globalAlpha = alpha;
    if (shot) {
      c.drawImage(shot.image, px(at[0]) - side / 2, py(at[1]) - side / 2, side, side);
    } else {
      c.fillStyle = "rgba(255,255,255,0.75)";
      c.beginPath();
      c.arc(px(at[0]), py(at[1]), box.r * 0.32, 0, Math.PI * 2);
      c.fill();
    }
  };
  const sampleAt = (ms) => {
    const step = Math.round((ms - play.from_ms) / play.step_ms);
    if (step < 0 || step >= play.keys.length) return null;
    return [play.cursor[step * 2], play.cursor[step * 2 + 1]];
  };

  const ribbon = show.skinned && pics && pics.cursor_middle;
  if (!ribbon) {
    for (let age = TRAIL_STEP_MS; age <= TRAIL_DISJOINT_MS; age += TRAIL_STEP_MS) {
      const at = sampleAt(show.head - age);
      if (at) mark(at, 1 - age / TRAIL_DISJOINT_MS);
    }
    c.globalAlpha = 1;
    return;
  }

  const interval = (side * TRAIL_INTERVAL_SHARE) / Math.max(0.001, box.scale);
  const head = sampleAt(show.head) || [now.x, now.y];
  let last = head;
  let walked = 0;
  for (let age = 0; age < TRAIL_CONTINUOUS_MS; ) {
    age += TRAIL_STEP_MS / 4;
    const at = sampleAt(show.head - age);
    if (!at) break;
    walked += Math.hypot(at[0] - last[0], at[1] - last[1]);
    last = at;
    if (walked < interval) continue;
    walked = 0;
    mark(at, 1 - age / TRAIL_CONTINUOUS_MS);
  }
  c.globalAlpha = 1;
}

function drawCursor(c, box, px, py, show) {
  const play = show.scene;
  const now = cursorAt(play, show.head);
  const rules = show.skinned && pics ? pics.rules : null;
  const trail = show.skinned && pics ? pics.cursor_trail : null;

  drawTrail(c, box, px, py, show, trail);

  const expands = effects.expand && (!rules || rules.cursor_expand);
  const held = expands && (now.keys & 15) !== 0;
  if (show.skinned && pics && pics.cursor) {
    const side = box.r * (held ? 1.5 : 1.35);

    if (rules && rules.cursor_rotate) {
      c.save();
      c.translate(px(now.x), py(now.y));
      c.rotate(((show.head % CURSOR_TURN_MS) / CURSOR_TURN_MS) * Math.PI * 2);
      c.drawImage(pics.cursor.image, -side / 2, -side / 2, side, side);
      c.restore();
    } else {
      c.drawImage(pics.cursor.image, px(now.x) - side / 2, py(now.y) - side / 2, side, side);
    }

    if (pics.cursor_middle) {
      const middle = box.r * 1.35;
      c.drawImage(pics.cursor_middle.image, px(now.x) - middle / 2, py(now.y) - middle / 2, middle, middle);
    }
    return;
  }
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
  drawPlay(c, { scene, judged, head, skinned, popups: true, frame: true }, w, h);
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
  for (const piece of scene.objects) {
    const at = Math.floor(((piece.start_ms - scene.from_ms) / span) * buckets);
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
    loadPics().catch(() => {});
  } catch (why) {
    console.error(why);
  }

  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (opening) playOpening();
  armIdle();

  setTimeout(() => lookForUpdate(false), 3500);
})();
