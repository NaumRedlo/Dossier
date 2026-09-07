use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use dossier_produce::locate;
use dossier_render::{Layout, Scene, Skin};
use dossier_sim::GameState;

pub struct Wanted {
    pub replay: PathBuf,
    pub songs: Option<PathBuf>,
    pub skin: Option<PathBuf>,
    pub fine: crate::draw::Fine,
}

enum Ask {
    Frame {
        ms: f64,
        width: u32,
        height: u32,
        back: Sender<Option<Vec<u8>>>,
    },
    Facts {
        seconds: Option<f64>,
        back: Sender<Result<crate::play::Scene, String>>,
    },
    Shut,
}

const SHOWS_KEPT: usize = 4;

type Open = Vec<(u64, Sender<Ask>)>;

static DESKS: OnceLock<Mutex<Open>> = OnceLock::new();

static NEXT: AtomicU64 = AtomicU64::new(1);

fn desks() -> &'static Mutex<Open> {
    DESKS.get_or_init(|| Mutex::new(Vec::new()))
}

fn desk(id: u64) -> Option<Sender<Ask>> {
    let mut open = desks().lock().ok()?;
    let at = open.iter().position(|(known, _)| *known == id)?;
    let taken = open.remove(at);
    let ask = taken.1.clone();
    open.push(taken);
    Some(ask)
}

pub fn open(wanted: Wanted) -> Result<u64, String> {
    let (ask, asked) = channel::<Ask>();
    let (told, hears) = channel::<Result<(), String>>();

    std::thread::Builder::new()
        .name("dossier-show".to_owned())
        .spawn(move || {
            let loaded = locate::load(&wanted.replay, None, wanted.songs.as_deref());
            let (beatmap, replay, _origin, _text) = match loaded {
                Ok(all) => all,
                Err(why) => {
                    let _ = told.send(Err(why));
                    return;
                }
            };
            let state = GameState::new(&beatmap, &replay);

            let mut skin = Skin::with_combo_colours(beatmap.combo_colours());
            if let Some(folder) = &wanted.skin {
                skin = dossier_produce::skin::from_folder(skin, folder, None);
            }
            skin.cursor_rotate = wanted.fine.cursor_rotate;
            skin.hit_lighting = wanted.fine.hit_lighting;
            skin.snake_in = wanted.fine.snake_in;
            skin.snake_out = wanted.fine.snake;
            skin.cursor_expand = wanted.fine.cursor_expand;
            if let Ok(Some(font)) = dossier_produce::font::find(None) {
                skin = skin.with_font(font);
            }

            let scene = Scene::new(&state, skin).bare().over_video();
            if told.send(Ok(())).is_err() {
                return;
            }
            drop(told);

            while let Ok(ask) = asked.recv() {
                match ask {
                    Ask::Shut => return,
                    Ask::Frame {
                        ms,
                        width,
                        height,
                        back,
                    } => {
                        let layout = Layout::new(width.max(1), height.max(1));
                        let _ = back.send(scene.frame(ms, &layout).encode_png().ok());
                    }
                    Ask::Facts { seconds, back } => {
                        let mut facts = crate::play::read(&beatmap, &replay, &state);
                        if let (Ok(one), Some(seconds)) = (&mut facts, seconds) {
                            crate::play::narrow(one, seconds);
                        }
                        let _ = back.send(facts);
                    }
                }
            }
        })
        .map_err(|why| format!("предпросмотр не запустился: {why}"))?;

    hears
        .recv()
        .map_err(|_| "предпросмотр закрылся, не открывшись".to_owned())??;

    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut open = desks().lock().expect("открытые показы");
    open.push((id, ask));
    while open.len() > SHOWS_KEPT {
        let (_, oldest) = open.remove(0);
        let _ = oldest.send(Ask::Shut);
    }
    Ok(id)
}

pub fn facts(id: u64, seconds: Option<f64>) -> Result<crate::play::Scene, String> {
    let ask = desk(id).ok_or_else(|| "предпросмотр закрылся".to_owned())?;
    let (back, hears) = channel();
    ask.send(Ask::Facts { seconds, back })
        .map_err(|_| "предпросмотр закрылся".to_owned())?;
    hears
        .recv()
        .map_err(|_| "предпросмотр замолчал".to_owned())?
}

pub fn frame(id: u64, ms: f64, width: u32, height: u32) -> Option<Vec<u8>> {
    let ask = desk(id)?;
    let (back, hears) = channel();
    ask.send(Ask::Frame {
        ms,
        width,
        height,
        back,
    })
    .ok()?;
    hears.recv().ok().flatten()
}

pub fn shut(id: u64) {
    let mut open = desks().lock().expect("открытые показы");
    let Some(at) = open.iter().position(|(known, _)| *known == id) else {
        return;
    };
    let (_, ask) = open.remove(at);
    let _ = ask.send(Ask::Shut);
}

pub fn asked_for(uri: &str) -> Option<(u64, f64, u32, u32)> {
    let query = uri.split_once('?')?.1;
    let mut id = None;
    let mut ms = None;
    let mut width = None;
    let mut height = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "show" => id = value.parse().ok(),
            "ms" => ms = value.parse().ok(),
            "w" => width = value.parse().ok(),
            "h" => height = value.parse().ok(),
            _ => {}
        }
    }
    Some((id?, ms?, width?, height?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_address_carries_the_show_the_instant_and_the_size() {
        let said = asked_for("frame://localhost/?show=7&ms=1234.5&w=480&h=360");
        assert_eq!(said, Some((7, 1234.5, 480, 360)));
    }

    #[test]
    fn an_address_missing_a_piece_is_refused_rather_than_guessed() {
        assert!(asked_for("frame://localhost/?show=7&ms=1234.5&w=480").is_none());
        assert!(asked_for("frame://localhost/").is_none());
        assert!(asked_for("frame://localhost/?show=nine&ms=1&w=1&h=1").is_none());
    }

    #[test]
    fn a_show_that_was_never_opened_draws_nothing_instead_of_waiting() {
        assert!(frame(u64::MAX, 0.0, 320, 240).is_none());
        shut(u64::MAX);
    }
}

#[cfg(test)]
mod against_real_files {
    use super::*;

    fn corpus() -> Option<(PathBuf, PathBuf)> {
        let replays = PathBuf::from("/Users/none/Documents/Dossier Corpus");
        let songs = replays.join("Beatmap");
        (replays.is_dir() && songs.is_dir()).then_some((replays, songs))
    }

    #[test]
    #[ignore]
    fn a_show_opens_once_and_answers_frames_and_facts() {
        let Some((replays, songs)) = corpus() else {
            return;
        };
        let mut opened = None;
        for entry in std::fs::read_dir(&replays).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "osr") {
                continue;
            }
            let wanted = Wanted {
                replay: path.clone(),
                songs: Some(songs.clone()),
                skin: Some(PathBuf::from("/Users/none/Documents/Skins/vv_idke_trail")),
                fine: crate::draw::Fine::default(),
            };
            if let Ok(id) = open(wanted) {
                opened = Some((id, path));
                break;
            }
        }
        let (id, path) = opened.expect("a replay whose map is here");
        println!("opened {} as show {id}", path.display());

        let facts = facts(id, Some(6.0)).expect("the facts");
        assert!(!facts.starts.is_empty(), "a map with nothing in it");
        assert!(facts.to_ms > facts.from_ms);
        assert!(
            facts.to_ms - facts.from_ms <= 7300.0,
            "the window is six seconds plus the run-up"
        );

        let middle = (facts.from_ms + facts.to_ms) / 2.0;
        let png = frame(id, middle, 480, 360).expect("a frame");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "not a png");
        let wide = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let high = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!((wide, high), (480, 360));
        println!("frame at {middle}ms: {} bytes", png.len());

        let other = frame(id, middle + 400.0, 320, 240).expect("another frame");
        assert_ne!(png, other, "two instants drew the same picture");

        shut(id);
        assert!(
            frame(id, middle, 480, 360).is_none(),
            "a shut show still answers"
        );
    }

    #[test]
    #[ignore]
    fn only_the_last_few_shows_stay_open() {
        let Some((replays, songs)) = corpus() else {
            return;
        };
        let mut found = None;
        for entry in std::fs::read_dir(&replays).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "osr") {
                continue;
            }
            let wanted = Wanted {
                replay: path.clone(),
                songs: Some(songs.clone()),
                skin: None,
                fine: crate::draw::Fine::default(),
            };
            if let Ok(id) = open(wanted) {
                found = Some((id, path));
                break;
            }
        }
        let (first, path) = found.expect("a replay whose map is here");

        let mut ids = vec![first];
        for _ in 0..SHOWS_KEPT {
            ids.push(
                open(Wanted {
                    replay: path.clone(),
                    songs: Some(songs.clone()),
                    skin: None,
                    fine: crate::draw::Fine::default(),
                })
                .expect("another show of the same replay"),
            );
        }
        assert!(
            frame(ids[0], 0.0, 64, 48).is_none(),
            "the oldest show was kept past the limit"
        );
        assert!(
            frame(*ids.last().unwrap(), 0.0, 64, 48).is_some(),
            "the newest show was dropped"
        );
        for id in ids {
            shut(id);
        }
    }
}
