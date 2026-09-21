# The application's design

Written 2026-09-15, before the first line of the native application, so that
the code reads its decisions from here rather than inventing them screen by
screen. The web application in `app/` was drawn the other way round — one
screen at a time, each with its own idea of a colour — and that is the reason
it is being replaced.

## The reference

One picture set the tone: a task panel, near-black, a single warm accent, a
headline saying what is happening now and how far along it is — *Restart
AirDrop · 5/24* — and beneath it a monospaced ledger where finished lines carry
a quiet tick and the current line is the only bright thing on the screen. As
lines finish, the list advances smoothly; nothing jumps, nothing shouts.

What it does right, in the order that matters:

1. **One thing at a time.** The headline is the whole status. Everything else
   is history or detail.
2. **Progress is a ledger, not a bar.** Steps are named; the eye reads what was
   done, not how much of a rectangle is filled.
3. **Two voices.** A sans headline, a monospaced ledger. Nothing else.
4. **One hue.** The accent is the only colour. Done and current share it; they
   differ in brightness, not in hue.
5. **Nothing moves suddenly.** Lines slide, they do not appear.

Dossier's application is built from these five, and a screen that breaks one
of them is wrong even if it looks fine.

## Tokens

These live once, in `native/src/theme.rs`. No colour, size or duration is
written anywhere else.

### Colour

| token | value | for |
|---|---|---|
| `ground` | black into burgundy — see *The background* | the window |
| `raised` | white 4.5% | cards, the sidebar |
| `sunk` | black 26% | fields, wells, the stage behind a frame |
| `line` | white 8% | every border; there is no other border |
| `line-high` | white 16% | a border under the pointer |
| `ink` | `#ece7e2` | the current line, headlines, values |
| `muted` | `#a9a29b` | finished lines, captions, secondary text |
| `faint` | `#6b655f` | what may be ignored |
| `accent` | `#e24848` | the mark's own red: the dot, the tick, the primary button (white on it) |
| `accent-soft` | accent 16% | a selected row, the halo of the dot |
| `danger` | `#e24848` | failure and destruction — the same red, told apart by glyph and words, never by hue |

One red, chosen 2026-09-15 over a warm gold and a rose: it is the colour the
mark already owns. It means "alive" on a dot and "done" on a tick; it means
"wrong" only together with a cross and a red sentence. A running line is a red
dot with `ink` text; a failed line is a red cross with red text. The glyph and
the words carry the difference, so a person who cannot tell the reds apart
still can.

Green does not exist. A finished step is a red tick, not a green one.

### The background

The window is black that flows into burgundy towards the top — the ground the
web application had, kept on purpose:

```
radial-gradient(115% 95% at 50% -18%,
  #3a1015 0%, #26090f 30%, #17070b 55%, #0d0508 78%, #070304 100%)
```

It gives the screen a top and a bottom the way a lit room does, and the
accent has somewhere to come from. Cards are glass on it — `raised` is a white
tint, not a paint — so the burgundy shows through them and nothing on the
screen is a grey slab. The *Background* setting keeps its four levels: the
slow drift of colour behind the content for *rich* and *live*, a still
gradient for *quiet*, and a flat `#0d0508` for *still*, which computes nothing.
The gradient is the only one in the application besides that drift; nothing
else fades from one colour to another.

### Type

| face | for |
|---|---|
| **Commissioner** 400 / 600 | everything that is a sentence: headlines, labels, buttons, captions |
| **JetBrains Mono** 400 / 700 | everything that is a record: ledgers, numbers, names of files and devices, tags |
| **M PLUS Rounded 1c** | fallback for kana and kanji, via the text stack's own fallback |

Varela Round leaves the application. It has no Cyrillic, and an interface in
Russian that switches face mid-sentence is the kind of seam this document
exists to remove. The engine's HUD keeps it.

Four sizes and no fifth:

| | px | line |
|---|---:|---:|
| `caption` | 12 | 16 |
| `body` | 14 | 20 |
| `lead` | 16 | 22 |
| `title` | 22 | 28 |

Numbers are tabular everywhere.

### Space and shape

- Grid of **8 px**; the allowed gaps are 4, 8, 12, 16, 24, 32, 48.
- Cards: radius **12**, padding **24**, border `line`, no shadow.
- Controls: height **32**, radius **8**, padding 0 12.
- The sidebar is **220 px**; content has **40 px** on each side.
- Shadows exist only under things that float — a sheet, a menu, a toast — and
  only one: `0 16px 40px black 45%`.

### Motion

- One curve, `cubic-bezier(0.22, 0.9, 0.28, 1)`; **200 ms** for a state
  change, **320 ms** for something entering or leaving, **450 ms** for the
  ledger advancing.
- A line that finishes fades to `muted` and the list slides up; a new line
  fades in from below. Nothing pops in at full opacity.
- A change of screen crossfades; the sidebar and the headline never move.
- With reduced motion, every duration is zero. Nothing depends on an animation
  having happened.

## Language

The application speaks **English first and Russian second**, and both are
first-class: every screen is drawn and approved in both, every layout fits the
longer of the two strings, and a string that exists in one language and not the
other fails the build. The engine's users are the international community; the
bot's are Russian-speaking; neither is an afterthought.

- Strings live in Fluent files, `native/lang/en-US.ftl` and `native/lang/ru-RU.ftl`,
  and nowhere in the code. Fluent is chosen for Russian plurals — *1 карта,
  2 карты, 5 карт* — which no format string handles honestly.
- The system locale picks the language the first time; the first screen of the
  first run lets the person change it, and Settings keeps that choice.
- This document writes copy in English and gives the Russian beside it where
  the wording matters. Golden frames are named with the locale.

## The ledger

The application has one way to show that something is going on, and it is the
panel from the reference. It is used for a render, a job for the bot, a download of
maps, the first-run checks, an update — anything with steps.

```
● Rendering for the bot · 4 of 6          ● Рисую для бота · 4 из 6

  ✓ Replay received                         ✓ Реплей получен
  ✓ Map on disk                             ✓ Карта на месте
  ✓ Judgement agrees                        ✓ Судейство сошлось
  ● Drawing · 4,512 of 7,280                ● Рисую · 4 512 из 7 280
    Encoding                                  Кодирую
    Delivering                                Отдаю боту
```

- The headline is the current step and the count, parted by a middle dot
  that stands as far from the one as from the other; the same dot parts a
  line's name from its detail.
- When there is a fraction worth knowing, it sits on the current line, not
  in the headline.
- Finished steps are `muted` with a red tick. The current step is `ink` with
  the red dot. Steps to come are `faint` with nothing.
- At most five lines are visible; older finished lines scroll out of the top
  as new ones finish. The scroll is the 450 ms slide.
- A failed step turns `danger` and the ledger stops there, with one line of
  reason beneath it and the button that undoes or retries. Nothing else on the
  screen goes red.
- There is no percent bar. If a step is long, its own line carries the
  fraction and, if it is known, the time left.

Every producer of progress in the engine and the application reports steps
into this shape. A screen never invents its own spinner.

## Screens, and the least each may show

The rule from the person this is for: *minimise the information, or it is
easy to lose the thread and to see the flaws.* Each screen below lists
everything it shows. Adding a line to a screen means adding it here first.

**The mark** is the letter D with four slits through it, evenly spaced, red.
The application icon puts it on a rounded tile that runs from the burgundy at
the top of the ground to the black at its bottom; inside the window it stands
bare beside the word, centred above whatever the screen is about, and it is
the one thing that reacts to the application's own moments — a check passing,
the bot saying yes — with a beat of 420 ms. No rings, no border, and the very
same letter, from the same drawing, sits in the heart of the QR.

**The word.** This device, lent to the bot, is a *worker* — *воркер* — and
never a farm; the farm is the bot's word for all of them together.

**The main screen** is the scene, the viewer and the journal in one, chosen
2026-09-16 over a sidebar with cards and eight other directions:

- *The scene is the window.* The chosen replay fills it, dimmed at the top
  and the bottom; the middle is the engine's own picture with nothing on it.
- *The viewer is the lower third*, anchored to the bottom so that whatever
  grows in it grows upward. Left: the date and the client in mono, the player
  large, the map, then one mono line — mods as lettered badges · combo ·
  length, a dot between each — and one button, *Render*. Right: the accuracy
  in the largest type on the screen, its outcome under it.
- *The journal is the strip along the bottom.* Replays only, as small frames
  grouped by day, newest first, the chosen one outlined in red with no glow
  and a little larger. A frame is the map's own background — what song
  select shows, the picture a player already knows the map by — at about
  half strength until it is hovered or chosen, and nothing on it: no grade,
  no number (decided 2026-09-16 after seeing it with real maps; the grade
  lives in the caption, in its own colour — SS pale gold, S gold, A green, B
  blue, C purple, D and F the accent red — the one place the single hue
  gives way). Not the accuracy: every engine frame looks like every
  other and 98,71 beside 97,88 tells the eye nothing; the number lives in
  the viewer where it is large, and the rest — full combo, sliderbreak,
  misses — is the caption on hover. A replay whose map is not on disk is a
  frame of fine diagonal hatching. A day is labelled the way its language says it:
  *today* / *сегодня* as the bare word, then *Aug 14* / *14 авг*, and the
  year only when it is not this one — *May 10, 2025* / *10 мая 2025*. It
  scrolls sideways, and the line above it is its scrubber: a two-pixel
  track with the visible stretch drawn on it in the muted tone, brighter
  under the pointer, draggable and clickable; *36 / 179* sits at its right
  end in eleven-pixel mono, faint. Frames are 108×61, the chosen one
  116×65.
- *Thin chrome.* One row at the top: the crest — the first run's block at
  the first run's sizes, the letter at 36 px, a 22 px rule, the word at
  20 px — at the left, and three words at the right: Replays, Worker,
  Settings. No search field (taken out 2026-09-16; the strip and the keys
  are enough for now, and a search may come back when the library asks for
  it). There is no state line and nothing about the worker on this screen:
  a worker has its own screen, and an operations centre for downloads and
  notices comes later.

### What each thing does

Every pointer and key on the main screen, so that nothing is invented at
the keyboard. Where a choice is still open it is marked *open*.

*The scene.*
- At rest the engine draws the chosen replay live, muted, at a low rate; on
  the *still* background setting it holds one frame. *Open:* whether it plays
  always, only while the window is focused, or for a few seconds after a
  choice and then holds.
- Click pauses and resumes it. Double-click hides all chrome — the picture
  alone — and Esc brings it back.
- Vertical wheel scrubs the replay's time; horizontal wheel or a two-finger
  swipe moves to the next or previous replay, the same as ← and →.
- Drop `.osr` files anywhere on the window: they are copied into the
  application's own Replays folder, appear under *today* and the first one
  is chosen.

*The viewer.*
- The date line: hovering shows the exact time and the file's name;
  clicking shows the file in its folder.
- The player's name: clicking filters the journal to that player, and the
  filter appears in the search field as a word to delete.
- The map: clicking filters the journal to that map — every play of yours on
  it, which is the "by map" direction folded into a click. *Open:* whether
  a second click on the map opens its page on the osu! site.
- A mod badge: hovering names it in full; clicking filters by that mod.
- The accuracy: hovering shows the four counts under it with their dots —
  300 · 100 · 50 · ✕ — and they stay while the pointer is there; clicking
  pins them. *Open:* whether a click flips the number to the unstable rate
  instead.
- The outcome tag: hovering says how many misses and where the first one
  fell; clicking seeks the scene to that moment.
- *Render*: the button itself becomes the progress, the way lazer's
  download button does (chosen 2026-09-21 over a ledger in the viewer,
  which took the meta line's place and was too much): its face turns to
  the soft accent, one word on it follows the work — *Reading*, *Judging*,
  *Drawing*, *Saving* — and a two-pixel bar along its bottom fills, eased
  frame by frame so it never jumps; the bar is the number, so there is no
  number. Clicking it stops.
  The player's name and the map stay, the accuracy stays; the frame in the
  journal wears nothing (a red dot on it was tried 2026-09-21 and did not
  belong to the picture). Encoding is not a step of its
  own: the encoder eats frames as they are drawn, so *Drawing* is both.
  When it is done the button reads *Open*, with *In folder* as a link
  beside it, until the next choice; a stop or a failure leaves a quiet
  *Once more*. The scene keeps playing throughout. Built 2026-09-16: the
  engine's own pipeline in a thread of its own, its progress events read
  as they come, `halt` for *Stop*. The picture is 1920×1080 at 60 fps, crf
  20, preset medium, with the map's background, the map's own hit-sounds
  over the *click* kit, no storyboard and no video — the defaults until
  Settings exists — and lands in the application's own `Renders/` as
  *Player - Artist — Title [Version].mp4*.
- Right-click anywhere in the viewer or on a frame opens the menu where the
  pointer is: *Render* (also Enter), *Show in folder*, *Open .osr*, *Copy
  path*, and after a rule *Delete* in red (also ⌫; asks once). Esc or a
  click elsewhere closes it. Judging is not in the menu yet; it comes back
  in its own time.
- A replay without its map: the scene cannot show the play, so it shows
  what the replay alone holds — the cursor's path, drawn live over the
  hatched ground in the accent, fading behind the cursor. The header alone
  gives the player, the mods, the combo and the counts, so the accuracy and
  the grade are real, not a dash; the length is the replay's last frame; the
  map's name is parsed from the file's name when it has one, else *Unknown
  map*. One button, *Get the map*, and no line beside it — the hatched
  ground has already said why. Its frame in the journal is hatched. Built
  2026-09-21: the path is the replay's own frames, the last three seconds
  drawn as a fading accent line with the cursor as a dot and a ring, looped
  over the play, paused and resumed by a click like the live picture; the
  hatched ground is a picture too, dimmed at the top and the bottom the
  same way as every other scene, so it never looks brighter than its
  neighbours.
- *Get the map* is the same progress button, a word at a time: *Looking*,
  *Found*, *Downloading*, *Unpacking*, *Checking*, the bar filling with the
  bytes; clicking it stops. (A mirror's answer to a hash carries the set's
  number and little else; the song's name arrives with the files.) When
  the hash agrees with the replay's, the map joins
  every replay that names it, the scene crossfades from the hatched ground
  to the map's background, the frame takes it too, and the button reads
  *Render*. When no mirror knows the map, the line turns red — *Not on any
  mirror* — as a quiet *Not found*; a stop or a failure leaves *Once more*.
  Built 2026-09-16: osu.direct then catboy.best for the look-up,
  the answering mirror first for the file, the other as the second try; the
  set is unpacked into the application's own Songs (or a plain folder's
  own Songs when that is the source), never into the game's; the folder is
  named *<set> Artist - Title* when the mirror said so and *<set>* when it
  did not.

*The journal.*
- Hovering a frame lifts it 2 px and brightens it to full over 200 ms —
  each frame with a rise of its own, so moving from one to the next lifts
  the new while the old settles back, both smoothly — and a bubble appears
  above it: the player with the accuracy at the right, the map's title
  under in the muted tone, then mods · combo · outcome · grade in small
  mono; 260 px wide, 8 px above the frame, 10 px radius, a hairline border,
  a soft shadow, no caret; it fades and rises with the frame's own lift.
  The bubble is a layer of the screen placed by the frame's arithmetic, not
  a tooltip widget: a tooltip inside the lifted frame lost the strip's
  scroll, and outside it never heard the pointer. The viewer does not
  change (a preview in the viewer was tried 2026-09-21 and the bubble
  won). The scene does not change.
- Clicking a frame chooses it: the outline slides to the frame (200 ms), the
  scene crossfades to the new replay (320 ms), the viewer's words erase from
  the right and type in from the left (450 ms, the same typewriter as a
  language change), the accuracy included. → and ← do the same for the
  next and the previous frame. Nothing shows through in between: the old
  picture — the live play's last frame over its background — stays whole
  until the new background is decoded, and only then does the crossfade
  begin; the new live play fades in over it when its first frame comes.
  (A flash of the hatched ground between two replays was the first bug the
  user saw; it was the hatch standing in for a picture not yet decoded.)
- Clicking a day's label scrolls the strip to that day. Home and End go to
  the newest and the oldest.
- The rail above the strip is a map of time: its ticks are the days, the
  bright stretch is what is on screen; dragging it scrolls.

*Search.*
- Typing filters the strip as you type — there is no second list of results,
  the strip is the results. Frames that do not match dim to a third; a day
  left with nothing folds away; the first match is outlined at once and the
  viewer already shows it; the field says *2 / 187* and the counter above
  the rail *1 / 2*. Enter keeps the outlined one and leaves the field, ↑ ↓
  or ← → walk the matches, Esc clears and the days unfold.
- It understands a player, a map, a mod acronym, *fc* and *miss*, and a day
  in the language's own words — *yesterday*, *august*, *вчера*, *август*.

*Nothing yet.* An empty library says *No replays yet · Drop .osr files
anywhere on this window* and offers one button, *Look on this device*.
That walks the home folder — Downloads, Desktop and Documents first, then
the rest — for files that carry a replay's signature, skipping what cannot
hold one (Library, AppData, Applications, node_modules, targets, media
folders, anything hidden) and the folders already read as sources, as a
one-line ledger: *Looking on this device · 84 120 files · 37 replays · 12
s*, with *Stop*. What it finds is remembered in the application's own
`found.json` and read as a source of its own, *found*, beside the others;
maps sitting in a *Songs*, *Beatmap* or *Beatmaps* folder next to a found
replay are indexed too. Two copies of the same replay are one entry: the
library keeps one file per replay hash, the first source's. The client in
the date line — *stable* or *lazer* — is read from the replay itself, not
from where the file was found. Built 2026-09-16; a rescan from Settings
comes with Settings.

*The stripe, the chip, the menu.* The island was drawn (`docs/mockups/main/
island.py`) and set aside the same day, 2026-09-21: the strip from the
first centre round is kept, but it must earn its place — it shows what
matters, not a mood. The stripe is a 30 px line under the top row, there
only while there is something to say, made of segments with hairlines
between: at the left the jobs of this machine (a breathing dot, one word,
the thing's name, a mono figure, a two-pixel line of progress under the
segment); then the latest notice (a tick and *Карта скачана · 13:51*, or in
danger red a cross, *Рендер не завершился* and its own *Ещё раз*, which
stays until dealt with); at the right the worker (*Воркер · готов · 2 в
очереди*, or *рисует для @friend · 3 из 7*, or *Офлайн · повтор через 40
с*). Done: *Готово · Открыть · В папке* for five seconds, then the segment
goes. Nothing happening, no stripe. The top-right corner answers one
question only — from whose name — as an account chip beside the three
words (an avatar dot and the handle; a plain avatar, a count on the word
*Воркер* and the bare row were drawn and read worse). The chip opens the
operations menu; of the four menus drawn — sections, a timeline, two
columns, a minimal list — the timeline is proposed: one column of *time ·
mark · words*, the live jobs at the top with their figures, then what
happened, the account line and *Esc — закрыть* at the foot. Three places,
three questions: the stripe says what is happening, the chip whose, the
menu what was. Drawn in `docs/mockups/main/strip.py`; awaiting the user's
pick, then built after the notice queue.

*The three words.* Replays is this screen. Worker and Settings open over
the scene, dimmed to a fifth, in the first run's centred column; the crest
does not move, and Esc or the word *Replays* brings the scene back. Until
they are built, each is one card — its name and *Coming later* / *Будет
доступно позже* — with *Back to replays*.

*The scene.* The engine draws the chosen replay live, bare — no score, no
counters, no key overlay, the play alone over the map's background — at
960×540 on one CPU thread, one frame for every beat of the window's own
redraw — the display's refresh rate, 60 or 120 — asked for by the window
and drawn to order, so the picture and the screen never disagree; when a
frame takes longer than a beat the requests fold into one and the play
simply skips ahead. It loops from the lead-in to the end, muted. Each frame
is dimmed before it is shown: near-black at the top and the bottom, easing
to a centre held at two-fifths dark, so the words read over any play. A
click on the scene does not freeze it: the play eases to a stop over 700
ms and rests sharp (a blur on rest was tried and taken out); another click
eases it back up to speed. Under the live picture, and
before its first frame arrives, sits the map's background, blurred and
dimmed harder (two-thirds at the centre); the picture fades in over it in
640 ms. A choice stops the old play and starts the new one. The dim is baked
into the picture rather than laid over it, because a gradient quad's alpha
does not blend reliably in the GPU renderer (a five-stop gradient over the
picture drew nothing in the window), and a picture is the same in both
renderers anyway. A replay without its map shows the hatched ground. Built
2026-09-16; still open: whether the play should rest after a while or when
the window loses focus, and the wheel as a scrub.

### Entering

- The crest does not travel (the glide from the centre was tried and
  looked wrong): it fades in where it lives, top-left, rising 8 px as it
  comes (450 ms), the moment the window opens and before anything else has
  arrived. Then, once the library is read, the scene fades up from black
  (640 ms), the journal rises from below the edge (450 ms), the viewer's
  words type in, and the three words fade in last. About 1.2 s, nothing
  jumps.
- From the first run: on *Open Dossier* the card, the ledger and the
  centred crest fade out together, and the main screen enters as above.
- From a cold start: the window opens black, the crest fades in top-left
  while the library is read — the emblem is the loading screen, no spinner
  — and the rest follows. If reading takes long, the crest's dot breathes.

**Реплеи** is the main screen above; there is no separate library. What a
replay is beyond the viewer — the judgement's counts, whether it agrees with
the header, the score — is designed after this screen is real, not before.

**Воркер.** Title. One switch: take work from the bot. Two cards: the device
(state line, three tiles: speed, delivered, handed back; then only the checks
that fail, each with its fix) and the ledger of what is being drawn now. Below,
who else is online: device, state, threads, delivered.

**Настройки.** A rail of eight: Подключение, Папки, Видео, Звук, Скины, Игра,
Клиент, Авторство. Each row is a name, at most one short line under it, and
its control on the right. No row explains what the person can see for
themselves.

**First run** is built first, and it is the ledger applied to setting up. No
sidebar; one column of 560 px in the middle of the window; the mark and the
word above it. The headline reads *Setting up · 2 of 4* and the ledger beneath
it names the four steps — Language, osu! folder, This device, The bot — with
the current one bright. Under the ledger, one card for the current step: a
title, at most one line of why, the control, and a row of two buttons —
*Continue* and a quiet *Back* or *Skip*. Nothing else is on the screen.

1. **Language.** Two options, *English* and *Русский*, the system's one
   preselected. This is the only step that cannot be skipped, and it is first
   so that every word after it is in the right language. Choosing the other
   one retypes every word on the screen over 640 ms: the old words are
   erased from the right first, then the new ones are typed from the left —
   the whole interface, not the card alone.

2. **osu! folder.** The application has already looked, and it looks the way
   the game does rather than by guessing folder names:

   - **stable** is a folder with `osu!.exe`, or with `Songs/` beside a
     `osu!.<user>.cfg`. That file is read: `BeatmapDirectory` may point the
     songs elsewhere, and the application follows it rather than assuming
     `Songs/`. `Skins/` and `Replays/` are checked separately, and one that
     is missing is said so — *replays: none yet* — not treated as a failure.
     Roots tried: `%LOCALAPPDATA%\osu!` and `%PROGRAMFILES%\osu!` on
     Windows; `~/osu!`, `~/osu`, `~/Games/osu!` and every Wine prefix's
     `AppData/Local/osu!` elsewhere.
   - **lazer** is a data folder with `client.realm` and `files/`:
     `%APPDATA%\osu` on Windows, `~/.local/share/osu` on macOS and Linux —
     and its `storage.ini` is read first, because lazer lets the person move
     that folder and writes the new place there. Lazer keeps everything in a
     content-addressed store the application cannot name files in without
     the database, so it takes from lazer only what it can recognise by
     content: **replays** — every file in the store that parses as an `.osr`,
     and everything in `exports/` — and **skins** exported as `.osk`. Maps
     are not taken from lazer at all: a replay's map is found in stable's
     folder when there is one, and otherwise fetched from the mirrors the
     application already uses. The card says exactly that in one line.
   - Both found: two rows, one per client, each with its path, its counts
     and a switch that is on — *Use both* is the button, and turning one
     switch off is how a person chooses. See *Sources* below for what "both"
     means. Neither found: *Couldn't find osu! on this device*, one line
     saying the application can keep maps, skins and replays in a folder of
     its own, a *Browse…* button that accepts either kind of folder, and a
     quiet *Keep everything in its own folder*, which makes `~/.dossier` a
     source like any other — with its own Songs, Skins and Replays — and
     shows it as one, tagged *dossier*. Nothing after that step changes: the
     bot stays optional, replays dropped into that folder render locally.

   Found: the client's name as a tag, the path in a well, and three tiles —
   maps, skins, replays — with *Use this* and a quiet *Add another…*, which
   accepts a second install, a lazer folder or a plain folder of replays.

   **Sources.** The application does not have a songs folder, a skins folder
   and a replays folder; it has a list of *sources*, each a place it knows
   how to read: a stable install (maps, skins and replays, through the
   game's own config), a lazer data folder (replays and exported skins), or
   a plain folder of `.osr` files. Everything shown is the union of the
   sources. A replay or a skin remembers which source it came from and wears
   the client's tag; the same replay in two sources — the same replay hash —
   is shown once. A replay's map is looked for in every stable source and
   then on the mirrors. Downloaded maps land in the application's own
   folder, never inside a game's. The first run adds what it found; Settings
   → Folders is the same list, with *Add…* and the switch per source, and
   nothing else on that page.

3. **This device.** One field, prefilled with the machine's name. One line:
   *The worker's name, as the bot will show it.* (*Имя воркера — так его
   покажет бот.*)

4. **The bot.** No address and no token: the application knows the bot, and
   the person only has to be recognised. The card shows a code — `K7QN-M4XZ`,
   eight letters from an alphabet without `0`/`O`/`1`/`I`, good for ten
   minutes, single use — a QR beside it that encodes
   `https://t.me/<bot>?start=pair-K7QNM4XZ`, and a button *Open Telegram*
   that opens the same link on this machine. Under the code, one line with
   the red dot: *Waiting for Telegram…*. The person scans or taps; the bot
   answers with the card from the roadmap — *Add „MacBook Pro" — macOS on
   ARM, 12 cores, build 0.11.0?* — and a button; on yes the application's
   next poll comes back linked. The line becomes *Linked to the bot* with a
   red tick, the code and the QR go, and *Continue* lights up. One quiet
   link, *Later — just my own replays*, and nothing else: there is no field
   to type a code into. The code on the screen is not for typing — the bot's
   card repeats it, and the person holds the two against each other before
   pressing yes.

   The QR is drawn, not pasted: round modules, finders rounded only as far
   as a reader still finds them, the mark in the middle at error-correction
   level H, and a test that reads the drawn code back into the link.

   Linking is also what lets the application show the person their own side
   of the bot — what they have queued, what was rendered for them, what
   their device has done — which is a screen of the main menu and is
   designed with it.

Then the ledger runs on its own — *Checking · 3 of 4*: osu! folder, ffmpeg,
engine, bot — each line ticking as it passes. Everything passed: one button,
*Open Dossier*. A line failed: that line turns `danger` with its reason —
*ffmpeg · not installed* — and nothing is explained under it; where the
explanation used to be sits the fix, a *Download* button indented under the
line. It fetches a static build for this system into the application's own
`bin/` — martin-riedl.de for macOS and Linux (with osxexperts.net and
evermeet.cx as the second try on a Mac), gyan.dev for Windows — and the line
itself is the progress: *ffmpeg · downloading · 12.4 / 27.5 MB ·
martin-riedl.de*, then *unpacking*, then the check runs again and the line
ticks with the version. The card's own row does not change: *Check again*
and *Continue anyway*, because a missing ffmpeg stops rendering, not
judging. Only when no build could be fetched does a line under it say why,
with *Where to get it* as the link and the button offered again; a system
nobody builds for gets the link from the start. The application's own
`bin/ffmpeg` is looked for before the PATH from then on. A bot that was skipped with *Later* is not a failure: its line stays
quiet, *not linked*, and counts as done.

The other way round — osu! found, bot skipped with *Later* — is the ordinary
case for someone who only wants their own replays drawn. Then the worker is
not hidden but idle: its screen shows the same code-and-QR card instead of
the device's status, and the sidebar's bottom line reads *Not linked to the
bot* rather than a readiness. Nothing about the bot appears anywhere else.

The main screen is designed after this flow is approved, not before.

**Судейство and Студия** are deferred. They are the engine's own views and
deserve their own document once the five above are real.

### What the renderer taught us

Learned building the main screen, kept so nobody rediscovers it:

- Text inside a `pin` that moves every frame is not drawn while it moves;
  the emblem and the rule were, the word was not. Moving something means a
  `float` with a translation, which draws through a transformation, and the
  word stays. The crest's small rise is done that way.
- A quad with a gradient background is not trusted for alpha in the window:
  two stops blend, five stops over a picture drew nothing. Dims that must be
  exact are baked into the picture.
- The test simulator's image atlas does not grow: after a strip of
  thumbnails, a scene picture wider than about 800 px is silently not drawn
  in a snapshot, though the window draws it. Frames for the gallery keep the
  scene at 640 px, and the application itself decodes it at 960 px, blurred,
  which is all a backdrop needs.
- Within one layer the renderer draws quads before images; a veil meant to
  sit over a picture needs a layer of its own (`with_layer`), or, better, no
  veil.
- A `FillPortion` row inside a `stack` child drew nothing at all; a
  two-pixel bar that must be a fraction of its button is a small canvas.

### Two renderers, one look

The window draws with `wgpu`, which blends colours in linear light; the
software fallback, `tiny-skia`, blends in sRGB. The translucent tokens above
are written the way a stylesheet would write them — white 4.5 % — and read
that way on the software renderer; the code carries their linear-light
equivalents (white 0.77 %, black 48 %, and so on) so the window shows the same
glass. The approved frames are always taken with the renderer the window
uses on that machine, and a frame's file name says which; a machine without a
GPU takes its own set rather than comparing against another renderer's.

## What is checked by a machine

Every screen above is drawn as a pure function of its state, so every state of
it can be drawn without a window and compared with a picture that was approved.

- `tests/golden/<screen>-<state>-<size>.png` — approved frames. A frame is
  approved once, by a person; after that any difference fails the build.
- States: empty, loading, ready, busy, failed, first run. Sizes: 980×720,
  1280×800, 1920×1080.
- Invariants checked on every frame, with no picture needed: no text is
  clipped by its box, nothing lies outside the window, every clickable thing
  is at least 28 px tall, every text meets 4.5:1 against what it sits on, no
  size is NaN.
- `native --gallery <dir>` draws every screen in every state at every size to
  PNG, which is how the frames are reviewed before anyone runs the window.

## The stack this assumes

Rust throughout; `iced` on `wgpu`, with its software renderer as the fallback;
text through `cosmic-text`, which does the shaping and the fallback the
application needs; `iced_test` for the frames above. The engine stays on the
CPU and on `tiny-skia`, because a video for the farm must be identical on
every machine and a GPU does not promise that. What Tauri used to provide —
tray, dialogs, links, bundles — comes from `tray-icon`, `rfd`, `open` and
`cargo-bundle`.

**The engine ships inside the application.** It is the `crates/dossier-*`
workspace, linked into the one binary the way `app/` linked it; there is
nothing to download separately, and only `ffmpeg` is looked for outside. An
engine version is therefore an application version: the bot's `hello`
answers whether it agrees with this build, and when it does not the worker
is not given work and the application says a newer build is needed. An
updater — the application fetching its own next build — belongs to the
operations centre, later.

**Rehearsing without the game.** `dossier --open <folder>` opens any folder
of replays in the real window without touching the saved settings, `--snap
out.png --after ms` takes the window's own picture and leaves,
`--render-first` presses Render on arrival; `dossier --render <replay>`
renders one file from the terminal with the steps timed; `--library
<folder>` prints what the index made of a folder. The corpus is the
rehearsal stage.

**Maps come from mirrors by hash.** A replay names its map by MD5 alone, and
ppy has no endpoint from a hash to an id, so a mirror is asked: osu.direct
first, because it also carries graveyard, which is most of what a replay
from a friend is played on; catboy.best second, when the first says no. The
`.osz` is downloaded from the mirror that answered, unpacked into the
application's own Songs folder — never into the game's, and never into
lazer's store, which is the game's to write — and kept only if the header
reads `osu file format v`, the size is under 50 MB and the MD5 is the one
the replay asked for, following osu!'s own `BeatmapStore`. This is what
`app/src/mirror.rs` and `tools/fetch-maps.py` already do, carried over.
