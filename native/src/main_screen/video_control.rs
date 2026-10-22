use super::*;

impl Main {
    pub(super) fn watching(&self) -> bool {
        (self.player.is_some() || self.sharing.open.is_some()) && !self.leaving_player && !self.mini_player
    }

    pub(super) fn show_controls(&mut self, shown: bool, now: Instant) {
        let at = self.controls.interpolate(0.0, 1.0, now);
        let span = if shown { CONTROLS_IN } else { CONTROLS_OUT };
        let left = if shown { 1.0 - at } else { at };
        self.controls = Animation::new(!shown)
            .duration(span.mul_f32(left.clamp(0.2, 1.0)))
            .easing(if shown { Easing::EaseOutCubic } else { Easing::EaseInOutCubic })
            .go(shown, now);
    }

    pub(super) fn play_clip(&mut self, link: String, path: PathBuf, media: videos::Probe) {
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        let post = self.news.posts.iter().find(|post| post.videos.iter().any(|video| video.link == link));
        let thumb = post.and_then(|post| post.videos.iter().find(|video| video.link == link)).and_then(|video| video.thumb.clone());
        let clip = Clip {
            path: path.clone(),
            from: post.map_or_else(String::new, |post| if post.name.is_empty() { format!("@{}", post.channel) } else { post.name.clone() }),
            said: post.map_or_else(String::new, |post| post.text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_owned()),
            link,
            thumb,
        };
        let fresh = self.player.is_none() || self.leaving_player || self.mini_player;
        if let Some(old) = self.player.take() {
            old.borrow_mut().close();
        }
        self.leaving_player = false;
        self.mini_player = false;
        let now = Instant::now();
        if fresh {
            self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
        }
        self.stirred = now;
        self.over_controls = false;
        self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
        let manner = player::Manner { level: self.settings.player_level, muted: self.settings.player_muted, rate: self.settings.player_rate };
        self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(&ffmpeg, &path, media, manner))));
        self.open_video = None;
        self.sharing.open = None;
        self.clip = Some(clip);
        self.asking_delete = false;
        self.scrubbing = None;
        self.hint = None;
        self.cinema.go_mut(true, now);
    }

    pub(super) fn finish_closing(&mut self) {
        if let Some(player) = self.player.take() {
            player.borrow_mut().close();
        }
        self.leaving_player = false;
        self.mini_player = false;
        self.open_video = None;
        self.clip = None;
        self.sharing.open = None;
        self.asking_delete = false;
        self.over_controls = false;
        self.pointer = None;
        self.shut_cinema();
    }

    pub(super) fn shut_cinema(&mut self) {
        self.video_request = None;
        let now = Instant::now();
        self.cinema.go_mut(false, now);
        self.widened.go_mut(false, now);
        self.scrubbing = None;
        self.hint = None;
    }

    pub(super) fn handle_video_control(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenVideo(at) => {
                let Some(video) = self.store.videos.get(at) else {
                    return Task::none();
                };
                if self.overlay != Overlay::Videos {
                    let shown = self.update(Message::Show(Overlay::Videos));
                    return shown.chain(self.update(Message::OpenVideo(at)));
                }
                let Some(ffmpeg) = self.ffmpeg.clone() else {
                    return Task::none();
                };
                let path = video.path.clone();
                let length_ms = video.length_ms;
                self.video_serial = self.video_serial.wrapping_add(1);
                let serial = self.video_serial;
                self.video_request = Some(path.clone());
                if let Some(old) = self.player.take() {
                    // Closing processes can wait for the decoder; keep it off the UI thread.
                    if let Ok(old) = std::rc::Rc::try_unwrap(old) { std::thread::spawn(move || drop(old.into_inner())); }
                }
                self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::still(&path, length_ms, 0))));
                self.open_video = Some(at);
                self.clip = None;
                self.sharing.open = None;
                self.leaving_player = false;
                self.mini_player = false;
                self.asking_delete = false;
                self.scrubbing = None;
                self.hint = None;
                self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, Instant::now());
                self.cinema.go_mut(true, Instant::now());
                self.stirred = Instant::now();
                self.over_controls = false;
                self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
                let manner = player::Manner { level: self.settings.player_level, muted: self.settings.player_muted, rate: self.settings.player_rate };
                ui::in_thread(move || {
                    let media = videos::probe(&ffmpeg, &path);
                    let player = media.map(|media| player::Player::open(&ffmpeg, &path, media, manner));
                    Message::VideoReady(serial, path, media, player::Prepared(std::sync::Arc::new(std::sync::Mutex::new(player))))
                })
            }
            Message::VideoReady(serial, path, media, prepared) => {
                let player = prepared.0.lock().ok().and_then(|mut ready| ready.take());
                if serial != self.video_serial || self.video_request.as_ref() != Some(&path) || (self.overlay != Overlay::Videos && !self.mini_player) {
                    if let Some(player) = player { std::thread::spawn(move || drop(player)); }
                    return Task::none();
                }
                self.video_request = None;
                if let Some(media) = media { self.store.refresh_media(&path, media); }
                self.player = player.map(|player| std::rc::Rc::new(std::cell::RefCell::new(player)));
                if self.player.is_none() {
                    self.stage_open.go_mut(false, Instant::now());
                    self.cinema.go_mut(false, Instant::now());
                    self.announce(notices::Mark::Bad, self.words.t("video-open-failed"), path.display().to_string(), String::new(), String::new(), notices::Link::None);
                }
                Task::none()
            }
            Message::ClosePlayer => {
                self.video_request = None;
                if (self.player.is_none() && self.sharing.open.is_none()) || self.leaving_player {
                    return Task::none();
                }
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if !player.paused {
                        player.toggle();
                    }
                }
                let now = Instant::now();
                self.leaving_player = true;
                self.stage_open.go_mut(false, now);
                self.cinema.go_mut(false, now);
                Task::none()
            }
            Message::PlayerMinimize => {
                if self.player.is_none() || self.leaving_player || self.mini_player {
                    return Task::none();
                }
                self.mini_player = true;
                self.asking_delete = false;
                self.over_controls = false;
                self.scrubbing = None;
                self.pointer = None;
                let now = Instant::now();
                self.stage_open.go_mut(false, now);
                self.cinema.go_mut(false, now);
                self.widened.go_mut(false, now);
                Task::none()
            }
            Message::PlayerRestore => {
                if self.player.is_none() || !self.mini_player || self.leaving_player {
                    return Task::none();
                }
                let home = if self.clip.is_some() { Overlay::Community } else { Overlay::Videos };
                let shown = if self.overlay != home { self.update(Message::Show(home)) } else { Task::none() };
                self.mini_player = false;
                let now = Instant::now();
                self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
                self.cinema.go_mut(true, now);
                self.stirred = now;
                shown
            }
            Message::PlayerToggle | Message::SeekBy(_) | Message::SeekTo(_) | Message::Scrubbing(_) | Message::PlayerSpeed(_) if self.video_request.is_some() => Task::none(),
            Message::PlayerToggle => {
                if let Some(player) = &self.player {
                    player.borrow_mut().toggle();
                }
                Task::none()
            }
            Message::SeekTo(fraction) => {
                self.scrubbing = None;
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    let to = (fraction as f64 * player.length_ms as f64) as i64;
                    player.seek(to);
                }
                Task::none()
            }
            Message::Scrubbing(fraction) => {
                self.scrubbing = Some(fraction.clamp(0.0, 1.0));
                Task::none()
            }
            Message::SeekBy(delta) => {
                if let Some(player) = &self.player {
                    player.borrow_mut().seek_by(delta);
                }
                let sign = if delta > 0 { "+" } else { "−" };
                self.say(format!("{sign}{} {}", delta.abs() / 1000, self.words.t("seconds-short")));
                Task::none()
            }
            Message::StepFrames(by) => {
                if let Some(player) = &self.player {
                    player.borrow_mut().step(by);
                }
                self.say(format!("{}{by}", if by > 0 { "+" } else { "" }));
                Task::none()
            }
            Message::PlayerLevel(level) => {
                let level = level.clamp(0.0, 1.0);
                self.settings.player_level = level;
                self.settings.player_muted = false;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    player.set_level(level);
                    player.set_muted(false);
                }
                self.say(format!("{} %", (level * 100.0).round() as i32));
                Task::none()
            }
            Message::PlayerLouder(by) => {
                let level = (self.settings.player_level + by.clamp(-1.0, 1.0) * 0.05).clamp(0.0, 1.0);
                self.update(Message::PlayerLevel(level))
            }
            Message::PlayerMute => {
                let muted = !self.settings.player_muted;
                self.settings.player_muted = muted;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_muted(muted);
                }
                self.say(self.words.t(if muted { "sound-off" } else { "sound-on" }));
                Task::none()
            }
            Message::PlayerRate(by) => {
                let rate = player::next_rate(self.settings.player_rate, by);
                self.settings.player_rate = rate;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_rate(rate);
                }
                self.say(format!("×{}", self.words.rate(rate)));
                Task::none()
            }
            Message::PlayerSpeed(rate) => {
                if (rate - self.settings.player_rate).abs() < 0.001 {
                    return Task::none();
                }
                self.settings.player_rate = rate;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_rate(rate);
                }
                self.say(format!("×{}", self.words.rate(rate)));
                Task::none()
            }
            Message::PlayerStir(at) => {
                let moved = self.pointer.is_none_or(|was| (was.x - at.x).abs() + (was.y - at.y).abs() > 4.0);
                self.pointer = Some(at);
                if !moved {
                    return Task::none();
                }
                let now = Instant::now();
                self.stirred = now;
                if !self.controls.value() {
                    self.show_controls(true, now);
                }
                Task::none()
            }
            Message::ControlsHover(over) => {
                self.over_controls = over;
                if over && !self.controls.value() {
                    self.show_controls(true, Instant::now());
                }
                Task::none()
            }
            Message::PlayerLoop => {
                let over = !self.settings.player_loop;
                self.settings.player_loop = over;
                let _ = self.settings.save();
                self.say(self.words.t(if over { "loop-on" } else { "loop-off" }));
                Task::none()
            }
            Message::PlayerWiden => {
                let now = Instant::now();
                let wide = !self.widened.value();
                self.widened.go_mut(wide, now);
                Task::none()
            }
            Message::PlayerNeighbour(by) => {
                let Some(at) = self.open_video else {
                    return Task::none();
                };
                let next = at as i32 + by;
                if next < 0 || next as usize >= self.store.videos.len() {
                    return Task::none();
                }
                self.update(Message::OpenVideo(next as usize))
            }
            Message::RevealVideo => {
                if let Some(clip) = &self.clip {
                    return reveal_path(clip.path.clone());
                } else if let Some(path) = self.sharing.opened().and_then(|got| got.cached()) {
                    return reveal_path(path);
                } else if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
                    return reveal_path(video.path.clone());
                }
                Task::none()
            }
            Message::AskDeleteOf(at) => {
                if self.store.videos.get(at).is_some() {
                    self.open_video = Some(at);
                    self.asking_delete = true;
                }
                Task::none()
            }
            Message::AskDelete => {
                self.asking_delete = true;
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if !player.paused {
                        player.toggle();
                    }
                }
                Task::none()
            }
            Message::KeepVideo => {
                self.asking_delete = false;
                Task::none()
            }
            Message::DeleteVideo => {
                self.asking_delete = false;
                let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at).cloned()) else {
                    return Task::none();
                };
                if let Some(player) = self.player.take() {
                    player.borrow_mut().close();
                }
                self.open_video = None;
                self.mini_player = false;
                self.shut_cinema();
                if videos::to_bin(&video.path).is_ok() || !video.path.exists() {
                    self.store.forget(&video.path);
                }
                Task::none()
            }
            Message::ClipFetched(link, probed) => {
                self.clips_loading.remove(&link);
                match probed {
                    Ok((path, media)) if self.overlay == Overlay::Community => self.play_clip(link, path, media),
                    Ok((path, _)) if self.ffmpeg.is_none() => {
                        let _ = open::that_detached(&path);
                    }
                    Ok(_) => {}
                    Err(_) => {
                        let _ = open::that_detached(&link);
                    }
                }
                Task::none()
            }
            _ => unreachable!("message routed to the wrong main screen handler"),
        }
    }
}
