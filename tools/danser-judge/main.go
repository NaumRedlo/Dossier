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

	"github.com/wieku/danser-go/app/beatmap"
	"github.com/wieku/danser-go/framework/assets"
	"github.com/wieku/danser-go/framework/env"
	"github.com/wieku/danser-go/framework/goroutines"
	"github.com/wieku/danser-go/app/beatmap/difficulty"
	"github.com/wieku/danser-go/app/graphics"
	"github.com/wieku/danser-go/app/rulesets/osu"
	"github.com/wieku/danser-go/framework/math/vector"
	"github.com/wieku/rplpa"
)

type out struct {
	Replay string `json:"replay"`
	Err    string `json:"error,omitempty"`
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
	assets.Init(true)
	osuPath, osrPath := os.Args[1], os.Args[2]
	res := out{Replay: osrPath}
	defer func() {
		if r := recover(); r != nil {
			res.Err = fmt.Sprint(r)
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
	beatmap.ParseObjects(bMap, true, false)
	// bMap.Reset() loads skin textures through the objects; judging needs none.

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

	// NewCursor loads a texture and wants a GL context; judging needs none of it.
	cursor := &graphics.Cursor{}
	cursor.IsPlayer = true
	cursor.IsAutoplay = false

	ruleset := osu.NewOsuRuleset(bMap, []*graphics.Cursor{cursor}, []*difficulty.Difficulty{diff})

	// danser's own loop, minus the drawing: frame times are deltas, and each
	// frame offers the click first and sweeps afterwards — the order is
	// load-bearing and is why it is copied rather than simplified.
	frames := replay.ReplayData
	newHandling := replay.OsuVersion >= 20190506
	var t float64
	for i, frame := range frames {
		t += float64(frame.Time)
		now := int64(t)

		cursor.SetPos(vector.NewVec2d(frame.MouseX, frame.MouseY).Copy32())
		cursor.LastFrameTime = cursor.CurrentFrameTime
		cursor.CurrentFrameTime = now
		cursor.IsReplayFrame = true

		cursor.LeftKey = frame.KeyPressed.LeftClick && frame.KeyPressed.Key1
		cursor.RightKey = frame.KeyPressed.RightClick && frame.KeyPressed.Key2
		cursor.LeftMouse = frame.KeyPressed.LeftClick && !frame.KeyPressed.Key1
		cursor.RightMouse = frame.KeyPressed.RightClick && !frame.KeyPressed.Key2
		cursor.LeftButton = frame.KeyPressed.LeftClick
		cursor.RightButton = frame.KeyPressed.RightClick

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
