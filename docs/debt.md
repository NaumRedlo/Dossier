# Debt register and audit findings

Kept since 6 November 2026. Everything left unfinished on purpose and everything
the audit found is written here. Every entry has a price, what happens if it is
not paid off, and the release by which it should be. An entry is closed by the
commit that pays it off and is then removed from this file.

Severity: **high**: players or their data can be harmed; **medium**: work or
checks break; **low**: an inconvenience or a future cost.

## Security

### S1. A rendered video is not checked against its replay (high)

Since 7 November a paired device takes only its owner's jobs and the jobs of
chats its owner is in (membership is asked from Telegram on every job request
and kept for 2 minutes). Nobody renders for someone else's chat except the shared
worker token. For accounts without Telegram, one's own job is matched by osu!
player. This is deployed on the server.

Left: compare the video's length with the replay's before posting it to a chat.

Pay off: 0.98.3.

### S6. Updates are checked only against a hash from the same release (low)

The updater checks the archive against `SHA256SUMS` from the same release. That
protects from a broken download, not from a hijacked GitHub account.

Fix: sign `SHA256SUMS` with a key whose public half is built into the app (for
example minisign). The key is kept outside GitHub.

Pay off: 0.100.0.

### S8. The device token is stored in plain text (low)

`settings.json` with mode 0600 on macOS and Linux, with the profile's default
rights on Windows. A copy of the profile folder carries the token away.

Fix: the system keychain (Keychain, Credential Manager, Secret Service).
Price: one more dependency and a settings migration.

Pay off: not before 0.100.0, by the owner's decision.

### Accepted risk: Witness reading game memory

osu! support did not approve reading memory. The owner accepted this risk on
23 October 2026; Witness is turned on only by a switch. It is written here so
the question is not reopened.

## Process

### P2. A shipped version was rebuilt under the same number (medium)

On 6 November 0.97.8 was rebuilt with a fix. Whoever has the old 0.97.8 is not
offered the new one by the auto update.

Rule: any fix after a release gets a new number.

### P6. Goldens drift while no GPU runner checks them (medium)

The GPU job in CI runs only by hand, so nobody compared main-screen frames with
their goldens from 30 October to 7 November, and 261 of them drifted with
accepted visual work. On 7 November every new frame was looked at and the
goldens replaced.

Rule until a runner is connected: before a release, run
`cargo test --manifest-path native/Cargo.toml --test golden` locally on the GPU
and review what it reports.

Pay off: a self-hosted GPU runner, not before 0.100.0.

### P5. Cache limits were chosen without a measurement (medium)

Limiting the caches on 5 November broke avatars and covers, and testers found
it. Tests now hold both caches: a picture still on screen survives a full
Community picture cache, and the frame cache draws one frame shown twice in a
frame and more reels than it keeps.

Measured on 10 November 2026 with `dossier --soak <minutes> --open`: the
application walks the feed, people, the other sections, replays and videos by
itself and prints what every cache holds, while the process's footprint is
sampled from outside. Twenty minutes, seven rounds, a signed-in account with 43
people, 138 friends, 600 replays and 16 videos on an Apple Silicon Mac at
1960×1440:

| | Median | Lowest | Highest | 15 s after launch |
|---|---|---|---|---|
| Before | 1553 MB | 1252 MB | 2017 MB | 1023 MB |
| After | 1123 MB | 946 MB | 1615 MB | 741 MB |

What the measurement found and what was done:

- The Community picture cache held 298 MB against its 128 MB limit. Every
  picture the catalogue names (512 map cards, 181 covers of people) was
  fetched at launch and counted as on screen, so the limit could evict
  nothing. Now only faces and the first rows of the feed are fetched ahead; a
  screen that draws a picture it does not have asks for it, and a picture is
  held while a screen drew it in the last 10 seconds. The cache stays at its
  128 MB. The limit itself is kept: the heaviest screen, *People*, draws 123 MB
  at once.
- The HUD's wide face (M PLUS, for CJK titles) took 98 MB as soon as the live
  scene started, because the rasteriser builds every glyph on loading. It is
  now parsed on the first character the other faces lack.
- The balance desk kept a 768×432 cover for every row, 109 MB for 84 maps.
  Rows now keep 256×144 covers (12 MB), and the open map gets a sharp one, of
  which the last 8 are held.

Left:

- About 450 MB are GPU textures. The renderer's image atlas (33 MB) and the
  antialiasing targets (79 MB) are ruled out by counting them; the frame cache
  is capped at 128 MB. The rest is not attributed yet and needs a Metal
  capture.
- The feed and *People* draw every row they have, so a community of several
  hundred people will again hold more than the limit while that screen is
  open. Drawing only the visible rows fixes both memory and frame time.
- The hour-long run of the same script on the final build.

Pay off: 0.98.3.

## Code

### C1. Big screen files (low, growing)

`main_screen.rs` 7998 lines (10 November 2026, after its picture loading moved
to `main_screen/pictures.rs`), `ui.rs` 6364, `desk_screen.rs` 4100,
`gallery.rs` 2715, `pools_screen.rs` 2100. Every change in them touches other
screens, and parallel changes collide.

Fix: move screens into modules only together with work on them and only without
changing behaviour, in a separate commit with tests and the gallery run. No big
one-off move: it risks everything at once.

### C3. Unmaintained iced dependencies (low)

`paste`, `pin-utils`, `rustybuzz`, `ttf-parser`, `zbus_names` and `zvariant` are
pulled in by iced and wgpu and can only be replaced by an iced update. The CI
advisories job reports unmaintained crates only among direct dependencies
(`unmaintained = "workspace"` in `deny.toml`), so these do not fail it, while any
vulnerability does. We wait for a new iced; our copy of `iced_wgpu` in
`native/vendor` has to be compared again on that update.
