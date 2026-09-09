mod assay;
mod debug;
mod exhibit;
mod manifest;
mod report;
mod skinfile;

use dossier_produce::{self as produce, events, hitsounds, locate, reel, scenery, video};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dossier_beatmap::Beatmap;
use dossier_render::{Effects, Layout, Scene, Skin};

use dossier_replay::{GameMode, Replay};
use dossier_sim::{GameState, Judgement, Part, Ruleset};

use report::{error_json, Header, PartCheck, Report};

const USAGE: &str = "\
dossier — osu! replay analysis

USAGE:
    dossier inspect [--json] <replay.osr>...
    dossier judge [OPTIONS] <replay.osr>...
    dossier corpus [OPTIONS] <replay.osr>...
    dossier debug [OPTIONS] --from <ms> --to <ms> <replay.osr>
    dossier sliders [OPTIONS] <replay.osr>...
    dossier errors [OPTIONS] <replay.osr>...
    dossier score [OPTIONS] <replay.osr>...
    dossier health [OPTIONS] <replay.osr>...
    dossier frame [OPTIONS] --at <ms> <replay.osr>
    dossier video [OPTIONS] <replay.osr>
    dossier exhibit [OPTIONS] <replay.osr>
    dossier exhibit --survey [OPTIONS] <replay.osr>...
    dossier sounds [OPTIONS] [-o kit.wav]
    dossier skin [OPTIONS] -o <folder>

`inspect` reads the header alone — no map needed. Use it to learn which map a
replay wants before going and fetching it.

`exhibit` picks the few seconds of a play that say something about it, and
says why each was chosen. Unlike everything else here it has no ground truth
to be checked against — no header names the moments worth watching — so it
answers in reasons rather than in numbers, and `--json` shows the whole answer
without rendering a frame. A reel is as long as the play gives it reason to be:
selection stops when nothing left is worth the seconds it would cost, so a
clean run of a quiet map comes out short and a disaster on a marathon does not. With `-o` it renders the chosen clips and cuts them
together, crossfading each into the next and fading from and to black.

`sounds` writes a short WAV of the hit sounds alone — every voice, then a fast
stream — so a kit can be listened to and retuned without rendering a video.

`debug` reads a window of the play back object by object and click by click,
with the difficulty numbers it used and, when the note lock refuses a click,
which note it is stuck on and every press that came near that note. It is the
last step down from a total that disagrees to the one verdict responsible.

`corpus` is the measurement every change is judged by: one line per replay
that disagrees, and a total. With `--strict <n>` it fails when that total is
worse than n, which is what makes it a check rather than a report.

`--expect tools/corpus.tsv` goes further: that file names which replays the
corpus is, by hash, and what each one is expected to do. It catches what a
total cannot — a replay that got worse while another got better, a replay
this machine does not have, the same file counted twice from two folders.
`--update-expect` writes what this run measured into it, leaving the rows for
replays it did not see alone; `--prune` is how one of those rows leaves.

`sliders` and `errors` are for when `judge` disagrees with the replay and the
question is where. The first breaks slider verdicts down by which part was
dropped; the second shows how hits pile up around the judgement windows, which
is where off-by-one hides.

OPTIONS (judge):
    -m, --map <path>     .osu or .osz to judge against. Without it, --songs is
                         searched for the map the replay names by hash.
    -s, --songs <dir>    Directory to search (default: $DOSSIER_SONGS_DIR).
    -j, --json           One JSON object per replay, on its own line.
    -a, --at <ms>        frame: the instant to draw, in map time.
    -o, --out <path>     exhibit: the reel. Without it the selection is printed
                         and nothing is rendered.
        --for <s>        exhibit: the most video it may come to (default 120).
                         A ceiling, not a target — a reel is as long as the play
                         gives it reason to be.
        --survey         exhibit: aggregate over every replay given instead of
                         answering about one — what reels are made of, how long
                         they run, and how many say nothing about the play.
                         Selection has no ground truth, so this stands in for
                         one: a change cannot be shown to be right, only what it
                         did to a hundred replays.
        --worth <0..1>   exhibit: the score under which a moment is not worth
                         the seconds it costs (default 0.25). This is what
                         decides how long a reel is. Lower it for a longer reel
                         of weaker moments.
        --clip <s>       exhibit: shortest clip, in seconds (default 6). The
                         more important a moment, the longer its clip runs.
        --fps <n>        video: frames per second (default 60).
        --from <ms>      video: start of the span, in map time.
        --to <ms>        video: end of the span. Both default to the whole play.
        --crf <n>        video: x264 quality, lower is better (default 20).
        --preset <name>  video: x264 preset (default veryfast). Faster presets
                         trade file size for speed, and once the encoder is the
                         bottleneck that trade is the main thing left to make.
        --events         video/exhibit: say what the render is doing on stdout,
                         one JSON object per line — which clip is being drawn,
                         how far along its frames are, the shape of the file
                         and where it was written. For a caller that would
                         otherwise have to read the prose on stderr, which is
                         written for a person and changes like prose does.
        --mute           video: skip the map's audio.
--no-map-hitsounds   video: play the skin's hit sounds alone.
          --dim <0-100>  video: how far the map's artwork is darkened.
        --music <0-100>  video: how loud the map's own track is.
    --hitsounds <0-100>  video: how loud the hit sounds are.
        --ffmpeg <path>  video: the encoder to run (default `ffmpeg`).
    -o, --out <path>     frame: where to write the PNG (default frame.png).
        --size <WxH>     frame: output size (default 1920x1080).
        --threads <n>    video: threads drawing frames. Defaults to one fewer
                         than the machine has, leaving a core for the encoder.
                         corpus: threads judging replays, one replay each.
                         Defaults to every core — nothing waits on an encoder
                         here. `--threads 1` if a measurement has to be watched
                         happening.
        --encoder-threads <n>
                         video: cap the encoder's own threads. ffmpeg otherwise
                         takes about 1.5 per core and fights the drawing for
                         them. Tune it until the report's drawing-per-thread
                         and piping figures meet.
        --samples <dir>  sounds/video: a skin folder of `{set}-hit{sound}.wav`.
                         Whatever it lacks falls back to the synthesised kit.
        --kit <name>     sounds/video: click, soft, drum, glass or wood.
                         Overrides whatever the skin would have chosen.
   --game-sounds <dir>   osu!'s own sounds, which is where the game's lookup
                         ends: a skin that leaves `soft-hitwhistle` out gets
                         osu!'s rather than silence or another bank's.
                         `tools/stable.py assets` writes such a folder from a
                         client's `osu!gameplay.dll`. ($DOSSIER_GAME_SOUNDS)
        --pitch <x>      sounds/video: multiply every hit-sound frequency.
        --decay <x>      sounds/video: multiply every hit-sound decay.
        --level <x>      sounds/video: multiply hit-sound loudness.
        --bare           frame/video/exhibit: draw the play and nothing that
                         talks about it — no score, accuracy, combo, key
                         counters, scoreboard or signature. For a clip that has
                         to stand beside somebody's own footage rather than
                         explain itself. The red of a dying play stays: that is
                         the screen reacting, not a readout.
        --skin <name>    `1984` (the default: the bot's palette and a darker,
                         drier hit kit) or `classic` (the map's own combo
                         colours and a neutral kit).
        --font <path>    frame: typeface for the HUD and combo numbers.
                         Defaults to $DOSSIER_FONT, then Huninn beside the
                         program. Without one the play is drawn but no numbers.
    -t, --trace          judge: account for every click — where each one went,
                         and where the note lock refused several in a row. With
                         --from/--to it also lists the clicks in that window one
                         by one, with the object each was tested against.
        --marginal <n>   judge: the n hits that came closest to not being hits,
                         ranked by the room they had against the window and the
                         radius. For when the totals say we credited objects
                         the game did not and nothing structural explains it.
        --leaderboard <tsv>
                         video/frame: who else has played this map, drawn down
                         the left, climbing to the leader. A line each:
                         `name<TAB>score[<TAB>accuracy<TAB>mods<TAB>avatar.png<TAB>cover.png]`.
                         The player's own row is computed, not read. Pictures
                         must be PNG — the engine has one decoder and no
                         network.
        --my-pictures <avatar.png> <cover.png>
                         video/frame: the player's own avatar and cover, which
                         no rival line can carry.
        --expect <tsv>   corpus: the file naming the corpus and what each
                         replay in it does. Fails on any replay that got worse,
                         and with --strict on any that is missing here.
        --update-expect  corpus: write what this run measured into that file.
                         A replay it did not see keeps the row it had — the
                         corpus is a list of replays this machine may or may
                         not be holding today, and a partial run is not news
                         that the rest of it is gone.
        --prune          corpus: with --update-expect, drop the rows this run
                         did not see. For a replay that has left the corpus,
                         which is not the same thing as one that is elsewhere.
        --strict [n]     corpus: fail when the total count error is worse than
                         n. Without a number, judge: fail on any mismatch.
    -e, --explain        List every object we called a miss, and what the input
                         says near it — the difference between a geometry bug
                         and a genuinely missed note.
        --strict         Exit non-zero when a replay doesn't match exactly.
    -h, --help           This text.
    -V, --version        What this binary is, and which commit built it.
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Assay,
    Inspect,
    Judge,
    Corpus,
    Debug,
    Sliders,
    Errors,
    Score,
    Health,
    Frame,
    Video,
    Exhibit,
    Sounds,
    Skin,
}

impl Command {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "assay" => Self::Assay,
            "inspect" => Self::Inspect,
            "judge" => Self::Judge,
            "corpus" => Self::Corpus,
            "debug" => Self::Debug,
            "sliders" => Self::Sliders,
            "errors" => Self::Errors,
            "score" => Self::Score,
            "health" => Self::Health,
            "frame" => Self::Frame,
            "video" => Self::Video,
            "exhibit" => Self::Exhibit,
            "sounds" => Self::Sounds,
            "skin" => Self::Skin,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Assay => "assay",
            Self::Inspect => "inspect",
            Self::Judge => "judge",
            Self::Corpus => "corpus",
            Self::Debug => "debug",
            Self::Sliders => "sliders",
            Self::Errors => "errors",
            Self::Score => "score",
            Self::Health => "health",
            Self::Frame => "frame",
            Self::Video => "video",
            Self::Exhibit => "exhibit",
            Self::Sounds => "sounds",
            Self::Skin => "skin",
        }
    }

    fn synopsis(self) -> &'static str {
        match self {
            Self::Assay => "dossier assay --map <map.osu> [--mods HDDT] [PLAY]",
            Self::Inspect => "dossier inspect [--json] <replay.osr>...",
            Self::Judge => "dossier judge [OPTIONS] <replay.osr>...",
            Self::Corpus => "dossier corpus [OPTIONS] <replay.osr>...",
            Self::Debug => "dossier debug [OPTIONS] --from <ms> --to <ms> <replay.osr>",
            Self::Sliders => "dossier sliders [OPTIONS] <replay.osr>...",
            Self::Errors => "dossier errors [OPTIONS] <replay.osr>...",
            Self::Score => "dossier score [OPTIONS] <replay.osr>...",
            Self::Health => "dossier health [OPTIONS] <replay.osr>...",
            Self::Frame => "dossier frame [OPTIONS] --at <ms> <replay.osr>",
            Self::Video => "dossier video [OPTIONS] <replay.osr>",
            Self::Exhibit => "dossier exhibit [OPTIONS] <replay.osr>",
            Self::Sounds => "dossier sounds [OPTIONS] [-o kit.wav]",
            Self::Skin => "dossier skin [OPTIONS] -o <folder>",
        }
    }

    fn accepts(self, flag: &str) -> bool {
        const MAP: &[&str] = &["--map", "--songs"];

        const LOOK: &[&str] = &[
            "--out",
            "--size",
            "--font",
            "--skin",
            "--bare",
            "--volume",
            "--music",
            "--hitsounds",
            "--no-map-hitsounds",
            "--dim",
            "--blur",
            "--meter-scale",
            "--cursor-scale",
            "--cursor-rotate",
            "--skin-as-written",
            "--trace-hitsounds",
            "--effects",
            "--leaderboard",
            "--my-pictures",
        ];

        const ENCODE: &[&str] = &[
            "--fps",
            "--crf",
            "--preset",
            "--mute",
            "--ffmpeg",
            "--threads",
            "--encoder-threads",
            "--events",
            "--from",
            "--to",
        ];

        const HITSOUND: &[&str] = &[
            "--samples",
            "--game-sounds",
            "--kit",
            "--pitch",
            "--decay",
            "--level",
        ];

        let groups: &[&[&str]] = match self {
            Self::Assay => &[
                &["--map", "--mods"],
                &[
                    "--accuracy",
                    "--combo",
                    "--misses",
                    "--n300",
                    "--n100",
                    "--n50",
                ],
                &[
                    "--slider-ends",
                    "--large-tick-misses",
                    "--classic",
                    "--legacy-total",
                ],
            ],
            Self::Inspect => &[&["--json"]],
            Self::Judge => &[
                MAP,
                &[
                    "--json",
                    "--explain",
                    "--trace",
                    "--marginal",
                    "--strict",
                    "--from",
                    "--to",
                ],
            ],
            Self::Corpus => &[
                MAP,
                &[
                    "--expect",
                    "--update-expect",
                    "--prune",
                    "--strict",
                    "--threads",
                ],
            ],
            Self::Debug => &[MAP, &["--from", "--to"]],
            Self::Sliders | Self::Errors | Self::Score => &[MAP],
            Self::Health => &[MAP, &["--trace"]],
            Self::Frame => &[
                MAP,
                LOOK,
                &[
                    "--at",
                    "--background",
                    "--storyboard",
                    "--video",
                    "--ffmpeg",
                ],
            ],
            Self::Video => &[
                MAP,
                LOOK,
                ENCODE,
                HITSOUND,
                &["--slow", "--background", "--storyboard", "--video"],
            ],
            Self::Exhibit => &[
                MAP,
                LOOK,
                &["--background", "--storyboard", "--video"],
                &[
                    "--fps",
                    "--crf",
                    "--preset",
                    "--mute",
                    "--volume",
                    "--music",
                    "--hitsounds",
                    "--no-map-hitsounds",
                    "--trace-hitsounds",
                    "--dim",
                    "--ffmpeg",
                    "--threads",
                    "--encoder-threads",
                    "--events",
                ],
                HITSOUND,
                &["--json", "--for", "--worth", "--clip", "--survey"],
            ],
            Self::Sounds => &[HITSOUND, &["--out"]],

            Self::Skin => &[&["--out", "--skin", "--samples", "--font"]],
        };
        groups.iter().any(|group| group.contains(&flag))
    }

    fn help(self) -> String {
        let mut out = format!("{}\n\nOptions:\n", self.synopsis());
        for (flag, value, summary) in OPTIONS_TABLE {
            if self.accepts(flag) {
                let head = if value.is_empty() {
                    (*flag).to_owned()
                } else {
                    format!("{flag} {value}")
                };
                out.push_str(&format!("    {head:<24} {summary}\n"));
            }
        }
        out.push_str("    -h, --help               this text\n");
        out
    }
}

fn canonical(flag: &str) -> &str {
    match flag {
        "-m" => "--map",
        "-s" => "--songs",
        "-a" => "--at",
        "-o" => "--out",
        "-j" => "--json",
        "-e" => "--explain",
        "-t" => "--trace",
        other => other,
    }
}

const OPTIONS_TABLE: &[(&str, &str, &str)] = &[
    (
        "--map",
        "<path>",
        "the .osu or .osz to judge against (short -m)",
    ),
    (
        "--songs",
        "<dir>",
        "where to search for the map (short -s; $DOSSIER_SONGS_DIR)",
    ),
    (
        "--json",
        "",
        "one JSON object per replay, on its own line (short -j)",
    ),
    (
        "--explain",
        "",
        "list every miss and what the input says near it (short -e)",
    ),
    (
        "--trace",
        "",
        "account for every click; with --from/--to, list that window (short -t)",
    ),
    (
        "--marginal",
        "<n>",
        "the n hits that came closest to not being hits",
    ),
    (
        "--strict",
        "[n]",
        "fail on a mismatch; with n, fail when the corpus total is worse",
    ),
    ("--expect", "<tsv>", "the corpus manifest to check against"),
    (
        "--update-expect",
        "",
        "write what this run measured into the manifest",
    ),
    (
        "--prune",
        "",
        "with --update-expect, drop rows this run did not see",
    ),
    (
        "--threads",
        "<n>",
        "threads drawing frames, or judging replays for corpus",
    ),
    ("--at", "<ms>", "the instant to draw, in map time"),
    (
        "--slow",
        "<ms>",
        "a map instant to slow into and back out of",
    ),
    (
        "--background",
        "",
        "draw the map's own artwork behind the play",
    ),
    (
        "--storyboard",
        "",
        "draw the map's own storyboard, when it has one",
    ),
    (
        "--video",
        "",
        "lay the render over the map's own background video",
    ),
    ("--from", "<ms>", "start of the span, in map time"),
    ("--to", "<ms>", "end of the span, in map time"),
    (
        "--for",
        "<s>",
        "the most video a reel may come to (a ceiling, not a target)",
    ),
    (
        "--worth",
        "<0..1>",
        "the score under which a moment is not worth its seconds",
    ),
    ("--clip", "<s>", "shortest clip, in seconds"),
    (
        "--survey",
        "",
        "aggregate over every replay instead of answering about one",
    ),
    ("--out", "<path>", "where to write the output (short -o)"),
    ("--size", "<WxH>", "output size"),
    ("--fps", "<n>", "frames per second"),
    ("--crf", "<n>", "x264 quality, lower is better"),
    ("--preset", "<name>", "x264 preset"),
    ("--mute", "", "skip the map's audio"),
    ("--ffmpeg", "<path>", "the encoder to run"),
    ("--encoder-threads", "<n>", "cap the encoder's own threads"),
    (
        "--events",
        "",
        "report what the render is doing on stdout, as JSON lines",
    ),
    ("--skin", "<name>", "`1984` (default) or `classic`"),
    (
        "--bare",
        "",
        "draw the play and nothing that talks about it",
    ),
    ("--no-map-hitsounds", "", "play the skin's hit sounds alone"),
    ("--dim", "<0-100>", "how far the map's artwork is darkened"),
    ("--blur", "<0-100>", "how hard the map's artwork is blurred"),
    (
        "--meter-scale",
        "<0.5-3>",
        "how big the hit-error meter is drawn",
    ),
    (
        "--cursor-scale",
        "<0.4-2>",
        "how big the cursor and its trail are drawn",
    ),
    (
        "--cursor-rotate",
        "<on|off>",
        "turn the cursor, or hold it still, whatever the skin says",
    ),
    (
        "--skin-as-written",
        "",
        "date a skin the way osu! does, rocking arrows and all",
    ),
    (
        "--trace-hitsounds",
        "",
        "say what every sound resolved to, and how often",
    ),
    (
        "--volume",
        "<0-200>",
        "how loud everything is, over the two below",
    ),
    ("--music", "<0-100>", "how loud the map's own track is"),
    ("--hitsounds", "<0-100>", "how loud the hit sounds are"),
    (
        "--effects",
        "<list>",
        "which optional movements are on, comma separated",
    ),
    ("--font", "<path>", "typeface for the HUD ($DOSSIER_FONT)"),
    (
        "--leaderboard",
        "<tsv>",
        "who else has played this map, down the left",
    ),
    (
        "--my-pictures",
        "<a> <c>",
        "the player's own avatar and cover",
    ),
    ("--samples", "<dir>", "a skin folder of hit-sound WAVs"),
    (
        "--game-sounds",
        "<dir>",
        "osu!'s own sounds, for what a skin leaves out ($DOSSIER_GAME_SOUNDS)",
    ),
    ("--kit", "<name>", "click, soft, drum, glass or wood"),
    ("--pitch", "<x>", "multiply every hit-sound frequency"),
    ("--decay", "<x>", "multiply every hit-sound decay"),
    ("--level", "<x>", "multiply hit-sound loudness"),
];

fn version() -> String {
    format!(
        "dossier {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("DOSSIER_BUILD")
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args[0] == "-V" || args[0] == "--version" {
        println!("{}", version());
        return ExitCode::SUCCESS;
    }
    let Some(command) = Command::from_name(&args[0]) else {
        eprintln!("dossier: unknown command `{}`\n\n{USAGE}", args[0]);
        return ExitCode::FAILURE;
    };
    let rest = &args[1..];

    if rest.iter().any(|a| a == "-h" || a == "--help") {
        print!("{}", command.help());
        return ExitCode::SUCCESS;
    }
    match Options::parse(command, rest) {
        Ok(options) => dispatch(command, options),
        Err(message) => {
            eprintln!("dossier: {message}\n\n{}", command.help());
            ExitCode::FAILURE
        }
    }
}

fn dispatch(command: Command, options: Options) -> ExitCode {
    match command {
        Command::Corpus => corpus(options),
        Command::Judge => judge(options),
        Command::Sounds => sounds(options),
        Command::Video => video_command(options),
        Command::Exhibit => exhibit_command(options),
        Command::Frame => frame(options),
        Command::Health => health_command(options),
        Command::Score => score_command(options),
        Command::Errors => errors(options),
        Command::Debug => debug_command(options),
        Command::Sliders => sliders(options),
        Command::Inspect => inspect(options),
        Command::Skin => skin_command(options),
        Command::Assay => match assay_command(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("dossier: {message}");
                ExitCode::FAILURE
            }
        },
    }
}

struct Options {
    replays: Vec<PathBuf>,
    map: Option<PathBuf>,
    songs: Option<PathBuf>,
    json: bool,
    explain: bool,
    trace: bool,
    marginal: Option<usize>,
    strict: bool,

    corpus_ceiling: Option<u32>,

    expect: Option<PathBuf>,

    update_expect: bool,

    prune: bool,

    events: bool,

    leaderboard: Option<PathBuf>,

    my_avatar: Option<PathBuf>,
    my_cover: Option<PathBuf>,
    at_ms: Option<f64>,

    slow_at_ms: Option<f64>,

    background: bool,

    storyboard: bool,

    video: bool,

    exhibit_budget_s: Option<f64>,

    exhibit_worth: Option<f64>,

    survey: bool,

    bare: bool,

    dim: Option<u32>,

    meter_scale: Option<f32>,

    cursor_scale: Option<f32>,
    cursor_rotate: Option<bool>,

    blur: Option<u32>,

    skin_as_written: bool,

    trace_hitsounds: bool,

    map_hitsounds: bool,

    music_level: u32,
    hitsound_level: u32,

    volume: u32,

    effects: Option<String>,

    exhibit_clip_s: Option<f64>,
    out: PathBuf,
    size: (u32, u32),
    font: Option<PathBuf>,
    fps: f64,
    from_ms: Option<f64>,
    to_ms: Option<f64>,
    crf: u32,
    preset: String,
    ffmpeg: String,
    mute: bool,
    skin: SkinChoice,
    kit: Option<dossier_audio::Kit>,
    samples: Option<PathBuf>,

    game_sounds: Option<PathBuf>,
    threads: Option<usize>,
    encoder_threads: Option<usize>,
    pitch: Option<f32>,
    decay: Option<f32>,
    level: Option<f32>,

    mods: Option<String>,

    accuracy: Option<f64>,
    combo: Option<u32>,
    misses: Option<u32>,
    n300: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,

    slider_ends: Option<u32>,
    large_tick_misses: Option<u32>,

    classic: bool,
    legacy_total: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SkinChoice {
    Folder(PathBuf),

    Classic,
}

impl Options {
    fn levels(&self) -> (f32, f32) {
        let master = self.volume as f32 / 100.0;
        (
            self.music_level as f32 / 100.0 * master,
            self.hitsound_level as f32 / 100.0 * master,
        )
    }

    fn look(&self, beatmap: &Beatmap) -> Skin {
        let mut skin = self.skin.visual(beatmap, self.effects.as_deref());
        if let Some(at) = self.meter_scale {
            skin.meter_scale = at;
        }
        if let Some(at) = self.cursor_scale {
            skin.cursor_scale = at;
        }
        skin.cursor_rotate = self.cursor_rotate;
        skin.skin_version_as_written = self.skin_as_written;
        skin
    }

    fn behind<'a>(&'a self, at_ms: Option<f64>, scratch: Option<&'a Path>) -> scenery::Behind<'a> {
        scenery::Behind {
            background: self.background,
            storyboard: self.storyboard,
            video: self.video,
            dim: self.dim,
            blur: self.blur,
            ffmpeg: &self.ffmpeg,
            size: self.size,
            at_ms,
            scratch,
        }
    }

    fn kit(&self) -> dossier_audio::Kit {
        let mut kit = self.kit.unwrap_or_else(|| self.skin.kit());
        if let Some(pitch) = self.pitch {
            kit.pitch *= pitch;
        }
        if let Some(decay) = self.decay {
            kit.decay *= decay;
        }
        if let Some(level) = self.level {
            kit.level *= level;
        }
        kit
    }
}

impl Options {
    fn samples_folder(&self) -> Option<PathBuf> {
        if let Some(folder) = &self.samples {
            return folder.is_dir().then(|| folder.clone());
        }
        let named = self.skin.samples_dir()?;
        if named.is_dir() {
            return Some(named.to_path_buf());
        }
        ["", "../", "../../"]
            .iter()
            .map(|prefix| PathBuf::from(format!("{prefix}{}", named.display())))
            .find(|folder| folder.is_dir())
    }

    fn samples_with_map(
        &self,
        origin: &locate::Origin,
        scratch: Option<&Path>,
    ) -> dossier_audio::SamplePack {
        let pack = self.samples();
        if !self.map_hitsounds {
            return pack;
        }
        let Some(dir) = scratch.map(|at| at.join("map-samples")) else {
            return pack;
        };
        if std::fs::create_dir_all(&dir).is_err() {
            return pack;
        }
        let written = locate::extract_samples(origin, &dir, &self.ffmpeg);
        if written == 0 {
            return pack;
        }
        let pack = pack.with_beatmap(&dir);
        eprintln!(
            "dossier: {} sound(s) from the map itself",
            pack.from_beatmap()
        );
        pack
    }

    fn under(&self, pack: dossier_audio::SamplePack) -> dossier_audio::SamplePack {
        let Some(folder) = &self.game_sounds else {
            return pack;
        };
        let pack = pack.with_game_sounds(folder);
        if pack.from_game() == 0 {
            eprintln!(
                "dossier: no `{{set}}-hit{{sound}}.wav` under {} — osu!'s own sounds are not in play",
                folder.display()
            );
        }
        pack
    }

    fn samples(&self) -> dossier_audio::SamplePack {
        self.under(self.skin_samples())
    }

    fn skin_samples(&self) -> dossier_audio::SamplePack {
        if let Some(folder) = &self.samples {
            let pack = dossier_audio::SamplePack::load(folder);
            if pack.is_empty() {
                eprintln!(
                    "dossier: no `{{set}}-hit{{sound}}.wav` under {} — using the synthesised kit",
                    folder.display()
                );
            } else {
                eprintln!(
                    "dossier: {} sample(s) from {}",
                    pack.len(),
                    folder.display()
                );
                report_unused(&pack);
            }
            return pack;
        }

        let Some(relative) = self.skin.samples_dir() else {
            return dossier_audio::SamplePack::default();
        };
        for prefix in ["", "../", "../../"] {
            let folder = PathBuf::from(format!("{prefix}{}", relative.display()));
            let pack = dossier_audio::SamplePack::load(&folder);
            if !pack.is_empty() {
                eprintln!(
                    "dossier: {} sample(s) from {}",
                    pack.len(),
                    folder.display()
                );
                report_unused(&pack);
                return pack;
            }
        }

        dossier_audio::SamplePack::default()
    }
}

fn parse_number(value: Option<&String>, flag: &str) -> Result<f32, String> {
    value
        .ok_or_else(|| format!("{flag} needs a number"))?
        .parse()
        .map_err(|_| format!("{flag} wants a number"))
}

impl SkinChoice {
    fn parse(name: &str) -> Result<Self, String> {
        let path = Path::new(name);
        if path.is_dir() {
            return Ok(Self::Folder(path.to_path_buf()));
        }
        match name.to_ascii_lowercase().as_str() {
            "classic" | "map" => Ok(Self::Classic),

            "1984" | "dossier" => {
                Err("the `1984` skin was removed — use `classic`, or import a skin".to_owned())
            }
            other => Err(format!(
                "unknown skin `{other}` — try classic, or the path to a skin folder"
            )),
        }
    }

    fn visual(&self, beatmap: &Beatmap, effects: Option<&str>) -> Skin {
        let mut skin = Skin::with_combo_colours(beatmap.combo_colours());
        if let Self::Folder(path) = self {
            skin = dress(
                skin,
                path,
                effects.map(|list| Effects::asked_for(list, "slider-ball-tint")),
            );
        }

        if let Some(list) = effects {
            Effects::apply(&mut skin, list);
        }
        skin
    }

    fn visual_default(&self) -> Skin {
        let mut skin = Skin::default();
        if let Self::Folder(path) = self {
            skin = dress(skin, path, None);
        }
        skin
    }

    fn samples_dir(&self) -> Option<&Path> {
        match self {
            Self::Folder(path) => Some(path),
            Self::Classic => None,
        }
    }

    fn kit(&self) -> dossier_audio::Kit {
        match self {
            Self::Folder(_) | Self::Classic => dossier_audio::Kit::plain(),
        }
    }
}

fn dress(skin: Skin, path: &Path, tint_ball: Option<bool>) -> Skin {
    produce::skin::from_folder(skin, path, tint_ball)
}

impl Options {
    fn parse(command: Command, args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            replays: Vec::new(),
            map: None,
            songs: std::env::var_os("DOSSIER_SONGS_DIR").map(PathBuf::from),
            json: false,
            explain: false,
            trace: false,
            marginal: None,
            strict: false,
            corpus_ceiling: None,
            expect: None,
            update_expect: false,
            prune: false,
            events: false,
            leaderboard: None,
            my_avatar: None,
            my_cover: None,
            at_ms: None,
            slow_at_ms: None,
            background: false,
            storyboard: false,
            video: false,
            exhibit_budget_s: None,
            exhibit_worth: None,
            survey: false,
            bare: false,
            dim: None,
            meter_scale: None,
            cursor_scale: None,
            cursor_rotate: None,
            blur: None,
            skin_as_written: false,
            trace_hitsounds: false,
            map_hitsounds: true,
            music_level: 100,
            hitsound_level: 100,
            volume: 100,
            effects: None,
            exhibit_clip_s: None,
            out: PathBuf::from("frame.png"),
            size: (1920, 1080),
            font: std::env::var_os("DOSSIER_FONT").map(PathBuf::from),
            fps: 60.0,
            from_ms: None,
            to_ms: None,
            crf: 20,
            preset: "veryfast".to_owned(),
            ffmpeg: std::env::var("DOSSIER_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_owned()),
            mute: false,
            skin: SkinChoice::Classic,
            kit: None,
            samples: std::env::var_os("DOSSIER_SAMPLES").map(PathBuf::from),
            game_sounds: std::env::var_os("DOSSIER_GAME_SOUNDS").map(PathBuf::from),
            threads: None,
            encoder_threads: None,
            mods: None,
            accuracy: None,
            combo: None,
            misses: None,
            n300: None,
            n100: None,
            n50: None,
            slider_ends: None,
            large_tick_misses: None,
            classic: false,
            legacy_total: None,
            pitch: None,
            decay: None,
            level: None,
        };

        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            if arg.starts_with('-') && arg.as_str() != "-" && !command.accepts(canonical(arg)) {
                return Err(format!(
                    "`{}` has no option `{arg}` — see `dossier {} --help`",
                    command.name(),
                    command.name()
                ));
            }
            match arg.as_str() {
                "-m" | "--map" => {
                    options.map = Some(PathBuf::from(
                        rest.next().ok_or("--map needs a path")?.as_str(),
                    ));
                }
                "-s" | "--songs" => {
                    options.songs = Some(PathBuf::from(
                        rest.next().ok_or("--songs needs a path")?.as_str(),
                    ));
                }
                "-a" | "--at" => {
                    options.at_ms = Some(
                        rest.next()
                            .ok_or("--at needs a time in milliseconds")?
                            .parse()
                            .map_err(|_| "--at wants a number")?,
                    );
                }
                "--background" => options.background = true,
                "--storyboard" => options.storyboard = true,
                "--video" => options.video = true,
                "--slow" => {
                    options.slow_at_ms = Some(
                        rest.next()
                            .ok_or("--slow needs a time in milliseconds")?
                            .parse()
                            .map_err(|_| "--slow wants a number")?,
                    );
                }
                "--for" => {
                    options.exhibit_budget_s = Some(
                        rest.next()
                            .ok_or("--for needs a number of seconds")?
                            .parse()
                            .map_err(|_| "--for wants a number")?,
                    );
                }
                "--bare" => options.bare = true,
                "--skin-as-written" => options.skin_as_written = true,
                "--volume" => {
                    let at: u32 = rest
                        .next()
                        .ok_or("--volume needs a number from 0 to 200")?
                        .parse()
                        .map_err(|_| "--volume wants a number from 0 to 200")?;
                    if at > 200 {
                        return Err(format!("--volume runs from 0 to 200 — {at} is past it"));
                    }
                    options.volume = at;
                }
                "--no-map-hitsounds" => options.map_hitsounds = false,
                "--dim" => {
                    let level: u32 = rest
                        .next()
                        .ok_or("--dim needs a number from 0 to 100")?
                        .parse()
                        .map_err(|_| "--dim wants a number from 0 to 100")?;
                    if level > 100 {
                        return Err(format!("--dim is a percentage — {level} is past 100"));
                    }
                    options.dim = Some(level);
                }
                "--cursor-rotate" => {
                    let word = rest.next();
                    options.cursor_rotate = Some(match word.as_deref().map(|s| s.trim()) {
                        Some("on" | "1" | "yes") => true,
                        Some("off" | "0" | "no") => false,
                        Some(other) => {
                            return Err(format!("--cursor-rotate is on or off — not {other}"));
                        }
                        None => return Err("--cursor-rotate needs on or off".into()),
                    });
                }
                "--cursor-scale" => {
                    let at: f32 = rest
                        .next()
                        .ok_or("--cursor-scale needs a number from 0.4 to 2")?
                        .parse()
                        .map_err(|_| "--cursor-scale wants a number from 0.4 to 2")?;
                    if !(0.4..=2.0).contains(&at) {
                        return Err(format!(
                            "--cursor-scale runs from 0.4 to 2 — {at} is outside it"
                        ));
                    }
                    options.cursor_scale = Some(at);
                }
                "--blur" => {
                    let at: u32 = rest
                        .next()
                        .ok_or("--blur needs a number from 0 to 100")?
                        .parse()
                        .map_err(|_| "--blur wants a number from 0 to 100")?;
                    if at > 100 {
                        return Err(format!("--blur is a percentage — {at} is past 100"));
                    }
                    options.blur = Some(at);
                }
                "--meter-scale" => {
                    let at: f32 = rest
                        .next()
                        .ok_or("--meter-scale needs a number from 0.5 to 3")?
                        .parse()
                        .map_err(|_| "--meter-scale wants a number from 0.5 to 3")?;
                    if !(0.5..=3.0).contains(&at) {
                        return Err(format!(
                            "--meter-scale runs from 0.5 to 3 — {at} is outside it"
                        ));
                    }
                    options.meter_scale = Some(at);
                }
                "--mods" => {
                    options.mods = Some(
                        rest.next()
                            .ok_or("--mods needs acronyms, e.g. HDDT")?
                            .to_owned(),
                    );
                }
                "--classic" => options.classic = true,
                flag @ ("--accuracy"
                | "--combo"
                | "--misses"
                | "--n300"
                | "--n100"
                | "--n50"
                | "--slider-ends"
                | "--large-tick-misses"
                | "--legacy-total") => {
                    let raw = rest
                        .next()
                        .ok_or_else(|| format!("{flag} needs a number"))?;
                    let number: f64 = raw
                        .parse()
                        .map_err(|_| format!("{flag} wants a number, not {raw:?}"))?;
                    if number < 0.0 {
                        return Err(format!("{flag} cannot be negative"));
                    }
                    match flag {
                        "--accuracy" => options.accuracy = Some(number),
                        "--combo" => options.combo = Some(number as u32),
                        "--misses" => options.misses = Some(number as u32),
                        "--n300" => options.n300 = Some(number as u32),
                        "--n100" => options.n100 = Some(number as u32),
                        "--n50" => options.n50 = Some(number as u32),
                        "--slider-ends" => options.slider_ends = Some(number as u32),
                        "--large-tick-misses" => options.large_tick_misses = Some(number as u32),
                        _ => options.legacy_total = Some(number as u64),
                    }
                }
                "--trace-hitsounds" => options.trace_hitsounds = true,
                flag @ ("--music" | "--hitsounds") => {
                    let level: u32 = rest
                        .next()
                        .ok_or_else(|| format!("{flag} needs a number from 0 to 100"))?
                        .parse()
                        .map_err(|_| format!("{flag} wants a number from 0 to 100"))?;

                    if level > 100 {
                        return Err(format!("{flag} is a percentage — {level} is past 100"));
                    }
                    if flag == "--music" {
                        options.music_level = level;
                    } else {
                        options.hitsound_level = level;
                    }
                }
                "--effects" => {
                    options.effects = Some(
                        rest.next()
                            .ok_or("--effects needs a comma-separated list")?
                            .to_owned(),
                    );
                }
                "--survey" => options.survey = true,
                "--worth" => {
                    options.exhibit_worth = Some(
                        rest.next()
                            .ok_or("--worth needs a number between 0 and 1")?
                            .parse()
                            .map_err(|_| "--worth wants a number")?,
                    );
                }
                "--clip" => {
                    options.exhibit_clip_s = Some(
                        rest.next()
                            .ok_or("--clip needs a number of seconds")?
                            .parse()
                            .map_err(|_| "--clip wants a number")?,
                    );
                }
                "--fps" => {
                    options.fps = rest
                        .next()
                        .ok_or("--fps needs a number")?
                        .parse()
                        .map_err(|_| "--fps wants a number")?;
                }
                "--from" => {
                    options.from_ms = Some(
                        rest.next()
                            .ok_or("--from needs a time")?
                            .parse()
                            .map_err(|_| "--from wants a number")?,
                    );
                }
                "--to" => {
                    options.to_ms = Some(
                        rest.next()
                            .ok_or("--to needs a time")?
                            .parse()
                            .map_err(|_| "--to wants a number")?,
                    );
                }
                "--preset" => {
                    options.preset = rest.next().ok_or("--preset needs a name")?.clone();
                }
                "--crf" => {
                    options.crf = rest
                        .next()
                        .ok_or("--crf needs a number")?
                        .parse()
                        .map_err(|_| "--crf wants a number")?;
                }
                "--skin" => {
                    options.skin = SkinChoice::parse(rest.next().ok_or("--skin needs a name")?)?;
                }
                "--threads" => {
                    options.threads = Some(
                        rest.next()
                            .ok_or("--threads needs a number")?
                            .parse()
                            .map_err(|_| "--threads wants a number")?,
                    );
                }
                "--encoder-threads" => {
                    options.encoder_threads = Some(
                        rest.next()
                            .ok_or("--encoder-threads needs a number")?
                            .parse()
                            .map_err(|_| "--encoder-threads wants a number")?,
                    );
                }
                "--samples" => {
                    options.samples = Some(PathBuf::from(
                        rest.next().ok_or("--samples needs a path")?.as_str(),
                    ));
                }
                "--game-sounds" => {
                    options.game_sounds = Some(PathBuf::from(
                        rest.next().ok_or("--game-sounds needs a path")?.as_str(),
                    ));
                }
                "--kit" => {
                    let name = rest.next().ok_or("--kit needs a name")?;
                    options.kit = Some(dossier_audio::Kit::by_name(name).ok_or_else(|| {
                        format!("unknown kit `{name}` — try click, soft, drum, glass or wood")
                    })?);
                }
                "--pitch" => {
                    options.pitch = Some(parse_number(rest.next(), "--pitch")?);
                }
                "--decay" => {
                    options.decay = Some(parse_number(rest.next(), "--decay")?);
                }
                "--level" => {
                    options.level = Some(parse_number(rest.next(), "--level")?);
                }
                "--mute" => options.mute = true,
                "--ffmpeg" => {
                    options.ffmpeg = rest.next().ok_or("--ffmpeg needs a path")?.clone();
                }
                "--font" => {
                    options.font = Some(PathBuf::from(
                        rest.next().ok_or("--font needs a path")?.as_str(),
                    ));
                }
                "-o" | "--out" => {
                    options.out = PathBuf::from(rest.next().ok_or("--out needs a path")?.as_str());
                }
                "--size" => {
                    let raw = rest.next().ok_or("--size needs WxH")?;
                    let (w, h) = raw.split_once(['x', 'X']).ok_or("--size wants WxH")?;
                    options.size = (
                        w.parse().map_err(|_| "--size wants numbers")?,
                        h.parse().map_err(|_| "--size wants numbers")?,
                    );
                }
                "-j" | "--json" => options.json = true,
                "-e" | "--explain" => options.explain = true,
                "-t" | "--trace" => options.trace = true,
                "--marginal" => {
                    options.marginal = Some(
                        rest.next()
                            .ok_or("--marginal needs a count")?
                            .parse()
                            .map_err(|_| "--marginal needs a number")?,
                    );
                }

                "--strict" => match rest.clone().next().and_then(|n| n.parse::<u32>().ok()) {
                    Some(ceiling) => {
                        options.corpus_ceiling = Some(ceiling);
                        rest.next();
                    }
                    None => options.strict = true,
                },
                "--expect" => {
                    options.expect =
                        Some(PathBuf::from(rest.next().ok_or("--expect needs a path")?));
                }
                "--update-expect" => options.update_expect = true,
                "--prune" => options.prune = true,
                "--events" => options.events = true,
                "--leaderboard" => {
                    options.leaderboard = Some(PathBuf::from(
                        rest.next().ok_or("--leaderboard needs a path")?,
                    ));
                }
                "--my-pictures" => {
                    options.my_avatar = Some(PathBuf::from(
                        rest.next().ok_or("--my-pictures needs two paths")?,
                    ));
                    options.my_cover = Some(PathBuf::from(
                        rest.next().ok_or("--my-pictures needs two paths")?,
                    ));
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option `{other}`"));
                }
                path => options.replays.push(PathBuf::from(path)),
            }
        }

        if options.map.is_some() && options.replays.len() > 1 {
            return Err("--map judges one replay; drop it and use --songs for a batch".to_owned());
        }

        if options.prune && !options.update_expect {
            return Err("--prune only means something with --update-expect".to_owned());
        }
        Ok(options)
    }
}

fn default_measurers() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

fn corpus(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }

    struct Row {
        name: String,
        error: u32,
        combo: i64,

        score: Option<f64>,
        client: &'static str,

        md5: String,
        beatmap_md5: String,
    }
    let mut skipped = 0usize;

    let expected = match &options.expect {
        Some(path) if options.update_expect && !path.exists() => Some(BTreeMap::new()),
        Some(path) => match manifest::read(path) {
            Ok(rows) => Some(rows),
            Err(message) => {
                eprintln!("dossier: {message}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut duplicates = 0usize;
    let mut queue: Vec<(String, &PathBuf)> = Vec::new();

    for replay_path in &options.replays {
        let md5 = match std::fs::read(replay_path) {
            Ok(bytes) => locate::md5_hex(&bytes),
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if !seen.insert(md5.clone()) {
            duplicates += 1;
            continue;
        }
        queue.push((md5, replay_path));
    }

    let workers = options
        .threads
        .unwrap_or_else(default_measurers)
        .clamp(1, queue.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let measured: Vec<(Vec<(usize, Row)>, usize)> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..workers)
            .map(|_| {
                let (next, queue, options) = (&next, &queue, &options);
                scope.spawn(move || {
                    let mut mine: Vec<(usize, Row)> = Vec::new();
                    let mut missed = 0usize;
                    loop {
                        let at = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((md5, replay_path)) = queue.get(at) else {
                            break;
                        };
                        let Ok(report) = run_one(replay_path, options) else {
                            missed += 1;
                            continue;
                        };
                        let check = &report.check;
                        let (ours, theirs) = (check.ours, check.theirs);

                        let error = u32::from(ours.count_300).abs_diff(u32::from(theirs.count_300))
                            + u32::from(ours.count_100).abs_diff(u32::from(theirs.count_100))
                            + u32::from(ours.count_50).abs_diff(u32::from(theirs.count_50))
                            + u32::from(ours.count_miss).abs_diff(u32::from(theirs.count_miss));

                        let score = report.score_error;
                        mine.push((
                            at,
                            Row {
                                score,
                                name: replay_path.file_name().map_or_else(
                                    || replay_path.display().to_string(),
                                    |n| n.to_string_lossy().into_owned(),
                                ),
                                error,
                                combo: i64::from(check.our_max_combo)
                                    - i64::from(check.their_max_combo),
                                client: if report.client.starts_with("lazer") {
                                    "lazer"
                                } else {
                                    "stable"
                                },
                                beatmap_md5: report.beatmap_md5.clone(),
                                md5: md5.clone(),
                            },
                        ));
                    }
                    (mine, missed)
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().expect("a measuring thread panicked"))
            .collect()
    });

    let mut ordered: Vec<(usize, Row)> = Vec::with_capacity(queue.len());
    for (mine, missed) in measured {
        skipped += missed;
        ordered.extend(mine);
    }
    ordered.sort_unstable_by_key(|(at, _)| *at);
    let mut rows: Vec<Row> = ordered.into_iter().map(|(_, row)| row).collect();

    let total: u32 = rows.iter().map(|r| r.error).sum();
    let exact = rows.iter().filter(|r| r.error == 0 && r.combo == 0).count();
    let lazer = rows.iter().filter(|r| r.client == "lazer").count();

    rows.sort_by(|a, b| {
        b.error.cmp(&a.error).then(
            b.score
                .map_or(0.0, f64::abs)
                .total_cmp(&a.score.map_or(0.0, f64::abs)),
        )
    });
    let scored: Vec<f64> = rows.iter().filter_map(|r| r.score).map(f64::abs).collect();
    for row in rows
        .iter()
        .filter(|r| r.error != 0 || r.combo != 0 || r.score.is_some_and(|s| s.abs() >= 0.05))
    {
        let combo = if row.combo == 0 {
            String::new()
        } else {
            format!("  combo {:+}", row.combo)
        };
        let score = match row.score {
            Some(off) if off.abs() >= 0.05 => format!("  score {off:+.2}%"),
            _ => String::new(),
        };
        println!(
            "   {:>4}{combo:<13}{score:<16}  {:<6}  {}",
            row.error,
            row.client,
            row.name.chars().take(46).collect::<String>()
        );
    }

    let worst_score = scored.iter().copied().fold(0.0f64, f64::max);
    println!(
        "\n{exact} exact of {} ({lazer} lazer), total count error {total}, {skipped} skipped",
        rows.len()
    );
    if !scored.is_empty() {
        println!(
            "score compared on {}, worst {worst_score:.2}%, within 0.5% on {}",
            scored.len(),
            scored.iter().filter(|off| **off < 0.5).count()
        );
    }

    if duplicates > 0 {
        println!("{duplicates} duplicate replay file(s) counted once");
    }

    let mut regressed = 0usize;
    let mut absent = 0usize;
    if let (Some(expected), Some(path)) = (&expected, &options.expect) {
        if options.update_expect {
            let measured = rows
                .iter()
                .map(|row| manifest::Expectation {
                    replay_md5: row.md5.clone(),
                    beatmap_md5: row.beatmap_md5.clone(),
                    beatmap_id: None,
                    error: row.error,
                    combo: row.combo,
                    score: row.score,
                    name: row.name.clone(),
                })
                .collect();
            let (updated, dropped) = manifest::after_run(expected, measured, &seen, options.prune);
            let fresh = rows
                .iter()
                .filter(|row| !expected.contains_key(&row.md5))
                .count();
            match manifest::write(path, &updated) {
                Ok(()) => {
                    let kept = updated.len() - rows.len();
                    println!(
                        "\n{} rows written to {}: {} measured ({fresh} new), {kept} kept, {dropped} dropped",
                        updated.len(),
                        path.display(),
                        rows.len(),
                    );
                }
                Err(message) => {
                    eprintln!("dossier: {message}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            for row in expected.values() {
                if !seen.contains(&row.replay_md5) {
                    absent += 1;
                    println!(
                        "   ?? {}  {}",
                        &row.replay_md5[..12],
                        row.name.chars().take(46).collect::<String>()
                    );
                }
            }
            let mut unlisted = 0usize;
            for row in &rows {
                let Some(was) = expected.get(&row.md5) else {
                    unlisted += 1;
                    continue;
                };
                if let Some(what) = was.worse_than(row.error, row.combo, row.score) {
                    regressed += 1;
                    println!(
                        "   !! {}  {what}",
                        row.name.chars().take(46).collect::<String>()
                    );
                }
            }
            println!(
                "\nagainst {}: {} of {} rows present, {absent} absent, {unlisted} not listed, \
                 {regressed} worse",
                path.display(),
                expected.len() - absent,
                expected.len()
            );
        }
    }

    if regressed > 0 {
        eprintln!("dossier: {regressed} replay(s) got worse than the corpus says they are");
        return ExitCode::FAILURE;
    }

    if absent > 0 && options.strict {
        eprintln!("dossier: {absent} replay(s) of the corpus are not on this machine");
        return ExitCode::FAILURE;
    }
    match options.corpus_ceiling {
        Some(ceiling) if total > ceiling => {
            eprintln!("dossier: worse than the {ceiling} this was held to");
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}

fn judge(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    let mut failures = 0usize;
    let mut mismatches = 0usize;
    let mut exact = 0usize;

    for replay_path in &options.replays {
        match run_one(replay_path, &options) {
            Ok(report) => {
                if report.is_exact() {
                    exact += 1;
                } else {
                    mismatches += 1;
                }
                if options.json {
                    println!("{}", report.json());
                } else {
                    print!("{}", report.human());
                    if options.explain && !report.is_exact() {
                        print!("{}", report.explain());
                    }
                    if let Some(n) = options.marginal {
                        print!("{}", report.marginal(n));
                    }
                    if options.trace {
                        let window = match (options.from_ms, options.to_ms) {
                            (None, None) => None,
                            (from, to) => Some((from.unwrap_or(f64::MIN), to.unwrap_or(f64::MAX))),
                        };
                        print!("{}", report.trace(window));
                    }
                    println!();
                }
            }
            Err(message) => {
                failures += 1;
                if options.json {
                    println!(
                        "{}",
                        error_json(&replay_path.display().to_string(), &message)
                    );
                } else {
                    println!("── {}\n   SKIPPED: {message}\n", replay_path.display());
                }
            }
        }
    }

    if !options.json && options.replays.len() > 1 {
        println!("{exact} exact, {mismatches} mismatched, {failures} skipped");
    }

    if failures > 0 || (options.strict && mismatches > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn inspect(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    let mut failures = 0usize;
    for replay_path in &options.replays {
        let name = replay_path.display().to_string();
        match read_header(replay_path) {
            Ok(header) => {
                if options.json {
                    println!("{}", header.json());
                } else {
                    println!("{}", header.human());
                }
            }
            Err(message) => {
                failures += 1;
                if options.json {
                    println!("{}", error_json(&name, &message));
                } else {
                    println!("── {name}\n   ERROR: {message}\n");
                }
            }
        }
    }
    if failures > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn debug_command(options: Options) -> ExitCode {
    let Some(replay_path) = options.replays.first() else {
        eprintln!("dossier: debug needs a replay");
        return ExitCode::FAILURE;
    };
    let (Some(from), Some(to)) = (options.from_ms, options.to_ms) else {
        eprintln!("dossier: debug needs --from and --to — a whole replay is thousands of lines");
        return ExitCode::FAILURE;
    };

    let (beatmap, replay, source) = match load_found(replay_path, &options) {
        Ok(triple) => triple,
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    };
    let state = GameState::new(&beatmap, &replay);
    print!(
        "{}",
        debug::narrate(
            &replay_path.display().to_string(),
            &source,
            &beatmap,
            &replay,
            &state,
            (from, to),
        )
    );
    ExitCode::SUCCESS
}

fn sliders(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    for replay_path in &options.replays {
        let (beatmap, replay) = match load(replay_path, &options) {
            Ok(pair) => pair,
            Err(message) => {
                println!("── {}\n   SKIPPED: {message}\n", replay_path.display());
                continue;
            }
        };
        let state = GameState::new(&beatmap, &replay);
        let Some(judge) = state.judge() else {
            continue;
        };

        let mut verdicts = [0usize; 4];
        let mut dropped = [0usize; 4];
        let mut imperfect_without_a_dropped_tail = 0usize;

        let played = state.objects_played();

        for (index, object) in state.timeline().objects.iter().take(played).enumerate() {
            if !object.is_slider() {
                continue;
            }
            let mut verdict = Judgement::Great;
            let mut lost = [false; 4];
            for event in judge.events_for(index) {
                match event.part {
                    Part::Slider => verdict = event.result,
                    Part::SliderHead if event.result.is_miss() => lost[0] = true,
                    Part::SliderTick if event.result.is_miss() => lost[1] = true,
                    Part::SliderRepeat if event.result.is_miss() => lost[2] = true,
                    Part::SliderTail if event.result.is_miss() => lost[3] = true,
                    _ => {}
                }
            }

            verdicts[match verdict {
                Judgement::Great => 0,
                Judgement::Ok => 1,
                Judgement::Meh => 2,
                Judgement::Miss => 3,
            }] += 1;

            if verdict != Judgement::Great || lost.iter().any(|l| *l) {
                for (slot, was_lost) in lost.iter().enumerate() {
                    if *was_lost {
                        dropped[slot] += 1;
                    }
                }
                if !lost[3] && verdict != Judgement::Great {
                    imperfect_without_a_dropped_tail += 1;
                }
            }
        }

        let total: usize = verdicts.iter().sum();
        println!("── {}", replay_path.display());
        println!(
            "   {total} sliders: {} × 300, {} × 100, {} × 50, {} × miss",
            verdicts[0], verdicts[1], verdicts[2], verdicts[3]
        );
        println!(
            "   parts dropped on the rest: head {}, tick {}, repeat {}, tail {}",
            dropped[0], dropped[1], dropped[2], dropped[3]
        );
        println!("   downgraded without losing the tail: {imperfect_without_a_dropped_tail}");
        println!(
            "   tails credited only by the grace window: {}, only near the rim: {}",
            state.lenient_tails(),
            state.tails_near_the_rim()
        );

        for (index, object) in state.timeline().objects.iter().take(played).enumerate() {
            if !object.is_slider() {
                continue;
            }
            let verdict = judge
                .events_for(index)
                .find(|e| e.part == Part::Slider)
                .map(|e| e.result);
            let dropped: Vec<&str> = judge
                .events_for(index)
                .filter(|e| e.result.is_miss())
                .map(|e| match e.part {
                    Part::SliderHead => "head",
                    Part::SliderTick => "tick",
                    Part::SliderRepeat => "repeat",
                    _ => "tail",
                })
                .collect();
            if verdict.is_none() || (verdict == Some(Judgement::Great) && dropped.is_empty()) {
                continue;
            }
            let follow = state.difficulty().circle_radius() * 2.4;
            let mut trail = String::new();
            for offset in [-60.0, -48.0, -36.0, -24.0, -12.0, 0.0] {
                let t = object.end_ms + offset;
                let detail = match (object.ball_at(t), state.cursor_track().sample(t)) {
                    (Some(ball), Some(cursor)) => format!(
                        "{:+.0}ms {:.0}px{}",
                        offset,
                        cursor.pos.distance_to(ball),
                        if cursor.keys.is_pressed() { "" } else { " up" }
                    ),
                    _ => format!("{offset:+.0}ms —"),
                };
                trail.push_str(&format!("  {detail}"));
            }
            println!(
                "   #{index} at {:.0}ms, {:.0}ms long over {} slide(s) — lost {}",
                object.start_ms,
                object.duration_ms(),
                object.repeat_times().len() + 1,
                dropped.join(", ")
            );
            println!("      follow circle {follow:.0}px;{trail}");

            for event in judge.events_for(index).filter(|e| e.result.is_miss()) {
                let name = match event.part {
                    Part::SliderHead => "head",
                    Part::SliderTick => "tick",
                    Part::SliderRepeat => "repeat",
                    Part::SliderTail => "tail",
                    _ => continue,
                };
                let at = if event.part == Part::SliderTail {
                    dossier_sim::tail_check_ms(object)
                } else {
                    event.time_ms
                };
                let detail = match (object.ball_at(at), state.cursor_track().sample(at)) {
                    (Some(ball), Some(cursor)) => format!(
                        "ball ({:.0},{:.0}), cursor {:.1}px away{}",
                        ball.x,
                        ball.y,
                        cursor.pos.distance_to(ball),
                        if cursor.keys.is_pressed() {
                            ""
                        } else {
                            ", button up"
                        }
                    ),
                    _ => "no ball or no cursor there".to_owned(),
                };
                println!("         {name} at {at:.0}ms — {detail}");
            }
        }
        println!();
    }
    ExitCode::SUCCESS
}

fn health_command(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    let mut worst = 0f64;
    let mut total = 0f64;
    let mut counted = 0usize;
    for replay_path in &options.replays {
        let (beatmap, replay) = match load(replay_path, &options) {
            Ok(pair) => pair,
            Err(message) => {
                println!("── {}\n   SKIPPED: {message}\n", replay_path.display());
                continue;
            }
        };
        let graph = dossier_replay::life_points(&replay.life_bar);
        if graph.is_empty() {
            println!("── {}\n   no life-bar graph\n", replay_path.display());
            continue;
        }
        let state = GameState::new(&beatmap, &replay);
        let Some(judge) = state.judge() else {
            continue;
        };
        let ruleset = Ruleset::of_replay(&replay);
        let track = dossier_sim::HealthTrack::build(
            judge,
            state.timeline(),
            &beatmap.breaks,
            beatmap.format_version,
            replay.mods,
            ruleset,
        );

        let mut sum = 0f64;
        let mut signed = 0f64;
        let mut peak = 0f64;
        let mut divergences: Vec<(f64, f64, f32, f32)> = Vec::new();
        for &(time, theirs) in &graph {
            let ours = track.at(time);
            let d = f64::from((ours - theirs).abs());
            sum += d;
            signed += f64::from(ours - theirs);
            peak = peak.max(d);
            divergences.push((d, time, ours, theirs));
        }
        divergences.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mean = sum / graph.len() as f64;
        worst = worst.max(mean);
        total += mean;
        counted += 1;
        println!("── {}", replay_path.display());
        println!(
            "   {:?}  HP {:.1}  samples {}  mean {:.3}  bias {:+.3}  worst {:.3}  drain {:.5}",
            ruleset,
            beatmap.difficulty.hp_drain,
            graph.len(),
            mean,
            signed / graph.len() as f64,
            peak,
            track.drain_rate(),
        );
        if options.trace {
            let mut last = 0f64;
            for &(time, theirs) in &graph {
                let ours = track.at(time);
                let gap = f64::from(ours - theirs);
                println!(
                    "      {time:>8.0}ms  ours {ours:.3}  theirs {theirs:.3}  gap {gap:+.3}  step {:+.3}",
                    gap - last
                );
                last = gap;
            }
        } else {
            for &(d, time, ours, theirs) in divergences.iter().take(4) {
                println!("      {time:>8.0}ms  ours {ours:.3}  theirs {theirs:.3}  off {d:.3}");
            }
        }
    }
    if counted > 1 {
        println!(
            "\nmean {:.3} across {counted}, worst replay {worst:.3}",
            total / counted as f64
        );
    }
    ExitCode::SUCCESS
}

fn score_command(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    let mut worst = 0f64;
    let mut counted = 0usize;
    for replay_path in &options.replays {
        let (beatmap, replay) = match load(replay_path, &options) {
            Ok(pair) => pair,
            Err(message) => {
                println!("── {}\n   SKIPPED: {message}\n", replay_path.display());
                continue;
            }
        };
        let state = GameState::new(&beatmap, &replay);
        let Some(judge) = state.judge() else {
            continue;
        };
        let ruleset = Ruleset::of_replay(&replay);

        let Some(track) = state.score_track() else {
            continue;
        };

        let theirs = i64::from(replay.score);
        let ours = track.total() as i64;
        let off = if theirs > 0 {
            (ours - theirs) as f64 / theirs as f64 * 100.0
        } else {
            0.0
        };
        if theirs > 0 {
            worst = worst.max(off.abs());
            counted += 1;
        }
        println!("── {}", replay_path.display());
        let (flat, combo_units) = dossier_sim::score::stable_halves(judge);
        let mods = dossier_sim::score::stable_mod_multiplier(replay.mods);
        let fitted = if combo_units > 0.0 && mods > 0.0 {
            (theirs as f64 - flat) / combo_units / mods
        } else {
            f64::NAN
        };
        let drain = dossier_sim::score::drain_seconds(&beatmap);
        println!(
            "   {:?}  mods {:#x}  ×{}  ours {ours}  theirs {theirs}  off {off:+.2}%  fitted ×{fitted:.3}",
            ruleset,
            replay.mods.0,
            dossier_sim::score::difficulty_multiplier(&beatmap, beatmap.objects.len(), drain),
        );
        println!(
            "   HP {:.1} OD {:.1} CS {:.1}  objects {}  drain {drain:.1}s  density {:.2}  combo {}/{}",
            beatmap.difficulty.hp_drain,
            beatmap.difficulty.overall_difficulty,
            beatmap.difficulty.circle_size,
            beatmap.objects.len(),
            (beatmap.objects.len() as f64 / drain * 8.0).clamp(0.0, 16.0),
            judge.final_state().max_combo,
            replay.max_combo,
        );
    }
    if counted > 1 {
        println!("\nworst {worst:.2}% across {counted}");
    }
    ExitCode::SUCCESS
}

fn errors(options: Options) -> ExitCode {
    if options.replays.is_empty() {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    }
    for replay_path in &options.replays {
        let (beatmap, replay) = match load(replay_path, &options) {
            Ok(pair) => pair,
            Err(message) => {
                println!("── {}\n   SKIPPED: {message}\n", replay_path.display());
                continue;
            }
        };
        let state = GameState::new(&beatmap, &replay);
        let Some(judge) = state.judge() else {
            continue;
        };
        let difficulty = state.difficulty();

        let mut histogram = std::collections::BTreeMap::<i64, usize>::new();
        for event in judge.events() {
            if event.part != Part::Circle {
                continue;
            }
            if let Some(error) = event.error_ms {
                *histogram.entry(error.abs().round() as i64).or_default() += 1;
            }
        }

        println!("── {}", replay_path.display());
        println!(
            "   presses {}   objects {}   hit by the replay {}",
            state.press_count(),
            state.timeline().objects.len(),
            replay.hits.total_hits() - u32::from(replay.hits.count_miss),
        );
        println!(
            "   OD {:.1}  windows {:.0} / {:.0} / {:.0}",
            difficulty.overall_difficulty,
            difficulty.hit_window_300(),
            difficulty.hit_window_100(),
            difficulty.hit_window_50()
        );
        for (label, window) in [
            ("300", difficulty.hit_window_300()),
            ("100", difficulty.hit_window_100()),
        ] {
            let edge = window.round() as i64;
            let counts: Vec<String> = (edge - 2..=edge + 2)
                .map(|ms| format!("{ms}ms:{}", histogram.get(&ms).copied().unwrap_or(0)))
                .collect();
            println!("   around the {label} edge  {}", counts.join("  "));
        }
        println!();
    }
    ExitCode::SUCCESS
}

fn load(replay_path: &Path, options: &Options) -> Result<(Beatmap, Replay), String> {
    load_with_origin(replay_path, options).map(|(b, r, _, _)| (b, r))
}

fn load_found(replay_path: &Path, options: &Options) -> Result<(Beatmap, Replay, String), String> {
    let bytes = std::fs::read(replay_path).map_err(|e| format!("{e}"))?;
    let replay = Replay::parse(&bytes).map_err(|e| format!("{e}"))?;
    let found = match &options.map {
        Some(path) => locate::load_map(path, &replay.beatmap_hash)?,
        None => {
            let songs = options
                .songs
                .as_ref()
                .ok_or("no --map and no --songs to search")?;
            locate::search_dir(songs, &replay.beatmap_hash)?
                .ok_or_else(|| format!("map {} not found", replay.beatmap_hash))?
        }
    };
    let beatmap = Beatmap::parse(&found.text).map_err(|e| format!("{e}"))?;
    Ok((beatmap, replay, found.source))
}

fn load_with_origin(
    replay_path: &Path,
    options: &Options,
) -> Result<(Beatmap, Replay, locate::Origin, String), String> {
    locate::load(
        replay_path,
        options.map.as_deref(),
        options.songs.as_deref(),
    )
}

fn client_name(replay: &Replay) -> String {
    let ruleset = dossier_sim::Ruleset::of_replay(replay);
    format!("{} {}", ruleset.name(), replay.game_version)
}

fn read_header(replay_path: &Path) -> Result<Header, String> {
    let bytes = std::fs::read(replay_path).map_err(|e| format!("{e}"))?;
    let replay = Replay::parse(&bytes).map_err(|e| format!("{e}"))?;
    Ok(Header {
        client: client_name(&replay),
        replay_path: replay_path.display().to_string(),
        player: replay.player.clone(),
        mode: format!("{:?}", replay.mode),
        mods: replay.mods.to_string(),
        beatmap_hash: replay.beatmap_hash.clone(),
        counts: replay.hits,
        max_combo: u32::from(replay.max_combo),
        frames: replay.frames.len(),
        duration_ms: replay.duration_ms(),
        lazer_mods: replay
            .lazer_mods()
            .iter()
            .map(|m| {
                if m.settings.is_empty() {
                    m.acronym.clone()
                } else {
                    let settings: Vec<String> = m
                        .settings
                        .iter()
                        .map(|(k, v)| format!("{k}={v:?}"))
                        .collect();
                    format!("{}({})", m.acronym, settings.join(","))
                }
            })
            .collect(),
        statistics: replay
            .score_info
            .as_ref()
            .map(|info| {
                info.statistics
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn run_one(replay_path: &Path, options: &Options) -> Result<Report, String> {
    let bytes = std::fs::read(replay_path).map_err(|e| format!("{e}"))?;
    let replay = Replay::parse(&bytes).map_err(|e| format!("{e}"))?;

    if replay.mode != GameMode::Standard {
        return Err(format!("{:?} replays aren't simulated yet", replay.mode));
    }
    if replay.frames.is_empty() {
        return Err("replay has no frames".to_owned());
    }

    let found = match &options.map {
        Some(path) => locate::load_map(path, &replay.beatmap_hash)?,
        None => {
            let songs = options
                .songs
                .as_ref()
                .ok_or("no --map and no --songs to search")?;
            locate::search_dir(songs, &replay.beatmap_hash)?.ok_or_else(|| {
                format!(
                    "map {} not found under {}",
                    replay.beatmap_hash,
                    songs.display()
                )
            })?
        }
    };

    let beatmap = Beatmap::parse(&found.text).map_err(|e| format!("{e}"))?;
    let state = GameState::new(&beatmap, &replay);
    let check = state
        .verify(&replay)
        .ok_or("nothing to verify — no replay attached")?;

    Ok(Report {
        replay_path: replay_path.display().to_string(),
        map_source: found.source,
        beatmap_md5: replay.beatmap_hash.clone(),
        title: format!(
            "{} - {} [{}]",
            beatmap.metadata.artist, beatmap.metadata.title, beatmap.metadata.version
        ),
        player: replay.player.clone(),
        mods: replay.mods.to_string(),
        objects: beatmap.object_count(),
        client: client_name(&replay),
        our_accuracy: check.ours.accuracy_std(),
        their_accuracy: check.theirs.accuracy_std(),
        check,
        misses: state.explain_misses(),
        lenient_tails: state.lenient_tails(),
        tails_near_the_rim: state.tails_near_the_rim(),
        max_possible_combo: state.max_possible_combo(),
        combo_chains: state.combo_chains(),
        combo_suspects: state.combo_break_suspects(u32::from(replay.max_combo)),
        presses: state.press_verdicts(),
        press_detail: state.press_detail(),
        window_50: state.difficulty().hit_window_50(),
        parts: part_checks(&state, &replay),
        score_error: state
            .score_track()
            .filter(|track| track.comparable())
            .filter(|_| replay.score > 0)
            .filter(|_| {
                let (flat, _) = dossier_sim::score::stable_halves(state.judge().unwrap());
                flat <= 0.0 || f64::from(replay.score) >= flat * MIN_CREDIBLE_SHARE
            })
            .map(|track| {
                (track.total() as f64 - f64::from(replay.score)) / f64::from(replay.score) * 100.0
            }),
    })
}

const MIN_CREDIBLE_SHARE: f64 = 0.05;

fn part_checks(state: &GameState, replay: &Replay) -> Vec<PartCheck> {
    let Some(theirs) = replay.score_info.as_ref().map(|info| &info.statistics) else {
        return Vec::new();
    };
    if theirs.is_empty() {
        return Vec::new();
    }
    let Some(judge) = state.judge() else {
        return Vec::new();
    };

    let mut great = 0i64;
    let mut ok = 0i64;
    let mut meh = 0i64;
    let mut miss = 0i64;
    let mut large_tick_hit = 0i64;
    let mut large_tick_miss = 0i64;
    let mut slider_tail_hit = 0i64;
    let mut ignore_hit = 0i64;
    let mut ignore_miss = 0i64;

    let mut slider_alive = false;
    let mut in_slider = false;

    let close_slider = |alive: bool, hit: &mut i64, missed: &mut i64| {
        if alive {
            *hit += 1;
        } else {
            *missed += 1;
        }
    };

    for event in judge.events() {
        match event.part {
            Part::Circle | Part::Spinner | Part::Slider => match event.result {
                Judgement::Great => great += 1,
                Judgement::Ok => ok += 1,
                Judgement::Meh => meh += 1,
                Judgement::Miss => miss += 1,
            },
            Part::SliderHead => {
                if in_slider {
                    close_slider(slider_alive, &mut ignore_hit, &mut ignore_miss);
                }
                in_slider = true;
                slider_alive = !event.result.is_miss();
            }

            Part::SpinnerSpin | Part::SpinnerPoints | Part::SpinnerBonus => {}
            Part::SliderTick | Part::SliderRepeat => {
                if event.result.is_miss() {
                    large_tick_miss += 1;
                } else {
                    large_tick_hit += 1;
                    slider_alive = true;
                }
            }
            Part::SliderTail => {
                if event.result.is_miss() {
                    ignore_miss += 1;
                } else {
                    slider_tail_hit += 1;
                    slider_alive = true;
                }
            }
        }
    }
    if in_slider {
        close_slider(slider_alive, &mut ignore_hit, &mut ignore_miss);
    }

    [
        ("great", great),
        ("ok", ok),
        ("meh", meh),
        ("miss", miss),
        ("large_tick_hit", large_tick_hit),
        ("large_tick_miss", large_tick_miss),
        ("slider_tail_hit", slider_tail_hit),
        ("ignore_hit", ignore_hit),
        ("ignore_miss", ignore_miss),
    ]
    .into_iter()
    .filter(|(name, ours)| *ours != 0 || theirs.contains_key(*name))
    .map(|(name, ours)| PartCheck {
        name: name.to_owned(),
        ours,
        theirs: theirs.get(name).copied().unwrap_or(0),
    })
    .collect()
}

fn survey(options: &Options) -> ExitCode {
    let settings = exhibit::settings(
        options.exhibit_budget_s,
        options.exhibit_clip_s,
        options.exhibit_worth,
    );
    let mut survey = exhibit::Survey::default();
    for path in &options.replays {
        let Ok((beatmap, replay)) = load(path, options) else {
            survey.skipped += 1;
            continue;
        };
        let state = GameState::new(&beatmap, &replay);
        survey.add(
            &dossier_exhibit::choose(&state, settings),
            state.playback_rate(),
        );
    }
    print!("{}", survey.report());
    ExitCode::SUCCESS
}

fn exhibit_command(options: Options) -> ExitCode {
    let Some(replay_path) = options.replays.first() else {
        eprintln!("dossier: exhibit needs a replay");
        return ExitCode::FAILURE;
    };
    if options.survey {
        return survey(&options);
    }

    let (beatmap, replay, origin, map_text) = match load_with_origin(replay_path, &options) {
        Ok(triple) => triple,
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    };

    let state = GameState::new(&beatmap, &replay);
    let settings = exhibit::settings(
        options.exhibit_budget_s,
        options.exhibit_clip_s,
        options.exhibit_worth,
    );
    let clips = dossier_exhibit::choose(&state, settings);

    if options.json {
        println!(
            "{}",
            exhibit::as_json(&replay_path.display().to_string(), &replay, &state, &clips)
        );
    } else if options.events {
        eprint!("{}", exhibit::as_text(&clips, state.playback_rate()));
    } else {
        print!("{}", exhibit::as_text(&clips, state.playback_rate()));
    }

    if clips.is_empty() {
        eprintln!("dossier: no clip fits — the play is shorter than one clip");
        return ExitCode::SUCCESS;
    }

    if options.out == Path::new("frame.png") {
        return ExitCode::SUCCESS;
    }
    let out = options.out.clone();
    if let Err(message) = video::check_output(&out) {
        eprintln!("dossier: {message}");
        return ExitCode::FAILURE;
    }

    let mut skin = options.look(&beatmap);

    let layering = skin
        .sprites
        .as_ref()
        .is_none_or(|s| s.ini().layered_hit_sounds);
    match load_font(options.font.as_deref()) {
        Ok(Some(font)) => skin = skin.with_font(font),
        Ok(None) => eprintln!("dossier: no font found — drawing without numbers"),
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    }

    let scratch = Scratch::new();
    let audio = if options.mute {
        None
    } else {
        let found = scratch
            .as_ref()
            .and_then(|dir| locate::extract_audio(&origin, &beatmap.audio_filename, dir));
        if found.is_none() {
            eprintln!("dossier: no audio track found — rendering silent");
        }
        found
    };

    let scene = Scene::new(&state, skin)
        .signed_by(&replay)
        .with_leaderboard(
            load_leaderboard(options.leaderboard.as_deref(), &replay.player)
                .with_own_pictures(options.my_avatar.clone(), options.my_cover.clone()),
        );
    let scene = if options.bare { scene.bare() } else { scene };

    let fired = hitsounds::sounded(&state, &beatmap, layering);
    let scene = scenery::dress(
        scene,
        &options.behind(None, scratch.as_ref()),
        &beatmap,
        (&map_text, &origin),
        &fired,
    );
    let settings = video::Settings {
        out,
        fps: options.fps,
        size: options.size,
        from_ms: None,
        to_ms: None,
        ffmpeg: options.ffmpeg.clone(),
        crf: options.crf,
        preset: options.preset.clone(),
        music_level: options.levels().0,
        hitsound_level: options.levels().1,
        threads: options.threads,
        encoder_threads: options.encoder_threads,
        audio,

        video: scenery::film(
            &options.behind(None, scratch.as_ref()),
            &map_text,
            &origin,
            scene.skin(),
        ),
        hitsounds: None,
        events: events::Events::wanted(options.events),

        slow_at_ms: None,
        slow_focus: None,
    };

    let (kit, pack) = (
        options.kit(),
        options.samples_with_map(&origin, scratch.as_ref()),
    );
    let muted = options.mute;
    let sounds = |plan: &video::Plan, index: usize| -> Option<PathBuf> {
        if muted {
            return None;
        }
        write_hitsounds_as(
            &state,
            &beatmap,
            plan,
            kit,
            pack.clone(),
            scratch.as_ref(),
            &format!("hitsounds-{index}.pcm"),
            layering,
        )
    };

    eprintln!(
        "{} — {} [{}], {} · {} clips · {}",
        replay.player,
        beatmap.metadata.title,
        beatmap.metadata.version,
        replay.mods,
        clips.len(),
        settings.out.display()
    );

    match reel::render(&scene, &state, &clips, &settings, scratch.as_ref(), &sounds) {
        Ok(()) => {
            let size = std::fs::metadata(&settings.out)
                .map(|m| m.len())
                .unwrap_or(0);

            if options.events {
                settings.events.wrote(&settings.out, size);
            } else {
                println!(
                    "{} — {:.1} MB",
                    settings.out.display(),
                    size as f64 / 1_048_576.0
                );
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("dossier: {message}");
            ExitCode::FAILURE
        }
    }
}

fn frame(options: Options) -> ExitCode {
    let Some(at_ms) = options.at_ms else {
        eprintln!("dossier: frame needs --at <ms>");
        return ExitCode::FAILURE;
    };
    let Some(replay_path) = options.replays.first() else {
        eprintln!("dossier: frame needs a replay");
        return ExitCode::FAILURE;
    };

    let (beatmap, replay, origin, map_text) = match load_with_origin(replay_path, &options) {
        Ok(triple) => triple,
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    };

    let state = GameState::new(&beatmap, &replay);
    let mut skin = options.look(&beatmap);

    let layering = skin
        .sprites
        .as_ref()
        .is_none_or(|s| s.ini().layered_hit_sounds);
    match load_font(options.font.as_deref()) {
        Ok(Some(font)) => skin = skin.with_font(font),
        Ok(None) => eprintln!("dossier: no font found — drawing without numbers"),
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    }
    let scene = Scene::new(&state, skin)
        .signed_by(&replay)
        .with_leaderboard(
            load_leaderboard(options.leaderboard.as_deref(), &replay.player)
                .with_own_pictures(options.my_avatar.clone(), options.my_cover.clone()),
        );
    let scene = if options.bare { scene.bare() } else { scene };

    let scratch = Scratch::new();
    let dressed = options.behind(Some(at_ms), scratch.as_ref());

    let fired = hitsounds::sounded(&state, &beatmap, layering);
    let scene = scenery::dress(scene, &dressed, &beatmap, (&map_text, &origin), &fired);
    let layout = Layout::new(options.size.0, options.size.1);
    let pixmap = scene.frame(at_ms, &layout);

    match pixmap
        .encode_png()
        .map_err(|e| e.to_string())
        .and_then(|png| std::fs::write(&options.out, png).map_err(|e| e.to_string()))
    {
        Ok(()) => {
            let score = state.update(at_ms).score;
            println!(
                "{} — {}ms, {}×{}{}",
                options.out.display(),
                at_ms,
                options.size.0,
                options.size.1,
                match score {
                    Some(s) => format!(", {}x {:.2}%", s.combo, s.accuracy()),
                    None => String::new(),
                }
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("dossier: {message}");
            ExitCode::FAILURE
        }
    }
}

fn load_leaderboard(path: Option<&Path>, player: &str) -> dossier_render::Leaderboard {
    let Some(path) = path else {
        return dossier_render::Leaderboard::default();
    };
    match std::fs::read_to_string(path) {
        Ok(text) => dossier_render::Leaderboard::parse(&text, player),
        Err(error) => {
            eprintln!(
                "dossier: {}: {error} — drawing without a scoreboard",
                path.display()
            );
            dossier_render::Leaderboard::default()
        }
    }
}

fn load_font(explicit: Option<&Path>) -> Result<Option<dossier_render::Font>, String> {
    produce::font::find(explicit)
}

fn video_command(options: Options) -> ExitCode {
    let Some(replay_path) = options.replays.first() else {
        eprintln!("dossier: no replay given");
        return ExitCode::FAILURE;
    };
    let out = if options.out == Path::new("frame.png") {
        PathBuf::from("render.mp4")
    } else {
        options.out.clone()
    };
    if let Err(message) = produce::render::check_output(&out) {
        eprintln!("dossier: {message}");
        return ExitCode::FAILURE;
    }

    let (beatmap, replay, origin, map_text) = match load_with_origin(replay_path, &options) {
        Ok(loaded) => loaded,
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    };

    let state = GameState::new(&beatmap, &replay);
    let mut skin = options.look(&beatmap);

    let layering = skin
        .sprites
        .as_ref()
        .is_none_or(|s| s.ini().layered_hit_sounds);
    match load_font(options.font.as_deref()) {
        Ok(Some(font)) => skin = skin.with_font(font),
        Ok(None) => eprintln!("dossier: no font found — drawing without numbers"),
        Err(message) => {
            eprintln!("dossier: {message}");
            return ExitCode::FAILURE;
        }
    }

    let scratch = Scratch::new();
    let audio = if options.mute {
        None
    } else {
        let found = scratch
            .as_ref()
            .and_then(|dir| locate::extract_audio(&origin, &beatmap.audio_filename, dir));
        if found.is_none() {
            eprintln!("dossier: no audio track found — rendering silent");
        }
        found
    };

    eprintln!(
        "{} — {} [{}], {} · {}",
        replay.player,
        beatmap.metadata.title,
        beatmap.metadata.version,
        replay.mods,
        out.display()
    );

    let settings = video::Settings {
        out,
        fps: options.fps,
        size: options.size,
        from_ms: options.from_ms,
        to_ms: options.to_ms,
        ffmpeg: options.ffmpeg.clone(),
        crf: options.crf,
        preset: options.preset.clone(),
        music_level: options.levels().0,
        hitsound_level: options.levels().1,
        threads: options.threads,
        encoder_threads: options.encoder_threads,
        audio,
        video: None,
        hitsounds: None,
        events: events::Events::wanted(options.events),
        slow_at_ms: options.slow_at_ms,
        slow_focus: None,
    };
    let job = produce::render::Job {
        state: &state,
        beatmap: &beatmap,
        replay: &replay,
        map_text: &map_text,
        origin: &origin,
        skin,
        leaderboard: load_leaderboard(options.leaderboard.as_deref(), &replay.player)
            .with_own_pictures(options.my_avatar.clone(), options.my_cover.clone()),
        bare: options.bare,
        layering,
        behind: options.behind(None, scratch.as_ref()),
        settings,
    };

    let kit = options.kit();
    let pack = options.samples_with_map(&origin, scratch.as_ref());
    let track = |plan: &video::Plan| {
        if options.mute {
            return None;
        }
        write_hitsounds(
            &state,
            &beatmap,
            plan,
            kit,
            pack.clone(),
            scratch.as_ref(),
            options.trace_hitsounds,
            layering,
        )
    };

    match produce::render::render(job, &track) {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("dossier: {message}");
            ExitCode::FAILURE
        }
    }
}

fn scratch_of_ours(name: &str) -> bool {
    name.strip_prefix("dossier-")
        .is_some_and(|pid| !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()))
}

const SCRATCH_STALE_HOURS: u64 = 6;

struct Scratch(Option<PathBuf>);

impl Scratch {
    fn new() -> Self {
        let temp = std::env::temp_dir();
        Self::sweep(&temp);
        let path = temp.join(format!("dossier-{}", std::process::id()));
        Self(std::fs::create_dir_all(&path).ok().map(|()| path))
    }

    fn sweep(temp: &Path) {
        let Ok(entries) = std::fs::read_dir(temp) else {
            return;
        };
        let stale = std::time::Duration::from_secs(SCRATCH_STALE_HOURS * 3600);
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !name.to_str().is_some_and(scratch_of_ours) {
                continue;
            }
            let old = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .and_then(|at| at.elapsed().map_err(std::io::Error::other))
                .is_ok_and(|since| since > stale);
            if old {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    fn as_ref(&self) -> Option<&Path> {
        self.0.as_deref()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn write_hitsounds(
    state: &GameState,
    beatmap: &Beatmap,
    plan: &video::Plan,
    kit: dossier_audio::Kit,
    pack: dossier_audio::SamplePack,
    scratch: Option<&Path>,
    trace: bool,
    layering: bool,
) -> Option<PathBuf> {
    let track = hitsounds::build(
        state,
        beatmap,
        |map_ms| plan.video_time_of(map_ms),
        plan.video_seconds,
        kit,
        pack,
        layering,
    );
    if track.is_empty() {
        return None;
    }
    report_silences(&track);
    if trace {
        report_resolution(&track);
    }
    let path = scratch?.join("hitsounds.pcm");
    std::fs::write(&path, track.to_pcm()).ok()?;
    Some(path)
}

fn report_resolution(track: &dossier_audio::Track) {
    let mut rows: Vec<_> = track.resolved().collect();
    if rows.is_empty() {
        return;
    }
    rows.sort_by_key(|&((set, voice, index), _, count)| {
        (
            std::cmp::Reverse(count),
            set.name(),
            voice.file_name(),
            index,
        )
    });
    eprintln!("dossier: hit sounds, as resolved —");
    for ((set, voice, index), found, count) in rows {
        let asked = if voice.banked() {
            let suffix = if index > 1 {
                index.to_string()
            } else {
                String::new()
            };
            format!("{}-{}{suffix}", set.name(), voice.file_name())
        } else {
            voice.file_name().to_owned()
        };
        eprintln!("   {asked:24} ×{count:<5} {}", found.describe());
    }
}

fn report_unused(pack: &dossier_audio::SamplePack) {
    let unused = pack.unused();
    if unused.is_empty() {
        return;
    }

    let suspect: Vec<&String> = unused
        .iter()
        .filter(|name| {
            ["normal", "soft", "drum"]
                .iter()
                .any(|bank| name.starts_with(bank))
        })
        .collect();
    eprint!(
        "dossier: {} file(s) in the skin no voice uses",
        unused.len()
    );
    if suspect.is_empty() {
        eprintln!(" — menu sounds, which never play here");
    } else {
        eprintln!(
            " — including {}, which osu! does not read either",
            suspect
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn report_silences(track: &dossier_audio::Track) {
    let mut said: Vec<String> = track
        .silenced()
        .map(|((set, voice), count)| {
            if voice.banked() {
                format!("{}-{} ×{count}", set.name(), voice.file_name())
            } else {
                format!("{} ×{count}", voice.file_name())
            }
        })
        .collect();
    if said.is_empty() {
        return;
    }
    said.sort();

    eprintln!(
        "dossier: the skin blanks {} — removed on purpose, so they are not heard",
        said.join(", ")
    );
}

fn write_hitsounds_as(
    state: &GameState,
    beatmap: &Beatmap,
    plan: &video::Plan,
    kit: dossier_audio::Kit,
    pack: dossier_audio::SamplePack,
    scratch: Option<&Path>,
    name: &str,
    layering: bool,
) -> Option<PathBuf> {
    let track = hitsounds::build(
        state,
        beatmap,
        |map_ms| plan.video_time_of(map_ms),
        plan.video_seconds,
        kit,
        pack,
        layering,
    );
    if track.is_empty() {
        return None;
    }
    let path = scratch?.join(name);
    std::fs::write(&path, track.to_pcm()).ok()?;
    Some(path)
}

fn skin_command(options: Options) -> ExitCode {
    let folder = if options.out == Path::new("frame.png") {
        eprintln!("dossier: skin needs somewhere to write — pass -o <folder>");
        return ExitCode::FAILURE;
    } else {
        options.out.clone()
    };

    let name = match &options.skin {
        SkinChoice::Folder(path) => path
            .file_name()
            .map_or("dossier", |n| n.to_str().unwrap_or("dossier")),
        SkinChoice::Classic => "dossier",
    };

    let mut skin = options.skin.visual_default();
    match load_font(options.font.as_deref()) {
        Ok(Some(font)) => skin = skin.with_font(font),
        Ok(None) => eprintln!("dossier: no font found — the combo digits are left to the game"),
        Err(error) => {
            eprintln!("dossier: {error}");
            return ExitCode::FAILURE;
        }
    }

    let samples = options.samples_folder();
    match skinfile::write(&skin, name, &folder, samples.as_deref()) {
        Ok(written) => {
            println!(
                "{} — skin.ini, {} image(s) and {} sound(s)",
                written.folder.display(),
                written.images,
                written.sounds
            );
            eprintln!(
                "   drop it in osu!/Skins/ — anything not written falls back to the game's own"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("dossier: {error}");
            ExitCode::FAILURE
        }
    }
}

fn sounds(options: Options) -> ExitCode {
    let kit = options.kit();
    let out = if options.out == Path::new("frame.png") {
        PathBuf::from("kit.wav")
    } else {
        options.out.clone()
    };

    let track = hitsounds::audition(kit, options.samples());
    match std::fs::write(&out, track.to_wav()) {
        Ok(()) => {
            println!(
                "{} — {:.1}s, pitch {:.2} decay {:.2} level {:.2}",
                out.display(),
                track.seconds(),
                kit.pitch,
                kit.decay,
                kit.level
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("dossier: {}: {error}", out.display());
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod scratch_names {
    use super::scratch_of_ours;

    #[test]
    fn only_our_own_scratch_directories_are_ours() {
        assert!(scratch_of_ours("dossier-1"));
        assert!(scratch_of_ours("dossier-71847"));

        assert!(!scratch_of_ours("dossier-notapid"));
        assert!(!scratch_of_ours("dossier-12a"));
        assert!(!scratch_of_ours("dossier"));
        assert!(!scratch_of_ours("not-dossier-1"));
        assert!(!scratch_of_ours(""));
        assert!(!scratch_of_ours("com.apple.launchd.abc"));
    }

    #[test]
    fn a_prefix_with_no_pid_is_not_ours() {
        assert!(!scratch_of_ours("dossier-"));
    }
}

#[cfg(test)]
mod options_per_command {
    use super::{canonical, Command, Options, OPTIONS_TABLE};

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|i| (*i).to_owned()).collect()
    }

    #[test]
    fn a_command_refuses_an_option_that_is_not_its_own() {
        match Options::parse(Command::Judge, &s(&["--crf", "18", "r.osr"])) {
            Err(error) => assert!(error.contains("`judge` has no option `--crf`"), "{error}"),
            Ok(_) => panic!("judge should refuse --crf"),
        }

        assert!(Options::parse(Command::Inspect, &s(&["--songs", "d", "r.osr"])).is_err());
        assert!(Options::parse(Command::Sounds, &s(&["--map", "m.osu"])).is_err());
    }

    #[test]
    fn a_command_takes_its_own_options() {
        assert!(Options::parse(Command::Video, &s(&["--crf", "18", "r.osr"])).is_ok());
        assert!(Options::parse(Command::Judge, &s(&["--songs", "d", "r.osr"])).is_ok());
        assert!(Options::parse(Command::Judge, &s(&["-s", "d", "r.osr"])).is_ok());
        assert!(Options::parse(Command::Inspect, &s(&["--json", "r.osr"])).is_ok());
    }

    #[test]
    fn a_replay_path_is_not_mistaken_for_an_option() {
        assert!(Options::parse(Command::Judge, &s(&["a.osr", "b.osr"])).is_ok());
    }

    #[test]
    fn the_bot_s_invocations_all_pass_the_gate() {
        assert!(Command::Inspect.accepts("--json"));
        for f in ["--json", "--songs"] {
            assert!(Command::Judge.accepts(f), "judge {f}");
        }
        for f in [
            "--events",
            "--skin",
            "--preset",
            "--crf",
            "--songs",
            "--size",
            "--fps",
            "--mute",
            "--leaderboard",
            "--my-pictures",
            "--encoder-threads",
            "--out",
        ] {
            assert!(Command::Video.accepts(f), "video {f}");
        }
        for f in [
            "--events",
            "--skin",
            "--preset",
            "--crf",
            "--songs",
            "--size",
            "--fps",
            "--for",
            "--clip",
            "--out",
            "--leaderboard",
            "--my-pictures",
            "--encoder-threads",
        ] {
            assert!(Command::Exhibit.accepts(f), "exhibit {f}");
        }
    }

    #[test]
    fn the_table_and_the_gate_agree() {
        const ALL: &[Command] = &[
            Command::Inspect,
            Command::Judge,
            Command::Corpus,
            Command::Debug,
            Command::Sliders,
            Command::Errors,
            Command::Score,
            Command::Health,
            Command::Frame,
            Command::Video,
            Command::Exhibit,
            Command::Sounds,
        ];
        for (flag, _, _) in OPTIONS_TABLE {
            assert!(
                ALL.iter().any(|c| c.accepts(flag)),
                "{flag} is described in help but no command accepts it"
            );
        }
    }

    #[test]
    fn short_flags_fold_onto_their_long_names() {
        assert_eq!(canonical("-s"), "--songs");
        assert_eq!(canonical("--songs"), "--songs");
        assert_eq!(canonical("--crf"), "--crf");
    }
}

#[cfg(test)]
mod skin_choice_tests {
    use super::*;

    #[test]
    fn a_folder_that_exists_is_a_skin() {
        let dir = std::env::temp_dir().join(format!("dossier-choice-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");

        match SkinChoice::parse(dir.to_str().expect("a path")) {
            Ok(SkinChoice::Folder(path)) => assert_eq!(path, dir),
            other => panic!("expected a folder, got {other:?}"),
        }
    }

    #[test]
    fn a_path_that_is_not_there_is_not_silently_a_skin() {
        let miss = SkinChoice::parse("/no/such/folder");
        assert!(miss.is_err(), "{miss:?}");
    }

    #[test]
    fn the_named_skin_still_works_and_the_removed_one_says_so() {
        assert_eq!(SkinChoice::parse("classic"), Ok(SkinChoice::Classic));
        assert_eq!(SkinChoice::parse("map"), Ok(SkinChoice::Classic));
        let gone = SkinChoice::parse("1984").expect_err("removed");
        assert!(gone.contains("removed"), "{gone}");
    }

    #[test]
    fn a_skin_folder_is_where_its_sounds_come_from_too() {
        let dir = std::env::temp_dir().join(format!("dossier-sounds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let choice = SkinChoice::Folder(dir.clone());
        assert_eq!(choice.samples_dir(), Some(dir.as_path()));
        assert_eq!(SkinChoice::Classic.samples_dir(), None);
    }
}

fn assay_command(options: Options) -> Result<(), String> {
    let map = options
        .map
        .as_ref()
        .ok_or("assay needs a map: --map <path to .osu>")?;
    let mods = match options.mods.as_deref() {
        Some(text) => assay::parse_mods(text)?,
        None => dossier_replay::Mods::new(0),
    };

    let asked_about_a_play = options.accuracy.is_some()
        || options.combo.is_some()
        || options.misses.is_some()
        || options.n300.is_some()
        || options.n100.is_some()
        || options.n50.is_some();

    let play = if asked_about_a_play {
        let text = std::fs::read_to_string(map)
            .map_err(|error| format!("could not read {}: {error}", map.display()))?;
        let beatmap = dossier_beatmap::Beatmap::parse(&text)
            .map_err(|error| format!("could not parse the map: {error}"))?;
        let attributes = dossier_assay::attributes(&beatmap, mods);
        Some(assay::score_from(
            &attributes,
            options.accuracy,
            options.combo,
            options.misses.unwrap_or(0),
            options.n300,
            options.n100,
            options.n50,
            options.slider_ends,
            options.large_tick_misses.unwrap_or(0),
            options.classic,
            options.legacy_total,
        ))
    } else {
        None
    };

    print!("{}", assay::run(map, mods, play)?);
    Ok(())
}
