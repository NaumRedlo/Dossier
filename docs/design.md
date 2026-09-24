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
  *Drawing*, *Saving* — and the progress fills the whole button from the
  left, edge to edge (asked 2026-09-21; a six-pixel-inset bar looked
  mean): a tint of the accent over the face and a two-pixel line at its
  foot, both eased frame by frame so they never jump; the button keeps one
  width, 150 px, so the word changing does not move it. The fill is the
  number, so there is no number. Clicking it stops.
  The player's name and the map stay, the accuracy stays; the frame in the
  journal wears nothing (a red dot on it was tried 2026-09-21 and did not
  belong to the picture). Encoding is not a step of its
  own: the encoder eats frames as they are drawn, so *Drawing* is both.
  When it is done the button reads *Open* and nothing sits beside it
  (*In folder* as a link there was dropped 2026-09-21: the folder is in the
  video store), until the next choice; a stop or a failure leaves a quiet
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
  above it that says what the viewer does not (chosen 2026-09-21 from
  seven drawn in `docs/mockups/main/bubbles.py`; the first bubble repeated
  the viewer and so said nothing): the player as the head with the grade
  in its colour and the accuracy at the right; artist — song under it in
  the muted tone; then the judgement in small mono, 300 · 100 · 50 · ✕
  each in its colour with the count in bold; then the combo out of the
  map's maximum (counted by dossier-assay on first hover, in a thread),
  the outcome mark (left out on a fail, the grade already says F), the
  client and the day and time played. Every line is one line, cut with
  an ellipsis (the player past 22 characters, the song past 44) and the
  card clips, so nothing runs past the edge; 300 × 88,  10 px radius, a hairline border, a soft shadow, and a caret at its foot
  pointing at the frame's middle, the card sitting above the day labels
  with the caret's tip 14 px over the frame. It grows from the caret's
  tip — scale 0.84 → 1 and a fade, on the frame's own 200 ms rise — rather
  than sliding in, so it reads as coming out of the frame (asked
  2026-09-21 after two rounds of a bubble that slid and drifted). The
  bubble is a layer of the screen pinned at the frame's bounds in window
  coordinates: the frame reports them itself when the pointer enters it,
  subtracting the strip's scroll (a widget inside a scrollable sees
  unscrolled coordinates; the event's own cursor position gives the
  shift), and a scroll of the strip shifts the bounds with it, so the bubble
  stays on its frame. The frame's rise is a draw-time transform, not a
  floating overlay: floated, the frame became an overlay under the pointer
  and the strip stopped hearing the wheel. The viewer does not change (a
  preview in the viewer was tried 2026-09-21 and the bubble won). The
  scene does not change.
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

*The corner, the account, the video store.* The dot, the island and the
stripe were each drawn and each set aside on 2026-09-21; what stays in the
top-right corner is a person. Until the device is linked, a 28 px circle
in the grey hatching — the same hatching as a frame without a map: the
place is there, the person is not. After linking, the Telegram avatar,
28 px, no name (the name is in the menu); while something runs — a
render, a map, a send, a worker job — it wears a ring of the accent,
which goes out by itself (accepted 2026-09-21; a sign-in button in the
corner was drawn and replaced by the circle). The circle and the avatar
open the same menu, split into three tabs, *Аккаунт*, *Лента* and
*Статистика*. Signed out, the head says *Вход не выполнен · видео
остаются на этом компьютере* and the account tab says why to sign in
and holds the sign-in button, nothing more. Signed in, the head is the
name, the handle and the day of linking; the account tab keeps only the
important words — where videos go, the worker's state, the build — and
*Выйти* in the foot. The *Лента* tab is one timeline of the day, time ·
mark · words, running jobs on top with a progress line under each and
no percentage (the line is the number), a failure in red with its own
*Ещё раз*, *Всё прочитано* in the foot; it works signed out too, as the
device's own story. *Статистика* is where the figures live, so the
other tabs need not carry them: as a worker — jobs done and this month,
gigabytes given, the mean job time, today and the queue; on this device
— replays in the journal, renders and videos with their size, sent to
Telegram. A finished render announces itself top-right, under the words:
a card with a tick, *Отрендерено*, *Открыть* at the right and who — map
· length · size beneath, dropping 8 px and fading in over 240 ms,
standing six seconds or while hovered, leaving upward in 200 ms; a
second card stacks under the first, three at most; a failure's card has
a cross and *Ещё раз* and stays until dealt with; every card also goes
to the *Лента*. Signing in is a card, not a password: a code that lives five minutes and a QR,
*Открыть Telegram* leads to t.me/‹bot›?start=‹code›, the bot answers
*Привязано*, the application waits for that answer, and the avatar fades
in where the hatching was. Errors: E — when the application cannot go
on, the scene dims and one card, the first run's, says what, where and
the ways out (chosen 2026-09-21; B stays for Render and Get the map,
where the button already is the story).
Rendered videos live in the application: a fourth word, *Видео*, opens
a list — frame, when, who · map, mods, length, size — newest on top
(chosen 2026-09-21 over a grid and over a mirror of the main screen; a
Telegram column was drawn and dropped as foreign to the list). A click
opens the player over almost the whole window, 40 px from the edges: the
video 16:9, the scrubber and the time under it, then a caption block —
the player's name large, the map under it, and a small mono line of
mods · length · resolution · fps · size · when — with the buttons
*В Telegram · В папке · Удалить* at the right; Esc or a click outside
returns to the list. Space pauses, the arrows step five seconds, a
double click fills the screen; while it plays, the scrubber and the time
stay and the rest dims a little. Sending to Telegram happens only here,
with the same one-word progress button (*Отправляю*, the fill following
the bytes) and the ring on the avatar; done, the button returns and the
top-right card says where it went. Deleting is the screen's one
question, asked the way the first application asked it: no card, only a
deeper scrim and a small centred stack — the video's frame, *Удалить видео?*,
who and the weight, the line about the bin, and the two answers side by side
— rising and fading in over 220 ms. A render that finishes while its own replay is the one
on the screen writes itself into the feed without the top-right card — the
person is already looking at it. With no videos yet,
one line and a *К реплеям* button under it. Drawn in
`docs/mockups/main/store.py`; built 2026-09-22 as drawn, with these
particulars: the store is `~/.dossier/Renders/videos.json`, renders
already in the folder are adopted (name → player, song, version; ffmpeg's
banner → length, size, rate) and married to their replays by player and
song; the player decodes through two ffmpeg pipes, raw RGBA frames paced
by the decoder thread and read on the redraw beat, f32 sound into a cpal
stream, a seek restarting both; the player's card is sized to its picture
and centred; notices live in `~/.dossier/notices.json`, a hundred at most;
the account is the first run's pairing (token and name), the bot answers
`/render/me` with the name, the handle and whether there is a photo, and
`/render/me/avatar` with it; sending is `POST /render/send` with the file
as the body and the caption in a header, the bot handing it to the chat,
refused past Telegram's size. The account tab shows the worker line as
*coming later* and the statistics tab the same for the worker's figures:
the bot has no endpoint for them yet.

The player is a window, not a panel (reworked 2026-09-23, after the first
build let the picture run to the card's edges); later the same day the card
around it went too, so the title, the cross and the buttons stand in the air
over a near-black scrim, the list behind is not drawn at all once the window
is open, and the window rises and settles in over 280 ms and sinks away the
same way when closed — the sound stops at once, the picture fades after. A title bar carries the
player's name with its mods beside it and the map under, the close cross at
its right, both edges lined up with the picture's own; the video sits
inside its own frame, inset 14 px on every side, rounded and on a ground
darker than the card, so the card reads as the window around it and never as
the video itself. Under the frame: the scrubber, which shows the time at the
cursor in a small bubble while hovered, fills in the accent colour, thickens
while held and only asks for the seek when let go — dragging moves the mark,
not the decoder. Then one row of controls, drawn from Lucide (ISC): the
previous and next video, five seconds back and ahead, play or pause in the
middle, the time as *at / length*; at the right a frame back and ahead (lit
only while paused), the speed as ×0,5 to ×2, the sound with its own short
slider, repeat and fill-the-window. The last row keeps only the three
buttons, centred in their strip, *Отправить в Telegram* last so that the one
filled button ends on the picture's own edge, as the close cross does above; and it folds away when the window fills the screen; the size, the
rate and the weight are the list's business, not the player's. Keys:
space or K pauses, the arrows step five seconds, M mutes, L repeats, F fills,
the comma and the full stop step a frame, the brackets change the speed, a
digit jumps to that tenth, Esc leaves the filled window first and the player
second; the wheel over the picture is the sound, a double click fills the
screen. Every change says itself once in a pill over the picture, which
fades in 900 ms. While the player is open the top row and the crest fade out,
so nothing of the application stands over the video. Sound is shared with the
audio thread, so the slider is heard at once; frames arrive at 960×540, and
the picture's box is rounded down to whole pixels, so a frame is never
resampled into a fraction of one.

The speed is a compass, not a button (2026-09-23): a rule of small and larger
ticks with the speed above the mark it points at, dragged or clicked to a
step, and the steps come round again, so ×2 leads back to ×0,5 — before, the
last step was a wall and the speed could not be brought back. A paused player
stays paused through a change of speed or a seek, with its sound stream
opened paused as well; it used to start the sound at once, so that on play
the sound ran ahead of the picture by as much as the speed had moved it. While the player is open the top row steps aside, and it comes
back the moment the player goes, whichever way it goes — closed, deleted, or
left behind by another catalogue — because the fade now follows whether a
player exists at all rather than a flag set by one path (fixed 2026-09-23,
when leaving the video by another door left the application without its top
row). The picture is
asked of ffmpeg at sixty frames a second whatever the speed (`fps=60/rate`),
and the decoder paces those frames by the speed, so a double speed no longer
asks for a hundred and twenty frames a second of raw video — that was the
flicker, and at times the picture gave out altogether while the sound played
on. Only the last frame of a beat becomes a texture, so a late drain costs
one upload, not four.

A render now carries the skin's own hitsounds: the sample pack is read from
the chosen skin's folder before the beatmap's own samples are laid over it
(fixed 2026-09-23; it used to be read from an empty path, so a skin was seen
and never heard).
*What the bubble says* is in the journal's section above; the seven
variants stay drawn for the record.

*Сообщество.* A fifth word in the top row, after *Видео* (2026-09-22),
for the world of 1984 seen from inside Dossier — people, their renders
and records, the bot's contests — so the application and the bot make one
ecosystem rather than two doors to the same house.

The first prototype (2026-09-23) holds one group — the Telegram chat the
bot keeps its players by — under the same underlined words as Settings:
*Профиль · Лента · Люди · Рейтинг · Титулы*, and a faint line under them
naming the group, its size and when the bot last answered — or *образцы
данных* while the figures are the sample staged in
`native/src/community.rs` in the shape of the bot's own tables.

- *Лента* is a chronicle (2026-09-24), chosen from three drawn concepts
  over a board of panels and a daily digest: one stream of everything that
  happened — the group's plays, new top plays, titles and climbs, osu!'s
  news, the chosen Telegram channels' posts and the game's builds — newest
  first under *Сегодня*, *Вчера* and the dates before. Pills filter it
  (*Всё · Игры · Топ-плеи · Титулы · Рейтинг · Новости · Обновления*); a
  card lights under the pointer, opens its details where it has any
  (a top play's hits and combo, a build's full list) and carries its own
  actions (the map, the reader, the browser, the rankings). What arrives
  while the page is open waits behind a red *N новых событий* at the top
  rather than pushing the stream down under the reader. Beside it: the
  person's own card (level ring, pp, rank, accuracy with the week's gain,
  the streak, *Мой профиль →*) with the filters and channels; *В центре
  внимания*, turning every five seconds to the week's biggest gain, the
  best accuracy, the newest title, the longest streak and the person
  themselves; the friends in the game; and the week's table, turning every
  six seconds through its six boards with a thin line draining towards the
  next and its bars growing afresh. The pointer on either holds it still;
  its dots and tabs choose by hand. At the width of three columns the
  stream is in the middle; narrower, the filters go above it and the
  person's card to the top of the right column.
  Reworked the same day after the first real window (2026-09-24): the side
  columns grow with the window (a fifth and a little under a quarter of it)
  and the stream stops at 860, the three centred together, so a wide window
  no longer gives the stream a hall of empty cards. A card's actions and
  *Подробнее* sit in its heading row rather than a row of their own; a
  failed play shows only its red F. The spotlight says who, why, one large
  figure and one small one beside it — no second pp, no best play; the
  week's table names its board above a single segmented line of six, and
  a row carries a move only when there is one. The pointer over either
  pauses it: the countdown stops where it was and goes on from there when
  the pointer leaves, and nothing turns or grows again on leaving (it used
  to restart both, so the spotlight blinked and the bars regrew).
  The middle was rebuilt the same night from the fifth of the concepts
  drawn on a canvas (two streams under the day's highlights). A segmented
  *Всё · Группа · Новости*, *N новых событий* and a search by player, map,
  title or words sit on one line; under them *Главное*, four cards over
  their covers or a wash of their colour — the newest top play, title,
  climb and piece of news — each opening its player, board or reader.
  *Всё* sets the group's journal beside the news (three parts to two; one
  above the other under 620). The journal has no frame, as the news has
  none: its heading, its pills *Всё · Игры · Топ-плеи · Титулы · Рейтинг*
  with their counts and its rows lie on the page, level with the news
  (2026-09-24, from the journal drawn on the feed's canvas). It is a table
  of rows of 40 under a head of *Время · Игрок · Карта · pp*: the time, the
  avatar and the name, the map's cover and its title with the version
  muted, the pp and the grade as a letter of its colour; below 600 wide it
  keeps those columns, and wider (*Группа*) it adds the accuracy and the
  mods. A top play, a title and a climb are one line across the player's
  and the map's columns — the event's glyph where the avatar stands, the
  name, what happened (*новый топ-плей*, *получил титул*, *поднялся в
  таблице*) and its object, a title in its rarity's colour — with the pp
  and the place or the move (*#4 → #2*) under the pp column; a top play
  opens onto its hits. Rows have no fill and no frame of their own, only a
  faint light under the pointer (tinted rows read as boxes). The news is
  cards, a post's picture or video above its words, an article's title and
  lead, a build's first three changes, with *Всё · osu! · Каналы ·
  Обновления*; *Новости* lays the cards two abreast. The filters left the
  side card, which keeps only the channels.
- *Профиль* is the dossier (2026-09-24): the ringed avatar with the level
  and its progress, the title worn, whether the person is online, the
  country with its rank, the years in osu!, the streak and the duels, *Открыть
  в osu!* and *Сравнить*; the place on each of the six boards this week;
  five figures — pp, global rank, accuracy, plays, hours — each with its
  gain, which choose what the chart draws over 30 or 90 days (rank from
  the daily history osu! keeps, the rest from the bot's weekly snapshots),
  the pointer reading any day off it; the best plays as five posters
  (chosen from four concepts drawn on a canvas, 2026-09-24) — the map's
  cover with the place over it and the grade in a glowing ring on its edge,
  the pp large, the title, the version, the accuracy and FC or the misses —
  with the chosen one's particulars under them: artist and title, version
  and mapper, the combo (with the map's own when osu! says it), stars, BPM,
  length, the day it was set, the mods, the 300/100/50/miss bar and a way to
  the map; the heading is only the panel's name, with no sum beside it. The
  bot's card knows a play's pp, grade and combo but not its hits, stars,
  BPM, length or day, so one's own dossier fills those from the osu! page
  it reads, play by play (the same map and difficulty). A cover drawn to
  fill its frame is drawn larger than the frame and spills, so a poster's
  cover is cut to the poster's shape (1.45 : 1) when it is fetched and
  drawn exactly into its frame, clipped to it, with one radius on every
  corner (iced lost a top-only radius); the poster's press, its edge and
  its light under the pointer are a clear button laid over the whole
  poster, so the edge runs unbroken round it and lights over the cover too
  (a stack draws its later layers after its neighbours, so the chosen
  poster's glow stays on the card beneath, not on that button, or it
  would fall over the next poster). The cover melts into the card's own colour: the shade
  spans the cover and the grade's ring below it and is opaque a few points
  above the cover's foot (a shade that ended at the foot let the cover's
  last row through as a line), the stars sit on it in a pill of osu!'s own difficulty colour,
  the mods in its corner, FC and the misses are small pills, and the chosen
  play's particulars lie over its cover in a frame of its grade's colour; the grades, the one under the pointer lit and its share told; the
  titles as a collection, bar by rarity, the held ones filled and the next
  ones dashed, a press telling what each asks and when it was earned; and
  thirteen weeks of plays a day. No panel says *наведите* or *нажмите*:
  the grades tell a hovered grade's share in their heading, where the total
  stands otherwise, and the days tell theirs under the squares only while
  one is under the pointer. The chart has no values at its side —
  the pointer reads them — and its line and its shading are one monotone
  curve through the days, so the fill never parts from the line and the
  dot sits on it. *Место в группе* says how many and which week once, and
  each tile the board, the place (gold, silver and bronze for the first
  three), a move as an arrow beside it and a thin bar for the standing. Its
  side columns grow with the window up to 380. Its data is the bot's card
  (`/render/me/card`) with the weeks, the days and the titles' dates from
  `/render/community`; the sample stands in until the application is linked.
- Where the figures come from: the feed's people, plays and happenings
  come from the bot once the application is linked to it
  (`/render/community`, a chat the person is in: the one videos go to if it
  is a group, their first group otherwise), are kept in
  `~/.dossier/community.json` and asked for again every minute while the
  catalogue is open. Until the application is linked, or while the bot
  cannot answer, the catalogue shows its sample and says so. *Люди* switches
  between the chat's members and the person's osu! friends, which the bot
  reads with the person's own osu! link and so needs the `friends.read`
  permission — a person linked before it asked for that is told to link osu!
  again. News, channels and builds are read on the device and kept in
  `~/.dossier/news.json`: the changelog's `json-index` (the `/api/v2` path
  answers Cloudflare's block page to anything but a browser), the news Atom
  feed, and public Telegram channels read from their `t.me/s/` page, the
  list edited under the filters and *@osunewsru* the first. An article, a
  post or a build opens in the reader, not the browser: an osu! article in
  full from the body its Atom entry carries, a post laid out as the channel
  wrote it, every link opening the browser. A post brings its whole album
  and its videos (2026-09-24): the first picture is its cover in the stream,
  every picture stands in the reader, and a video is its still with ▶ and
  its length. Pressing one fetches the file Telegram's page names into
  `~/.dossier/cache/clips` (a `.part` renamed when whole, then kept) and
  plays it in the application's own player over the catalogue — the
  channel's name and the post's first line above it, *Показать в папке* and
  *В Telegram* under it, every key the renders' player knows; a clip that
  is not 16:9 is letterboxed rather than stretched. A video too big for the
  page to carry — most of *@osunewsru*'s: Telegram answers *Media is too
  big* to anyone but its own apps, on the channel's page and the post's
  alike — wears an outward arrow and *В Telegram* instead of ▶ and its
  length, and opens the post. The player's scrim is fully
  opaque: blending is linear, so the old 97 % let a bright page through at
  about a sixth. The subreddit was tried and set
  aside (2026-09-23): reddit answers 403 and 429 to anonymous reading often
  enough that it needs its own key; its reader stays in `news.rs`.
- The person's own figures do not wait for the bot (2026-09-24). The
  application reads the person's osu! page itself — the profile page's
  `data-initial-data`, which carries the statistics, the level, the grades,
  the avatar, the cover and ninety days of rank, and the public
  `/users/{id}/scores/best` list for the best plays with their hits —
  keeps it in `~/.dossier/osu-profile.json` and asks again at most every ten
  minutes. The name is the bot card's, else the person's own entry in the
  group, else the one kept. The bot's card still wins when it answers; this
  one fills the dossier and dresses the sample's *you* with the real avatar,
  cover and figures until it does. The dossier's chart opens on rank, the
  one line osu! keeps by the day.
- The whole catalogue is drawn at 0.84 of its size (2026-09-24): the tabs,
  the stream and the columns are laid out for the width divided by 0.84 and
  drawn scaled, the pointer mapped back through the same scale, so the
  three columns and the dossier's figures fit a laptop's window. Nothing in
  it shows a scrollbar; the wheel and the trackpad scroll it.
- *Люди* is a card per player over their osu! cover — avatar ringed in
  the medal's colour for the first three, name and flag, the title they
  wear in its rarity's colour, the place in the group large at the right,
  pp, global rank and accuracy, and plays, hours and the streak in a foot
  that darkens rather than a rule — in pp order, in a grid
  across the whole width (as many columns as fit at about 420 each), and
  *Титулы* the same way at about 360. A country is drawn as its flag
  everywhere a code stood (osu!'s own pictures, kept in
  `~/.dossier/cache/flags`); until one arrives, its two letters in a small
  frame. A rise or a fall on a board is a small arrow with the number in a
  green or red pill. A card opens that player's dossier in a large panel
  that unfolds over the catalogue (2026-09-24) — the same columns as
  *Профиль*: their osu! page read by the application (level, rank history,
  grades, best plays, avatar, cover) and what the bot keeps of them
  (`/render/community/person?chat=…&id=…`, answered only to a member of
  that group: weeks, days, titles' dates, duels); until those come, what
  the group's list knows. Esc or ✕ folds it back.
- *Рейтинг* is the bot's leaderboard in both of its modes (2026-09-24),
  *Общий* and *Адаптивный*, over the same six boards, chosen on a
  segmented switch that *Люди* uses too for *Беседа* and *Из игры*. *Общий* is the
  standing for all time. *Адаптивный* is the week's gain, as the bot's
  card draws it: only those who gained, the gain in green with the whole
  beneath (*6 396 всего*), a move against where the player closed last
  week (an arrow, *NEW* for no place then, — for the same), *неделя 39 ·
  22–28 сентября*, the count of participants and how many sat out, the
  person's own row telling how far the next place is (*до 16-го места
  осталось 12 pp*) or, if they have not played, pinned below; before the
  week's first snapshot it says the data is still gathering and when the
  first standing comes. The first three stand on a podium — second, first,
  third, the first tallest, each over its cover with its medal's ring and a
  thin edge of its colour; the glow round them is barely there (5 %, 10 px)
  and only grows a little under the pointer (10 %, 14 px), and a chosen
  filter's glow is as faint — the stronger glows (22 %, 18 px) glared; the
  rest are rows of 60 over their covers. The bot sends what the
  app cannot work out: `was`, the places each player closed last week on
  (the snapshot's `prev_positions`), `collecting` and `week_began`. A
  player's cover is fetched once at 720 wide, softened and darkened to 62 %
  before it is ever drawn, and every picture laid under words is shaded
  from 90 % to 98 %: blending is linear, so a lighter shade left bright
  covers glaring through the text. The cover lies 2 px inside its shade and
  the shade 1 px inside the card, so the picture's softened edge never shows
  as a light rim at a rounded corner; *NEW* is an opaque dark green pill.
  When the bot does not send `week_began` (an older bot), the week is taken
  from Monday 00:00 in Moscow of the current week, not from 1970, which read
  as *1–7 января*.

*Хранилище*, a tile in the application's settings (2026-09-24), gives
the size of the application itself (its bundle) and of everything it
keeps, split in a bar and a legend — videos, skins, maps, cache and the
rest — with *Показать в папке* and *Очистить кэш*, which drops the
pictures', flags', clips' and the catalogue's caches (they come back when
needed). A folder's size used to be read with a function that only walked
folders, so the maps' cache, two files, always read as nothing.

The application shares what it read of its person's osu! page with the
bot (`POST /render/me/profile`, kept on their rows as `app_profile`), and
the bot hands it back with that person's dossier (`card`, `card_at`) and
marks them in the group's list (`app`). Another member opening them takes
that card first and reads the osu! page only when there is none or it is
older than three hours — so one player's use of the application serves
everyone who opens them. The feed's filters are solid chips with their
count in a small badge, drawn in opaque colours rather than white at low
alpha. The application is 0.89.4 from here, a pre-release, and says so
beside its build in the settings.

- *Титулы* is the bot's catalogue by rarity, each rarity in the colour the
  bot draws it with, *открыто n из m* for the group, and on every card
  whose faces have it. A secret nobody holds shows *???*.

What it needs from the bot, when it stops being a prototype: the group's
players with the figures above (the `users` table), the board a week ago
(`leaderboard_snapshots`), the titles and who holds them
(`user_title_progress`), top plays (`user_best_scores`) and a log of what
happened — which the bot does not keep yet for renders: a finished render
goes to Telegram and is forgotten. Contests are not drawn: the bot's
bounty and duel columns have nothing behind them any more.

Esc inside the catalogue closes only what lies over it — the player, the
reader, a member's panel — or else clears the search and lets go of a
field; it never closes the catalogue itself (it used to, at once and
without the fold, which read as the page vanishing). In Settings, Videos
and the worker it goes back to the replays through the same fade as the
word *Реплеи*.

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
<folder>` prints what the index made of a folder, `--skins` prints what the
hunt found on this machine and how long it took; `--play N` opens a video,
with `--pause` to stop it and `--ask` to raise the question about deleting
it; `--skin-room` opens the skins window; `DOSSIER_WINDOW=820x570` opens the
window at that size, for the small-window cases. The corpus is the
rehearsal stage.

**One application.** On 2026-09-23 the web application in `app/` (Tauri) and
the Python bridge and terminal worker in `client/` left the repository: the
native application is the whole product now. The worker release that froze
`client/` into `dossier-worker` went with them, with its `packaging/` notes, and
CI checks the engine, the application and the Python scripts under `tools/`.
The faces the application bakes in come from `assets/fonts/`, where the one
weight only `app/` carried, Commissioner SemiBold, was brought across. The
engine's mod pictograms and the tool that drew them went the same day: a mod
is its acronym on a coloured plate, as the game's own badges read.

**Skins are hunted, and an .osk is taken in.** The panel lists the folders it
knows — the clients' own Skins folders, the application's, and whatever was
added by hand. The walk across the machine looks for archives alone (settled
2026-09-23, after a first pass that also collected loose folders and offered
a Desktop with a stray `cursor.png` as a skin): the home folder and what
stands beside it, every mounted volume, four levels deep, skipping the
system's own folders, anything hidden and the heavy ones, giving up after
eight seconds or twenty thousand folders, on its own thread. A folder with
`skin.ini` in it is not a find; an `.osk` is, and it is taken in at once —
unpacked into the application's Skins folder, where it becomes an ordinary
skin. `--skins` prints what the hunt found and how long it took.

An `.osk` is a zip, and the engine already knew how to open one
(`skin::unpack`, which refuses entries that climb out of the folder and keeps
everything else, nested folders and all). One dropped on the window, or a
skin folder dropped on it, is taken at once; *Добавить скин…* asks for a file: an `.osk`, or
the `skin.ini` of a folder, which stands for the folder it sits in — no one
dialog offers files and folders at the same time, and the archive is what
could not be reached before (fixed 2026-09-23, when a folder-only dialog left
an archive impossible to add by hand). An archive wrapped in a single folder is lifted out of it and an
archive already unpacked is left alone.

A skin is shown the way the engine will draw it, not by blowing up one file:
the preview asks `Sprites::read` for the same elements the renderer asks for,
so a skin.ini prefix, an animation's first frame and the `@2x` pair are
honoured, and whatever the skin does not carry is drawn by the application
as the engine draws it. The strip in the settings shows one circle per skin
in a row that scrolls sideways with no bar under it, so the tile grows wide
and never tall; the first cell is the engine's own skin, *Dossier Default*.
*Подробнее…* opens a window of panels, one per skin, each holding the same
prepared pattern, laid out in the playfield's own coordinates (settled
2026-09-23): four circles in a row with the approach circle on the last, one
long slider and one short one, both straight — a dark body inside the skin's
own border colour, the head numbered and the tail plain, as in the game, each circle
chosen as the engine chooses it: `sliderstartcircle` and `sliderendcircle`
when the skin speaks for them, a blank one meaning nothing is drawn, the
hit circle otherwise, and no circle at all at a tail when the skin says
nothing of circles; the overlay above or below the number as `skin.ini`
asks; the cursor with its `cursormiddle`, both at their own size — and
the cursor going up the left side, with its trail behind it only when the
skin carries one, since a trail the application invents says nothing about
the skin; so two skins can be told apart at a glance; choosing one there chooses it everywhere. The circle is tinted only
from the skin's own `Combo1`; tinting every skin with osu!'s default orange
made them all the same mustard blob (tried and dropped 2026-09-23). The live
replay on the main screen is drawn with the chosen skin too, and it starts
again the moment the choice changes.

**How the video plays is a tile of its own** (added 2026-09-23): *Игровой
процесс* holds the background's dim and blur as two bars — the dim as the
share the map's artwork is darkened, 82 % by default as the engine's own, the
blur as a share of the engine's own softening — and four switches: the
interface, the cursor answering presses (the engine's `cursor_expand`, still
subject to the skin's own `CursorExpand`), the map's own hitsounds and the
skin's. All of it goes to the render only; the live replay keeps its own
look. *Звуки карты* off means the mapper's sound design is set aside as a
whole, not only its custom samples: every hit is the skin's plain
`normal-hitnormal` at the map's loudness, with no whistle, finish or clap and
no sample set of the map's choosing, the slider's slide and ticks in the same
normal bank (sharpened 2026-09-23, when turning the map's sounds off still
left its claps and whistles playing in the skin's voice). The skin's own
sounds are read whole from its folder — every gameplay sample it carries;
the menu's sounds and numbered variants are left, as osu! leaves them in a
skin. `dossier --render <replay>` now renders as the application would, with
the saved skin and the saved gameplay choices, `DOSSIER_RENDER_HEIGHT` and
`DOSSIER_RENDER_FPS` making it quick.

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
