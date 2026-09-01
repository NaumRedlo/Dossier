# Judging the corpus with danser's own rules

danser is the reference this engine was written against, and it had never been
measured on the corpus — only quoted. This is the harness that measures it.

danser is a renderer and its CLI wants a window. Its rules are a library and do
not, so `main.go` drives that library the way `app/dance/rcontroller.go` drives
it — click, normal, post, once per replay frame, in that order, because the
order is load-bearing — and prints the counts as JSON.

## It does not run on macOS, and the reason is not the harness

`beatmap.ParseObjects` builds hit objects, and `BeatMap.Reset()` calls
`SetDifficulty` on each of them. For a circle that is:

```go
circle.hitCircleTexture = skin.GetTexture(name)
circle.comboText = sprite.NewTextSpriteSize(..., skin.GetFont("default"), ...)
```

— a texture atlas and a font, before a single note is judged. The atlas calls
`gl.GetIntegerv`, which segfaults with no GL context, and the font's result is
dereferenced immediately, so returning nil does not help either. Skipping
`Reset` gets past the crash and into a nil dereference in the ruleset, because
the objects genuinely need their difficulty set.

Upstream ships no macOS build and the repository carries only Linux `.so` and
Windows `.dll` for BASS, so there is no headless path on darwin worth building.

## Where to run it

A Linux box, which is what danser ships for. On the bot's own host:

```bash
git clone --depth 1 https://github.com/Wieku/danser-go.git
cp <this file> danser-go/cmd/judge/main.go
cd danser-go && go build -o judge-cli ./cmd/judge
./judge-cli <map.osu> <replay.osr>
```

It needs the `.osu` files rather than the `.osz` archives — one per replay, as
`dossier judge` names them in its `file` line — and no audio, so the corpus can
go over without its songs.

## What it is for

One table with three columns: what the client wrote in the replay header, what
this engine judges, and what danser judges, over the same replays. Until that
table exists, "as good as danser" is an assertion rather than a measurement.
