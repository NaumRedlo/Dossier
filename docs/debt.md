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

Pay off: 0.98.2.

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

Left: a long session memory measurement on a signed-in account (feed, people,
replays, videos for an hour) and limits chosen from it.

Pay off: 0.98.2.

## Code

### C1. Big screen files (low, growing)

`main_screen.rs` 7896 lines, `ui.rs` 6364, `desk_screen.rs` 4100,
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
