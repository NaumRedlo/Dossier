#[cfg(windows)]
mod shown {
    use std::time::{Duration, Instant};

    use dossier_overlay::{Display, Frame, Message, Viewport};
    use dossier_witness_overlay::hud::Hud;
    use dossier_witness_overlay::panes::{self, Panes};
    use dossier_witness_overlay::system;

    pub struct Shown {
        hud: Hud,
        panes: Panes,
        keeps: bool,
        pid: Option<u32>,
        window: Option<isize>,
        display: Option<Display>,
        fresh: bool,
        place: Option<(i32, i32, i32, i32)>,
        said: String,
        raised: u64,
        raised_told: Option<Instant>,
        window_told: String,
        monitors_told: Vec<String>,
    }

    impl Shown {
        pub fn new(lang: dossier_hud::Lang, offers: bool, keeps: bool) -> Shown {
            Shown { hud: Hud::new(lang, offers), panes: Panes::new(), keeps, pid: None, window: None, display: None, fresh: true, place: None, said: String::new(), raised: 0, raised_told: None, window_told: String::new(), monitors_told: system::current_monitors() }
        }

        pub fn take(&mut self, frame: &Frame) {
            match &frame.message {
                Message::Hello { pid, .. } => {
                    self.pid = Some(*pid);
                    self.window = None;
                }
                Message::Kept { passed } if self.keeps => {
                    self.hud.kept(*passed);
                    self.fresh = true;
                }
                Message::Snapshot { snapshot } if snapshot.display != self.display => {
                    eprintln!("witness overlay: the game says {}", match snapshot.display {
                        Some(Display::Window) => "it runs in a window",
                        Some(Display::Full) => "it runs full screen with OpenGL, where a graphics driver may keep windows over it from being seen",
                        Some(Display::Exclusive) => "it runs full screen in compatibility mode",
                        None => "nothing about its screen",
                    });
                    self.display = snapshot.display;
                    self.fresh = true;
                }
                _ => {}
            }
            if self.hud.told(frame) {
                self.fresh = true;
            }
        }

        pub fn moving(&self) -> bool {
            self.hud.moving()
        }

        pub fn hide(&mut self) {
            self.panes.hide();
        }

        pub fn frame(&mut self) {
            self.panes.pump();
            if let Some(pid) = self.pid.filter(|_| self.window.is_none_or(|window| panes::area(window).is_none())) {
                self.window = panes::game_window(pid);
            }
            let keys = self.window.map_or_else(Default::default, panes::keys);
            if self.hud.step(keys, Instant::now()) {
                eprintln!("witness overlay: the send key was held for its time; nothing is sent yet");
                self.fresh = true;
            }
            self.fresh |= self.hud.take_dirty();
            let place = self.window.and_then(panes::area).filter(|_| keys.front);
            let said = match (place, self.window) {
                (Some((x, y, wide, high)), Some(window)) => {
                    if self.fresh || self.place != place || self.hud.moving() {
                        let sprites = self.hud.sprites(Viewport { width: wide as u32, height: high as u32, scale: 1.0 });
                        self.panes.show((x, y), &sprites);
                        self.fresh = false;
                    }
                    let raised = self.panes.keep_above(window);
                    if raised > 0 {
                        self.raised += 1;
                        if self.raised_told.is_none_or(|at| at.elapsed() >= Duration::from_secs(30)) {
                            eprintln!("witness overlay: the game's window rose over {raised} plates and they were put back above it ({} times so far)", self.raised);
                            self.raised_told = Some(Instant::now());
                        }
                    }
                    format!("{} plates over the game, its picture is {wide}x{high} at {x}, {y}", self.panes.count())
                }
                _ => {
                    self.panes.hide();
                    self.fresh = true;
                    if self.window.is_some() {
                        "hidden: the game is not in front".to_owned()
                    } else {
                        "hidden: the game has no window yet".to_owned()
                    }
                }
            };
            if place != self.place {
                self.tell_window();
            }
            self.place = place;
            if said != self.said {
                eprintln!("witness overlay: {said}");
                self.said = said;
            }
        }

        fn tell_window(&mut self) {
            let Some(line) = self.window.and_then(system::window) else {
                return;
            };
            if line != self.window_told {
                eprintln!("witness overlay: {line}");
                self.window_told = line;
            }
            let monitors = system::current_monitors();
            if monitors != self.monitors_told {
                for line in &monitors {
                    eprintln!("witness overlay: now {}", line.trim_start_matches("system: "));
                }
                self.monitors_told = monitors;
            }
        }

        pub fn pause(&self) -> Duration {
            Duration::from_millis(if self.moving() { 16 } else { 50 })
        }
    }
}

#[cfg(windows)]
fn listen(frames: std::sync::mpsc::Sender<dossier_overlay::Frame>) {
    use std::io::{BufRead, Read};
    let input = std::io::stdin();
    let mut input = input.lock();
    loop {
        let mut line = Vec::new();
        match (&mut input).take((dossier_overlay::MAX_FRAME_BYTES + 1) as u64).read_until(b'\n', &mut line) {
            Ok(0) => return,
            Ok(_) if line.last() != Some(&b'\n') => {
                eprintln!("witness overlay: a frame came cut short or too long; the input is closed");
                return;
            }
            Ok(_) => match dossier_overlay::Frame::decode(&line) {
                Ok(frame) => {
                    if frames.send(frame).is_err() {
                        return;
                    }
                }
                Err(why) => eprintln!("witness overlay: a frame was set aside: {why}"),
            },
            Err(why) => {
                eprintln!("witness overlay: the input failed: {why}");
                return;
            }
        }
    }
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let english = args.iter().position(|arg| arg == "--lang").and_then(|at| args.get(at + 1)).is_some_and(|said| said == "en");
    let offers = args.iter().any(|arg| arg == "--offer");
    let keeps = offers || args.iter().any(|arg| arg == "--keeps");
    for line in dossier_witness_overlay::system::lines() {
        eprintln!("witness overlay: {line}");
    }
    let (tell, told) = std::sync::mpsc::channel();
    std::thread::spawn(move || listen(tell));
    let mut shown = shown::Shown::new(if english { dossier_hud::Lang::En } else { dossier_hud::Lang::Ru }, offers, keeps);
    loop {
        loop {
            match told.try_recv() {
                Ok(frame) => shown.take(&frame),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    shown.hide();
                    eprintln!("witness overlay: the frames ended");
                    return;
                }
            }
        }
        shown.frame();
        std::thread::sleep(shown.pause());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Witness Overlay draws over a Windows game and runs only on Windows.");
    std::process::exit(1);
}
