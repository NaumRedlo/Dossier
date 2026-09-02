# The Dossier client, with a face

The terminal worker in `../client` claims a job from the bot, fetches what it
needs, runs the engine and sends the video back. This is that, in a window, and
it replaces it rather than wrapping it.

Two things are different by construction rather than by decoration:

- **The engine is linked in, not shelled out to.** "Is the engine there?" and
  "do the builds agree?" were the two commonest ways somebody's first evening
  went, and neither question can be asked of an application that *is* the
  engine. They are gone from the readiness list, not answered faster.
- **One file.** No Python, no virtual environment, no `pip install` — the thing
  people were asked to do before they could lend a machine.

## Its own workspace

`Cargo.toml` here opens a workspace of its own. Tauri brings several hundred
crates with it, and inside the engine's workspace every `cargo test` would have
to look at all of them — the engine's tests are run dozens of times a day. The
path dependency reaches the engine from here and the traffic does not go back.

## Running it

```
cargo run           # from this directory
```

No Node and no `tauri` CLI: the interface is static files under `ui/`, and
`withGlobalTauri` puts `window.__TAURI__` on the page without a bundler. The CLI
is only wanted for building installers, which is a later problem than making the
thing work.

## What is not ported yet, and why it matters

**The machine's own policy.** `client/dossier/machine.py` is five hundred lines
that decide whether this computer should be rendering at all: the battery, the
thermal pressure, whether somebody is at the keyboard, the hours its owner
allowed. Every one of them is there because a laptop that renders flat out on
battery while its owner is typing is a bad guest. Until it is ported,
`work::provisional_capacity` says what it knows and nothing it does not — and
**the loop must not be offered as a button somebody can leave running.**
`work_once` does one job and stops, which is as far as that should go.

**Fetching a map nobody has.** The bot sends what it found when it drew the
card, and the terminal client downloads the beatmap when the machine does not
have it. Here a job whose map is not in the songs folder is handed straight
back, which is honest and not yet useful.

## Where it is

The readiness screen, which is `--check` with the two dead questions removed.
Everything else — claiming work, rendering, settings, the skin catalogue — is
still in `../client`, and moves here one piece at a time. Until it does, the
Python worker is the one that renders.
