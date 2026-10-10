# Witness overlay foundation

The foundation is `crates/dossier-overlay`: a shared telemetry protocol, a bounded
reader, a latest-state mailbox and a rendering-host contract. The existing stable
reader supplies it through `crates/dossier-witness/src/overlay.rs`.

This does not yet draw inside either client. No renderer hook, DLL loader, C ABI,
lazer component or lazer telemetry reader is implemented by this change.

## Trying the stable source

Build Witness for Windows as usual. Beside stable, on Windows or in its Wine
prefix, run `witness.exe --overlay`. This is an explicit telemetry-only mode:
stdout contains only overlay JSON lines, up to ten snapshots per second. It does
not record or transmit replays. The existing `--serve` mode and Dossier's normal
launch stay unchanged. `--leash` can be used with either mode.

The consumer example reads that stream:

```
cargo run -p dossier-overlay --example monitor
```

Pipe Witness's stdout to the example's stdin on the same machine (or run both
Windows executables in the prefix). The example prints decoded telemetry; it is
not a graphical overlay. A lost pipe ends the receiver lifetime. Production hosts
must read on a worker thread, clear their mailbox on EOF/error and never perform
pipe reads on a game/render thread.

## Protocol version 1

Each UTF-8 JSON line is at most 16 KiB including its newline. Every frame contains
`event: "overlay"`, `version: 1`, a nonzero `session`, an increasing `sequence`,
and a `kind`. Unknown versions and invalid numeric values are rejected.

- `hello`: `client` (`stable` or `lazer`), `pid`, `build`. Required before data;
  starts an empty view. Session numbers increase on client reattachment within
  the same producer process.
- `snapshot`: `snapshot.screen`, optional `beatmap`, optional `gameplay`, and
  nullable `watching_replay`. Every snapshot replaces the previous one completely.
- `disconnected`: clears the view and closes that session. More data requires a
  new hello with a higher session number.

Reset the receiver for a new transport/producer process; its session counter may
restart. Do not join independent producers onto one receiver. Sequence gaps are
valid, duplicates and older/foreign sessions are not. Invalid or oversized input
closes the transport; the bounded reader does not attempt to recover framing.

Gameplay includes ruleset, map time in milliseconds (negative lead-in is valid),
score, current and maximum combo, misses and optional accuracy in percent.
`legacy_mods` preserves stable's bitmask. `mods` supports lazer acronyms with
per-mod settings, including custom clock rates. Stable only computes accuracy
for osu!standard; other rulesets report it as unknown rather than applying the
standard formula. No replay frames, account tokens or process addresses cross
this protocol. Map text is capped by the stable publisher.

Three values were added on 10 October 2026 without a new version, all optional:
`gameplay.unstable_rate` (ten times the spread of the hit errors, at the played
speed, from two hits on), `gameplay.resting` (the map is in its lead-in, a break
or its outro; unknown when the map's file could not be read) and
`snapshot.meter` with `shown` and `scale` (the client's hit error meter, as its
configuration said when Witness attached; a change made in the client's settings
is not seen until the next attach). The rules live in `dossier_overlay::live`.

Two more followed the same day, again optional and only for osu!standard:
`gameplay.pp` is the play so far, valued over the part of the map that has been
judged (the difficulty of the map cut at the last judged object, the hits and
the best combo as they stand), and `gameplay.pp_clean` is the whole map with a
full combo, the misses turned into hits and the hundreds and fifties kept. Both
are worked out by `dossier-assay` in `crates/dossier-witness/src/pace.rs`, as a
classic score without its total. The value so far is worked out again when the
hits change, but not more often than every quarter of a second; one calculation
took at most 6 ms on the maps it was timed on. A map whose file cannot be read
has neither value.

`Mailbox` holds one latest snapshot. Its render-side `try_view` never waits for
its mutex: if the receiver is busy, the host gets no view for that frame. Data
expires two seconds after receipt, even if no explicit disconnection arrived.
Hosts must clear/hide their widgets on an absent view. A stalled stdout reader
can delay the telemetry producer; it cannot create an unbounded mailbox. Pipe
writing is synchronous in this opt-in development mode.

## Integrating inside the clients

`Host::draw` receives the current view and a viewport in physical pixels plus a
UI scale. The host owns the game graphics context, runs draw on its proper thread,
handles window resize/device recreation, and calls `release` before unloading.
No IPC, disk/network I/O or waiting belongs in draw. This initial HUD is passive;
there is no input capture or game-control command in the protocol.

- **stable:** implement the in-process graphics host and its lifecycle separately;
  feed it the stable telemetry stream. Renderer/API detection and loading still
  need real Windows/Wine validation. The Witness memory reader stays independent
  of the graphics host.
- **lazer:** implement a client-side component and telemetry adapter with the
  same messages. Use osu!framework's update/draw separation, not stable memory
  offsets or its mods bitmask. `Client::Lazer` and the round-trip tests establish
  the data contract only, not compatibility with an unmodified lazer release.
  See the upstream [threading model](https://github.com/ppy/osu-framework/wiki/Threading)
  and [OsuGame](https://github.com/ppy/osu/blob/master/osu.Game/OsuGame.cs).

Next integration tests must cover fullscreen/windowed rendering, DPI/resize,
context loss, client restart, a stalled/disconnected transport and host unloading
without affecting gameplay. Protocol tests cannot establish those properties.
