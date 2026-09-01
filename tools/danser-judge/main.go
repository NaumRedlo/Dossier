// A judging harness over danser's own ruleset.
//
// danser is a renderer, and its CLI wants a window; the rules are a library and
// do not. This drives that library the way `rcontroller.go` drives it — click,
// normal, post, per replay frame — and prints the counts, so danser's judgement
// of a corpus can be put beside another engine's on the same yardstick.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"runtime/debug"
	"path/filepath"

	"github.com/go-gl/gl/v3.3-core/gl"
	"github.com/go-gl/glfw/v3.3/glfw"
	"github.com/wieku/danser-go/app/beatmap"
	"github.com/wieku/danser-go/framework/assets"
	"github.com/wieku/danser-go/framework/env"
	"github.com/wieku/danser-go/framework/goroutines"
	"github.com/wieku/danser-go/app/beatmap/difficulty"
	"github.com/wieku/danser-go/app/graphics"
	"github.com/wieku/danser-go/app/dance/input"
	"github.com/wieku/danser-go/app/rulesets/osu"
	"github.com/wieku/danser-go/app/settings"
	"github.com/wieku/danser-go/framework/math/vector"
	"github.com/wieku/rplpa"
)

type out struct {
	Replay string `json:"replay"`
	Err    string `json:"error,omitempty"`
	Stack  string `json:"stack,omitempty"`
	C300   uint   `json:"c300"`
	C100   uint   `json:"c100"`
	C50    uint   `json:"c50"`
	CMiss  uint   `json:"cmiss"`
	Combo  uint   `json:"combo"`
}

func main() {
	goroutines.RunMain(run)
}

func run() {
	env.Init("danser")

	// danser's hit objects load skin textures and a font in `SetDifficulty`,
	// before a single note is judged, and the texture atlas wants a live GL
	// context. Rather than stub any of that — which would mean measuring a
	// modified danser — the harness opens a hidden one-pixel window and lets
	// danser be itself. Under `xvfb-run` on a headless runner this is Mesa's
	// software renderer, which is slow and entirely sufficient for a context
	// nothing ever draws into.
	if err := glfw.Init(); err != nil {
		panic(err)
	}
	glfw.WindowHint(glfw.Visible, glfw.False)
	glfw.WindowHint(glfw.ContextVersionMajor, 3)
	glfw.WindowHint(glfw.ContextVersionMinor, 3)
	glfw.WindowHint(glfw.OpenGLProfile, glfw.OpenGLCoreProfile)
	win, err := glfw.CreateWindow(1, 1, "judge", nil, nil)
	if err != nil {
		panic(err)
	}
	win.MakeContextCurrent()
	if err := gl.Init(); err != nil {
		panic(err)
	}

	assets.Init(true)
	osuPath, osrPath := os.Args[1], os.Args[2]
	res := out{Replay: osrPath}
	defer func() {
		if r := recover(); r != nil {
			// The stack, not just the message: "nil pointer dereference" on all
			// 176 says nothing about which of danser's fields was not set up.
			res.Err = fmt.Sprint(r)
			res.Stack = string(debug.Stack())
		}
		b, _ := json.Marshal(res)
		fmt.Println(string(b))
	}()

	data, err := os.ReadFile(osrPath)
	if err != nil {
		res.Err = err.Error()
		return
	}
	replay, err := rplpa.ParseReplay(data)
	if err != nil {
		res.Err = err.Error()
		return
	}

	// `ParseBeatMapFile` takes the path apart against danser's own songs
	// directory and `ParseBeatMap` puts it back together the same way, so a map
	// outside that directory is simply not found — every one of 176 came back
	// "beatmap did not parse" for this and nothing else. The value is cached on
	// first read, so it has to be set before anything asks.
	settings.General.OsuSongsDir = filepath.Dir(osuPath)

	f, err := os.Open(osuPath)
	if err != nil {
		res.Err = err.Error()
		return
	}
	bMap := beatmap.ParseBeatMapFile(f)
	f.Close()
	if bMap == nil {
		res.Err = "beatmap did not parse"
		return
	}
	beatmap.ParseTimingPointsAndPauses(bMap)
	beatmap.ParseObjects(bMap, false, false)
	bMap.Reset()

	diff := bMap.Diff.Clone()
	if replay.ScoreInfo != nil && len(replay.ScoreInfo.Mods) > 0 {
		mods := make([]rplpa.ModInfo, 0, len(replay.ScoreInfo.Mods))
		for _, m := range replay.ScoreInfo.Mods {
			mods = append(mods, *m)
		}
		diff.SetMods2(mods)
	} else {
		diff.SetMods(difficulty.Modifier(replay.Mods))
	}

	// `NewCursor` builds a framebuffer for drawing the cursor trail, and that
	// is the one thing the software renderer cannot give — it reports a maximum
	// texture size of zero and `initCursor` dereferences the result. The ruleset
	// reads a cursor's position, its keys and its frame times and nothing else,
	// so the zero value is the whole of what judging needs.
	cursor := &graphics.Cursor{}
	cursor.IsPlayer = true
	cursor.IsAutoplay = false

	ruleset := osu.NewOsuRuleset(bMap, []*graphics.Cursor{cursor}, []*difficulty.Difficulty{diff})

	// Under Relax the game does the clicking and writes none of it down, so
	// danser makes the presses itself. Reading the keys out of the file instead
	// hands the ruleset a replay that never pressed anything — eight of the
	// eleven worst results in the first run were this and nothing else.
	isRelax := diff.CheckModActive(difficulty.Relax)
	var relax *input.RelaxInputProcessor
	if isRelax {
		relax = input.NewRelaxInputProcessor(ruleset, cursor)
	}

	// Every judgement as it is made, for reading one replay object by object
	// rather than comparing four totals.
	if len(os.Args) > 3 && os.Args[3] == "--objects" {
		ruleset.SetListener(func(_ *graphics.Cursor, r osu.JudgementResult, _ osu.Score) {
			fmt.Printf("OBJ %d %d %v\n", r.Number, r.Time, r.HitResult)
		})
	}

	// danser's own loop, minus the drawing: frame times are deltas, and each
	// frame offers the click first and sweeps afterwards — the order is
	// load-bearing and is why it is copied rather than simplified.
	// danser's own preprocessing, out of `loadFrames`. Skipping it was worth
	// hundreds of phantom misses on some replays: the seed frame is a marker
	// rather than a moment, and a leading zero delta doubles the first frame.
	frames := replay.ReplayData
	for i, fr := range frames {
		if fr.Time == -12345 {
			frames = append(frames[:i], frames[i+1:]...)
			break
		}
	}
	if len(frames) > 0 && frames[0].Time == 0 {
		frames = frames[1:]
	}
	newHandling := replay.OsuVersion >= 20190506
	var t float64
	for i, frame := range frames {
		t += float64(frame.Time)
		now := int64(t)

		// `SetPos` ends by telling the cursor's renderer where to draw, and there
		// is no renderer here. With display inversion and edge bouncing off — both
		// default — everything it does before that is these two assignments, and
		// they are the two fields the ruleset reads.
		pos := vector.NewVec2d(frame.MouseX, frame.MouseY).Copy32()
		cursor.RawPosition = pos
		cursor.Position = pos
		cursor.LastFrameTime = cursor.CurrentFrameTime
		cursor.CurrentFrameTime = now
		cursor.IsReplayFrame = true

		if isRelax {
			relax.Update(float64(now))
		} else {
			cursor.LeftKey = frame.KeyPressed.LeftClick && frame.KeyPressed.Key1
			cursor.RightKey = frame.KeyPressed.RightClick && frame.KeyPressed.Key2
			cursor.LeftMouse = frame.KeyPressed.LeftClick && !frame.KeyPressed.Key1
			cursor.RightMouse = frame.KeyPressed.RightClick && !frame.KeyPressed.Key2
			cursor.LeftButton = frame.KeyPressed.LeftClick
			cursor.RightButton = frame.KeyPressed.RightClick
		}

		ruleset.UpdateClickFor(cursor, now)
		ruleset.UpdateNormalFor(cursor, now, false)
		if newHandling || i == len(frames)-1 {
			ruleset.UpdatePostFor(cursor, now, false)
		} else {
			next := i + 1
			if next >= len(frames) {
				next = len(frames) - 1
			}
			for local := now; local < int64(t+float64(frames[next].Time)); local++ {
				ruleset.UpdatePostFor(cursor, local, false)
			}
		}
		ruleset.Update(now)
	}

	s := ruleset.GetScore(cursor)
	res.C300, res.C100, res.C50, res.CMiss, res.Combo = s.Count300, s.Count100, s.Count50, s.CountMiss, s.Combo
}
