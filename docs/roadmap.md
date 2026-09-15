# Where the engine goes next

Written 2026-08-11, rewritten 2026-08-27 and again 2026-09-09, from the state
the engine is actually in rather than from where it was meant to be. Two
sibling documents say what was decided and why — [`stable-fidelity.md`](stable-fidelity.md)
for judgement, [`exhibit.md`](exhibit.md) for selection — and this one only says
what is left.

The September rewrite exists because the shape of the project changed rather
than because the list got shorter. There is a desktop application now, and it
is how most people who lend a machine will meet this. That moves the hardest
remaining problems out of the engine and into how a stranger joins, what they
are allowed to change, and what they get to see.

## Where it stands

Nine crates, about 34,500 lines of Rust in them, 826 tests. The desktop
application is 5,500 lines more in its own workspace; the Python that drives the
engine from a terminal is 3,300 in `client/`, and is the same code the bot and a
render worker both run.

| crate | lines | what it is |
|---|---:|---|
| `dossier-replay` | 1,152 | `.osr` parsing |
| `dossier-beatmap` | 2,345 | `.osu` parsing, slider geometry, storyboards |
| `dossier-sim` | 4,680 | judgement, scoring, health — the part with a right answer |
| `dossier-assay` | 3,082 | what a play was worth, told back in numbers |
| `dossier-render` | 9,944 | frames, skin, elements, the HUD |
| `dossier-audio` | 1,987 | hit sounds |
| `dossier-produce` | 4,543 | the scene: finding the map and skin, drawing, encoding |
| `dossier-exhibit` | 1,224 | which seconds of a play are worth watching |
| `dossier-cli` | 5,527 | the commands |

Measured 2026-09-09:

```
111 exact of 187 (11 lazer), total count error 188, 3 skipped
score compared on 183, worst 19.89%, within 0.5% on 166
```

322 to 188 in eight days, almost all of it one subject: the spinner was counted
in whole rotations where `osu!.exe` counts half of them. The reading is in
`stable-fidelity.md` under 2026-09-09, along with the one outlier left and the
tempting explanation for it that measurement rejected nine-to-one.

## The thing to do first, because it is a liability

**The published release contains a font that is not ours to publish.** Torus
Notched is © 2018 Paulo Goode, all rights reserved; `release.yml` copied it into
every zip, and `v0.11.0` is on the releases page carrying it. That is now fixed
in the tree — Huninn replaces it outright and the workflow ships Huninn and
JetBrains Mono with their OFL texts — but a fix in the tree does not unpublish a
zip.

So: **cut a release from the fixed tree, and take the old assets down.** It is
the same class of problem as the CC BY-NC mod badges that went this morning, and
it is the only item on this page with a clock on it.

Worth knowing and deciding separately: the file is out of `HEAD` but still in
history, and getting it out of history means rewriting it and force-pushing a
public repository. Not something to do casually, and not something to do without
saying so first.

## Joining the farm without a token

Today a person who wants to lend a machine fills in three fields: the bot's
address, **a token**, and a name. The token is the whole problem. Somebody has
to issue it, somebody has to carry it to the machine, and it lands in a
plaintext file — `~/.dossier/worker.env` — where it gets pasted into chats when
help is asked for, photographed in screenshots, and committed by accident. It is
also unrevokable in practice, because nobody knows which machine holds which
token.

The fix is not a better token. It is not having one.

### The shape

A pairing flow, the same one a television uses to sign into an account, with one
change that matters (below).

1. On first run the application generates a keypair. **The private half never
   leaves the machine** — OS keychain, not a file: Keychain on macOS,
   Credential Manager on Windows, the Secret Service on Linux.
2. It asks the server to pair: the public key, the machine's name, its OS, its
   cores, its engine build. The server answers with a short code —
   `K7-QN4M` — good for five minutes, usable once.
3. The application shows that code, large, with a QR beside it that opens the
   bot chat with the join command already typed.
4. The person sends it. The bot answers with a card describing **the machine**:
   *«Добавить „MacBook Pro Наума“ — macOS на ARM, 10 ядер, сборка 0.11.0?»* and
   a button.
5. On yes, the server binds that public key to the account. The application's
   next poll comes back approved.
6. From then on the worker **signs** every request with its key rather than
   presenting a secret. There is nothing to copy, so there is nothing to leak.

Steps 1 and 6 are what make it safer than what we have rather than merely
nicer. A bearer token is a secret that must travel; a signature is a
demonstration that does not.

### The one change that matters

Step 4 must describe the machine, not just ask for a yes.

The known attack on every code-pairing flow is that the attacker starts the
pairing, gets *their* code to a victim — "hey, can you approve this for me" —
and the victim's yes attaches the attacker's machine to the victim's account. A
confirmation that says only «Добавить устройство?» cannot be defended against
that. One that names a Windows PC with four cores, to somebody who is sitting in
front of a Mac, is defended by the person reading it.

That is the entire mitigation and it costs a sentence in a message. It is worth
writing down here because it is the sort of thing that gets dropped as decoration
when the implementation is running late.

### The rest of what safe has to mean

- **Codes**: six to eight characters from an alphabet with no `0`/`O` or `1`/`I`,
  single use, five minutes, rate-limited per account and per address.
- **Scope**: a worker key may claim jobs and upload results for jobs it claimed.
  It is not an account credential and must not be able to read anybody's
  replays, or anything at all it did not ask to render.
- **Revocation**: `/workers` lists devices by name and when each was last seen;
  one tap removes one, and removal bites immediately because every request is
  checked rather than trusted once.
- **Still pull-only.** No inbound port, no reachable address, nothing to
  portforward. That property is worth more than it looks and none of the above
  costs it.
- **Rotation**: the signature is over a nonce and a timestamp, so a captured
  request is not a reusable one.

### How it looks from the application — settled 2026-09-15

The bot already hands out a code (`invites.py`: eight letters from an
alphabet without `0`/`O`/`1`/`I`, ten minutes, single use) and `/render/join`
turns it into a worker token. The application's first run turns that around
so nobody types anything: it asks the bot for a code, shows it with a QR that
encodes `https://t.me/<bot>?start=pair-<code>` and an *Open Telegram* button
for the same link, and polls until the bot says the person pressed yes on a
card that names the machine. Three things on the bot's side: `POST
/render/pair` (name, OS, cores, build → code), `GET /render/pair/<code>`
(waiting, or linked with the token), and `/start pair-<code>` showing the
confirmation card. The bearer token stays the credential for now and moves
into the OS keychain; the signing key from the steps above is the next move,
not a precondition. `docs/design.md` has the screen.

The bot's three are done, the same day. `POST /render/pair` takes `name`
(required — the card has to name something), `os`, `cores` and `build`, and
answers `{code, link, expires_in}`; `link` is the full `t.me` URL when the bot
knows its own username, and empty when it does not, so the application should
be able to build it itself. `GET /render/pair/<code>` answers `{status:
"waiting"}`, then `{status: "linked", token}` exactly once, then 404 — the
token is written only at that collection, so a yes nobody collected leaves no
row behind. Guesses are refused at twenty misses a minute per address, a real
poll is never counted. The card is refused outside a private chat, because a
yes button in a group belongs to whoever presses it first.

### What it costs, and who pays it

Three endpoints and a command on the bot's side, a keychain dependency and a
screen on this side. **The engine is untouched** — this is entirely application
and bot, which also means it can be built without waiting on judgement work.

`RENDER_WORKER_TOKEN` stays valid for headless machines: a VPS has no window and
nobody at the keyboard to press a button. The application simply never writes
one again, and a machine that already has a token can register a key on its next
poll and stop using it.

## Plugins, and the line they may not cross

The rule first, because everything else follows from it:

> **A plugin may change what is shown and what is produced. It may never change
> what is judged.**

The engine's only real claim is that its judgement matches the game's, and
`stable-fidelity.md` is five thousand lines of measuring exactly that. A plugin
that can reach into judgement makes every number in that document meaningless.
Worse, on a render farm it would mean running a stranger's judgement code on a
volunteer's laptop.

### Three tiers, in the order they should be built

**Tier one — a file, no code.** The HUD's positions and sizes are constants in
`hud.rs` today: `SCORE_SIZE`, `COMBO_OF_SCORE`, `EDGE_MARGIN`, `PROGRESS_RADIUS`
and a dozen more. Lift them into a layout a person can write — which readouts
exist, where each anchors, how large, in what face and colour — and most of what
anybody actually asks for is answered: a minimal HUD, a stream overlay, a bigger
combo, the counters somewhere else. No security surface at all, testable like
anything else, and it turns those constants from magic into documentation.

The same shape fits `exhibit`: its scorers already produce numbers and reasons,
and the weights want to be a file rather than a rebuild.

**Tier two — WASM, and only if tier one runs out.** If somebody wants to *draw*
something the layout cannot describe, the plugin gets the frame's facts — time,
combo, score, cursor, what is on screen — and returns drawing commands.
`wasmtime` or `extism`: no filesystem, no network, no syscalls, and a fuel limit
so a bad plugin cannot hang a render instead of finishing it. That is the only
form in which running a stranger's code on somebody else's machine is
defensible.

**Never: native plugins.** Rust has no stable ABI, so every toolchain bump would
break every plugin built against the last one; and a `.dll` passed around a chat
and loaded by a render worker is malware distribution with extra steps.

### Reproducibility comes first, not after

Two workers with different plugins must not silently return different videos for
the same job. The build stamp already refuses a worker whose engine does not
match; plugins need the same answer before they ship, not after somebody
notices. The likely shape: **a bot render fixes its plugin set in the job**, and
a worker that cannot satisfy it hands the job back rather than improvising.
Local renders may do as they please.

## Things worth having that need almost nothing new

Ordered by how little is left to do rather than by how much anybody wants them.

- **A replay card.** One PNG: player, map, mods, accuracy, combo, rank. The mod
  badges are drawn already, the faces are loaded already, the layout is the only
  new part. It is the obvious thing for a chat to post beside a video.
- **The reason burned into a reel.** The JSON has said why each clip was chosen
  since the beginning; writing it on the frame was deferred until the selection
  could be trusted. It now can.
- **Slow motion back on for reels.** `reel::SLOW_INTO_A_MISTAKE` is one line.
  What it needs is not code but a judgement about whether it reads as an effect
  or as a bug, and that is ten minutes of watching.
- **A vertical cut.** The camera already follows the cursor with a closeness
  parameter. A 9:16 frame that tracks the play is a `Layout`, not a feature, and
  it is the shape everything is watched in now.
- **The hit-error graph as a picture.** `dossier errors` computes where hits
  fall around the windows; drawing it is a chart nobody has to read a table for.
- **Where the drain would have killed it.** `dossier health` knows; the
  application's seek bar is right there.
- **Ready, three, two, one, go.** Skins carry `ready`, `count3`, `count2`,
  `count1` and `go` as pictures, and the engine reads skin elements fluently
  now. The progress ring already counts the lead-in down; this is the same
  moment, said out loud.
- **Two replays on the same map, side by side.** Named in August, still true,
  and the scorers are shared.
- **A skin's own screen** — four full-size frames per skin rather than a
  thumbnail, which is what somebody choosing between two similar skins is
  actually comparing. The application has the page pattern for it now.
- **The six drawings nobody has placed.** `bad`, `meh`, `good`, `thumbsup`,
  `thumbsdown` and `bell` came in with the mod badges. A bell wants
  notifications; the faces want to sit beside an accuracy.

## Still open from before, and still true

**The lock on the 37% replay.** The single open question in
`stable-fidelity.md`: the lock suppresses roughly as many clicks as that player
genuinely missed, and it is unknown whether it is right or coincidentally right.
Identifiable by its header rather than its name — **609/600/177/843, combo 422**,
2229 objects, on `6e7f6f08671ad9a9d2fa079665d8d443`. Nothing in the corpus
matches it.

**Hit-window boundaries.** 47 of 48 disagreeing replays disagree by no more than
the hits sitting within 2ms of an edge, and the direction splits 27 against 22.
That is rounding, and the replay does not carry the digit that would settle it.

**Write the baseline, then back the corpus up.**
`dossier corpus --expect tools/corpus.tsv --update-expect` records what is on
disk as the new baseline; the replays themselves live outside this repository
and losing them once already cost months.

**The background is off by default.** `--background` works and costs nothing per
frame; the bot does not pass it. Turning it on changes how every render looks,
which is a decision rather than a flag.

**`classic` is not as classic as it claims** — a flat slider body where the game
draws a gradient. A fidelity gap in the skin that is *about* fidelity.

**Stringly-typed errors in the CLI**, and **edition 2021 on a 1.97 toolchain**.
Hygiene. Now that CI actually runs on pushes again, an edition bump is a change
that gets checked rather than taken on trust.

## Order

1. **The release.** The zips on the releases page carry a font that is not ours
   to publish. Everything else on this page can wait a week; this cannot.
2. **Agree the pairing flow with the bot.** It is a two-repository change and
   the bot's half is the larger one, so the design wants settling before either
   side starts. Nothing in the engine blocks it.
3. **The HUD layout file.** Tier-one plugins, and the piece the other tiers get
   designed against once it exists.
4. **The replay card**, because it is nearly free and it is the thing the chat
   sees most often.
5. **The baseline, and a copy of the corpus somewhere else.**
6. Then the open fidelity question, and the exhibit features — one asks whether
   the engine is right, the others are about what it shows.
