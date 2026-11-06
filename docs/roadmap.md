# The road to Dossier 0.100.0

Drawn up on 6 November 2026. Problems found and compromises taken live in
`docs/debt.md`; labels like S1 and P1 point there. Criteria M and V point to
section 23 of `docs/duels.md`.

## How every release is built

Reworking what exists is more dangerous than adding something new: every change
to old behaviour touches the people used to it and the data already on their
disks. So every release has five parts:

1. **A theme.** One per release. Everything outside the theme, except fixes,
   waits for its own release.
2. **Tasks by priority:**
   - **must**: the release does not ship without it;
   - **should**: done once the musts are ready, otherwise it moves to the next
     release as its first item;
   - **if time allows**: moved on without regret.
3. **What changes in existing behaviour.** Every change to old behaviour on its
   own line: what the player sees, what happens to their settings and files, how
   to roll it back.
4. **Bot compatibility.** The server is deployed first and accepts both the new
   and the previous app version. New fields are optional. A feature that needs
   the new server stays off until the server announces it.
5. **Debt.** What the release pays off from `docs/debt.md` and what new debt it
   takes. New debt is written down in the same commit where it appears.

Checks before every release:

- CI is green on the tagged commit; the release build waits for it and refuses
  otherwise;
- the CI advisories job (`cargo deny check advisories`) finds no new
  vulnerabilities;
- gallery frames through `wgpu` at 980 and 1920, in Russian and English, looked
  at by eye;
- for memory, cache and network changes: a long session at a tester's;
- for Witness and overlay changes: a Windows check by testers following
  `docs/windows-check.md`.

Rules between releases:

- A release number is never used twice. A fix after a release gets the next
  number (P2).
- Release notes in `docs/releases/` are written as work goes.
- Big screen files are split only together with work on that screen and without
  changing behaviour, in a separate commit (C1).
- Nothing is taken away from players: not titles, not badges, not saved
  settings. If a rule changes, what was already earned stays.

## Order of releases

| Release | Theme | Why now |
|---|---|---|
| 0.98.0 | Fixes, holes and foundation | Ready fixes, vulnerabilities, CI and a checked ffmpeg; without them every next release is risky |
| 0.98.1 | Library control | The most frequent everyday complaints: deleting, keys, menus |
| 0.98.2 | Renders through the bot | The farm already works for every chat, and speed-ups show at once |
| 0.98.3 | Overlay | Witness is in use by testers, and the plates' look is unfinished |
| 0.99.0 | Profile and titles | Cheap, on data we have, visible to everyone |
| 0.99.1 | The first real duel | A closed test on the server |
| 0.99.2 | Community interactions | People already see each other; time to give them actions |
| 0.99.3 | Balance desk and Kamerton | A catalogue that can be trusted, and a skill profile on it |
| 0.99.4 | Play breakdown and video | Rests on the difficulty graph and the two-player engine |
| 0.100.0 | Duels for everyone | Rated duels and a line under the foundation |

0.97.9 was planned as a release of its own; on 7 November 2026 the owner
decided to ship its work together with the foundation as 0.98.0. The number
0.97.9 is never released.

## 0.98.0. Fixes, holes and foundation

No new features.

Done:

- avatars in Community no longer go missing, covers in the replay feed are not
  lost, no light line under the score sheet's head, a name in the video
  leaderboard no longer runs under the rank number;
- TLS crates updated, `zip` 9; updates and map archives unpack through one
  bounded function in `unzip.rs` that refuses links and paths outside the
  folder. The 0.97.8 archive from GitHub unpacks and installs with this code;
- links open through one function, `links.rs`, which passes `https`, `http`,
  `mailto` and `osu`; a test keeps every other `open::that` call to local places;
- the release build signs the whole `Dossier.app` and checks the signature (it
  survives packing and the updater's unpacking);
- CI jobs have time limits, package installs are retried up to three times;
- CI runs on every push to `continuum`, with an advisories job, and a release
  builds only a commit whose CI passed;
- ffmpeg 9.0.2 is downloaded only from pinned version links and refused when its
  SHA-256 differs; Windows has two sources with one hash;
- in the bot: a device renders only its owner's replays and those of chats its
  owner is in (S1, waiting for deployment).

Must, still open:

- S1 deployed on the server.

Should:

- P4, P5: gallery scenes with covers in the feed, cache tests, a review of the
  outdated subscription goldens;
- C2: one wrapper for a picture under a veil.

If time allows: a long session memory measurement and cache limits based on it.

What changes in existing behaviour:

- Unpacking updates. The most dangerous place of the release: if it is wrong,
  nobody can update. Check updating from 0.97.8 to 0.98.0 on a copy before
  publishing. Rollback: the 0.97.8 archive stays in its release.
- Links: only `https`, `http`, `mailto` and `osu` open (*Open in osu! through
  osu!direct* opens `osu://b/…`).
- Map archive limits leave room for sets with video; a refusal is explained in
  words.
- On macOS a downloaded copy is no longer called damaged; instead the first
  launch asks in System Settings, Privacy & Security. Updating from inside the
  app does not ask.
- Downloading ffmpeg. An ffmpeg already downloaded stays. A new player gets it
  from a pinned source; evermeet.cx is no longer used because it stopped
  serving files.
- The C2 wrapper moves the bottom of picture headers by 1–2 pixels. Compare
  frames before and after for every such header.

Bot compatibility: S1 is deployed before the release. A chat where no member has
a worker waits for a render up to the common 15 minute limit. Everything already
in the bot's `continuum` goes to the server with it.

Debt: pays off S1 (deployment), and if done P4, P5 and C2. The iced update (C3)
is not taken: our copy of `iced_wgpu` makes it a separate piece of work.

## 0.98.1. Library control

Must:

- **Deleting replays to the trash**, one at a time and in batches. A batch is
  selected with Shift and Cmd (Ctrl on Windows and Linux) on cards or as a whole
  group. Before deleting, the number of files and their size are shown.
- **Keys in the replay list:** Enter renders the selected replay, Delete and
  Cmd+Backspace delete, Cmd+A selects the group, Esc clears the selection.

Should:

- **A card menu on right click:** render, open the folder, show the map in osu!,
  delete.
- **A key list:** `?` opens the list of chords for the current screen.
- **Sections on Cmd+1…4:** Replays, Videos, Community, Settings.

If time allows: **first launch** as one screen of steps: find osu!, download
ffmpeg, link the bot, decide about Witness.

What changes in existing behaviour:

- Replays from stable's folder (`Data/r`) are tied to local scores. A deleted
  replay disappears from viewing in the game itself. The confirmation says so.
- Files inside lazer's storage are never touched. Only exported replays and
  replays from folders the player added can be deleted.
- Deleting goes only to the trash, never for good. A file held by a running game
  (on Windows) is not deleted. The dialog says which files stayed and why.
- Enter and Delete are wired today as pool keys (`Message::PoolHot`) and work
  only in *Pools*. They have to become the screen's shared keys, otherwise every
  next chord must be checked in every section. The pool key tests must pass
  unchanged.
- Batch selection changes what a click with Shift and Cmd does on a card. A
  plain click works as before.

Bot compatibility: a deleted replay that already went into the bot's history
stays there. The app asks the server to delete nothing.

Debt: a context menu widget of our own, since iced has none. Write it down under
C if it ends up serving only the replay list.

## 0.98.2. Renders through the bot

Must:

- switches in `sts`: the PP counter, the difficulty graph, the leaderboard;
- faster preparation: the worker keeps the map sets a chat plays most often and
  starts fetching a map while the job waits in the queue. Goal: under 10 seconds
  from `.osr` to drawing for a map the worker already has;
- compare the video's length with the replay's before posting it (S1).

Should:

- mirrors by quality: count refusals and stale archives, ask the best one first;
- the bot's message shows the render's stage.

If time allows: the video's Telegram cover taken from the play's most intense
moment.

What changes in existing behaviour:

- The worker's set cache takes a tester's disk space. A limit in the settings,
  modest by default.
- The mirror order floats. Checking the difficulty by md5 (since 6 November)
  guards against an old map version from another mirror.

Bot compatibility: the switches are stored on the server and reach the worker in
the job. An older worker ignores the new fields.

## 0.98.3. Overlay

Starting point, the `dossier-hud` sample frames of 6 November: text about 11 px
at 720p, almost no gaps between words (*Witness подключился* runs together), and
plates in different corners with no shared grid.

Must:

- **A canvas mockup before code.** Plate variants over game frames; the app then
  repeats the chosen one exactly.
- **Legibility:** text size from the game window's height, checked on 1080 and
  1440 frames; fix the gaps between words in the overlay's font.

Should:

- one grid of places for the plates: notices, PP and UR, the chat leaderboard,
  sending with Tab;
- plates appear and leave in place, as in the app.

If time allows: an overlay preview in the app next to the size and plate settings.

What changes in existing behaviour:

- The saved size (70–130%) and plate choice carry over as they are. The chords
  Ctrl+Shift+H, = and − do not change.
- The new look changes what testers got used to since 30 October. Before the
  release, show the frames to the tester whose overlay works in full screen.

Bot compatibility: no server changes.

## 0.99.0. Profile and titles

Must:

- **Rebalance titles by the share of holders.** First count on a copy of the
  database what share of players holds each title, then adjust rarity. Check with
  `scripts/recheck_titles.py` without `--apply`.
- **The share of holders** next to the rarity: "held by 4% of players".
- **Closest titles:** the three nearest ones, with what is left to the goal.

Should:

- a showcase of up to three titles on the card;
- form over weeks: PP, accuracy, play count, a short trend mark;
- a pinned play with its video on the profile card.

If time allows:

- the player's goals and how far they are;
- head-to-head in comparison: shared maps and the score on them;
- the bot's profile card looking the same as the app's.

What changes in existing behaviour:

- The rebalance changes rarity colours on cards. No title is taken away. If a
  condition becomes stricter, those who earned the title keep it.
- A three-title showcase changes the profile card's height. Check the card in the
  bot and in the app with long names.

Bot compatibility: the share of holders and the closest titles are computed on
the server. An older app shows one title as before.

## 0.99.1. The first real duel

Must:

- the bot runs Bancho rooms under the NaumRedlo account;
- `scripts.duel_probe` at levels 4.5, 5.5, 6.5 and 7.5, with `--human-only` and
  without; write down the slots whose corridor has fewer than five maps;
- a closed test for two: an ordinary duel, giving up mid-map, closing osu!,
  restarting the bot during a map, a closed room. Every case ends with a result
  or a cancellation without rating.

Should: watching through Witness, and differences from the official match in the
duel's log.

What changes in existing behaviour: nothing for players without duels. The
section is closed to everyone except the test's participants.

## 0.99.2. Community interactions

Must:

- **Following a player:** a notice under the bell about a record, a title or
  starting to play. Only with the consent of the person followed.
- **Chat titles:** shared weekly goals; every member gets the title.

Should:

- one reaction to a play in the feed, no comments;
- actions with pools: rate, copy, suggest a map for someone else's pool;
- the chat's best moments of the week: a reel of `dossier-exhibit` clips.

If time allows: watching a friend's play live through Witness, if the friend
turned showing on.

What changes in existing behaviour:

- A new consent switch for being followed, off by default. The feed and *People*
  work as before.
- The weekly reel loads the farm. It is built at night and only for chats where
  it is on.

Bot compatibility: everything is stored on the server. An older app simply does
not show reactions and following.

## 0.99.3. Balance desk and Kamerton

Must:

- a review queue ordered by usefulness;
- double labelling of 10% of maps;
- the M04 and M05 report per slot.

Should:

- **a skill profile from Kamerton**: a player's strong and weak skills from their
  plays on labelled maps. Only in slots where auto admission is already open,
  otherwise the profile would lie;
- the analyser: rhythm, triplet patterns, sliders, the hardest stretch, mods,
  similar maps.

What changes in existing behaviour: changing `FEATURE_VERSION` retrains the model.
Maps keep their old analysis until the new one measures them again, so duels are
not cancelled during the recount. Afterwards the server needs `reanalyse`,
`train` and publishing the model.

Done when: every working slot has at least 30 checked maps and auto admission is
open in at least six of nine slots.

## 0.99.4. Play breakdown and video

Must: **a breakdown in the score sheet**: where accuracy was lost and where the
misses were, from the judge's events next to the difficulty graph.

Should: **a ghost**, two replays on one field: your own and someone else's record.

What changes in existing behaviour: the score sheet gets taller. Check it at 980
and 1920.

## 0.100.0. Duels for everyone

Must:

- M10: 1000 generations on a copy of the catalogue, at least 95% successful pools;
- V07: the restart test in every state of a duel;
- Glicko-2 calibration on the first duels;
- no high severity entries in `docs/debt.md`.

Should:

- duel titles: the first win, a comeback from 0:2 to 3:2, a tiebreaker win, a win
  over an opponent rated much higher;
- S6: a signed `SHA256SUMS`.

What changes in existing behaviour: the *Duels* section opens to everyone. The
first duels' rating is a calibration, which is explained before the first duel.

Done when: under 5% technical cancellations over two weeks of open duels.

## For the owner to decide

- An Apple Developer ID at 99 dollars a year.
- What the subscription gives besides the map background, the badge and the
  title. Proposed, nothing accepted: profile themes, an avatar frame, an animated
  banner, renders ahead of the queue, 1440p, more personal skins, videos without a
  length limit. Only cosmetics and convenience, never an advantage in play or duels.
- The device token in the system keychain (S8).

## After 0.100.0

Not to be started until the owner returns to the topic:

- the settings redesign (canvas https://claude.ai/artifact/EPQRef8pas4pjGban5A6Bx):
  panels of one height, thick value bars, experiments with the sound panel;
- rebalancing the duel rating after the first month of open duels;
- tournament pool archives on the server with their own categories;
- the overlay through tosu for full screen OpenGL (`docs/tosu-counter.md`);
- Kamerton learning from play results, a pretrained map encoder;
- the iced update and splitting big screen files where ordinary work did not
  touch them.

For the indefinite future (put aside by the owner on 7 November 2026):

- duel ranks. Proposed: a rank by rating thresholds without divisions, a 30 point
  margin before falling, no rank until calibration ends, no season resets at the
  start. Names to choose from: tempos (Largo … Presto, Prestissimo for the first
  10 places), chess-style classes, or "League 1…7". Nothing chosen.
