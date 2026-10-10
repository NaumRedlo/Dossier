use std::io::{self, BufRead};
use std::sync::{Arc, Mutex, TryLockError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub mod live;

pub const VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 16 * 1024;
pub const STALE_AFTER: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Client {
    Stable,
    Lazer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    Menu,
    Selection,
    Playing,
    Results,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beatmap {
    pub md5: String,
    pub title: String,
    pub artist: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gameplay {
    pub ruleset: u8,
    pub time_ms: i64,
    pub score: u64,
    pub combo: u32,
    pub max_combo: u32,
    pub accuracy: Option<f64>,
    pub misses: u32,
    pub legacy_mods: Option<u32>,
    #[serde(default)]
    pub mods: Vec<Mod>,
    #[serde(default)]
    pub unstable_rate: Option<f64>,
    #[serde(default)]
    pub resting: Option<bool>,
    #[serde(default)]
    pub pp: Option<f64>,
    #[serde(default)]
    pub pp_clean: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mod {
    pub acronym: String,
    #[serde(default)]
    pub settings: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Meter {
    pub shown: bool,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub screen: Screen,
    pub beatmap: Option<Beatmap>,
    pub gameplay: Option<Gameplay>,
    pub watching_replay: Option<bool>,
    #[serde(default)]
    pub meter: Option<Meter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Hello {
        client: Client,
        pid: u32,
        build: String,
    },
    Snapshot {
        snapshot: Snapshot,
    },
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub event: String,
    pub version: u16,
    pub session: u64,
    pub sequence: u64,
    #[serde(flatten)]
    pub message: Message,
}

impl Frame {
    pub fn new(session: u64, sequence: u64, message: Message) -> Self {
        Self {
            event: "overlay".into(),
            version: VERSION,
            session,
            sequence,
            message,
        }
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.event != "overlay" || self.version != VERSION || self.session == 0 {
            return Err(invalid("unsupported overlay envelope"));
        }
        if let Message::Snapshot { snapshot } = &self.message {
            if snapshot.gameplay.is_some() && snapshot.screen != Screen::Playing {
                return Err(invalid("gameplay outside the playing screen"));
            }
            if let Some(game) = &snapshot.gameplay {
                if game.ruleset > 3
                    || game.combo > game.max_combo
                    || game
                        .accuracy
                        .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
                    || [game.unstable_rate, game.pp, game.pp_clean]
                        .into_iter()
                        .flatten()
                        .any(|value| !value.is_finite() || value < 0.0)
                {
                    return Err(invalid("invalid gameplay values"));
                }
            }
            if snapshot.meter.is_some_and(|meter| {
                !meter.scale.is_finite() || !(0.1..=10.0).contains(&meter.scale)
            }) {
                return Err(invalid("invalid meter scale"));
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> io::Result<String> {
        self.validate()?;
        let mut line = serde_json::to_string(self).map_err(invalid)?;
        if line.len() >= MAX_FRAME_BYTES {
            return Err(invalid("overlay frame too large"));
        }
        line.push('\n');
        Ok(line)
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(invalid("overlay frame too large"));
        }
        let frame: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        frame.validate()?;
        Ok(frame)
    }
}

fn invalid(why: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Frame>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    reader
        .take((MAX_FRAME_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.last() != Some(&b'\n') {
        return Err(invalid(
            "truncated or oversized overlay frame; close transport",
        ));
    }
    Frame::decode(&bytes).map(Some)
}

#[derive(Debug, Clone)]
pub struct View {
    pub client: Client,
    pub pid: u32,
    pub snapshot: Snapshot,
}

#[derive(Default)]
pub struct Receiver {
    connection: Option<(u64, u64, Client, u32)>,
    latest: Option<(View, Instant)>,
    closed: bool,
}

impl Receiver {
    pub fn accept(&mut self, frame: Frame, now: Instant) -> io::Result<()> {
        frame.validate()?;
        if let Message::Hello { client, pid, .. } = frame.message {
            if self
                .connection
                .is_some_and(|(session, ..)| frame.session <= session)
            {
                return Err(invalid("repeated or old overlay handshake"));
            }
            self.connection = Some((frame.session, frame.sequence, client, pid));
            self.latest = None;
            self.closed = false;
            return Ok(());
        }
        let Some((session, sequence, client, pid)) = self.connection else {
            return Err(invalid("overlay handshake required"));
        };
        if self.closed || frame.session != session || frame.sequence <= sequence {
            return Err(invalid("old or foreign overlay frame"));
        }
        self.connection = Some((session, frame.sequence, client, pid));
        match frame.message {
            Message::Snapshot { snapshot } => {
                self.latest = Some((
                    View {
                        client,
                        pid,
                        snapshot,
                    },
                    now,
                ))
            }
            Message::Disconnected => {
                self.latest = None;
                self.closed = true;
            }
            Message::Hello { .. } => unreachable!(),
        }
        Ok(())
    }

    pub fn visible(&self, now: Instant) -> Option<&View> {
        self.latest
            .as_ref()
            .filter(|(_, at)| now.saturating_duration_since(*at) < STALE_AFTER)
            .map(|(view, _)| view)
    }
}

#[derive(Clone, Default)]
pub struct Mailbox(Arc<Mutex<Receiver>>);

impl Mailbox {
    pub fn accept(&self, frame: Frame, now: Instant) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|_| invalid("overlay receiver unavailable"))?
            .accept(frame, now)
    }

    pub fn reset(&self) -> io::Result<()> {
        *self
            .0
            .lock()
            .map_err(|_| invalid("overlay receiver unavailable"))? = Receiver::default();
        Ok(())
    }

    pub fn try_view(&self, now: Instant) -> io::Result<Option<View>> {
        match self.0.try_lock() {
            Ok(receiver) => Ok(receiver.visible(now).cloned()),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Poisoned(_)) => Err(invalid("overlay receiver unavailable")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

pub trait Host {
    type Error;
    fn draw(&mut self, view: Option<&View>, viewport: Viewport) -> Result<(), Self::Error>;
    fn release(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(session: u64, client: Client) -> Frame {
        Frame::new(
            session,
            1,
            Message::Hello {
                client,
                pid: 42,
                build: "test".into(),
            },
        )
    }

    fn snapshot(session: u64, sequence: u64) -> Frame {
        Frame::new(
            session,
            sequence,
            Message::Snapshot {
                snapshot: Snapshot {
                    screen: Screen::Playing,
                    beatmap: None,
                    watching_replay: Some(false),
                    meter: Some(Meter {
                        shown: true,
                        scale: 1.5,
                    }),
                    gameplay: Some(Gameplay {
                        ruleset: 0,
                        time_ms: 1500,
                        score: 1234,
                        combo: 3,
                        max_combo: 4,
                        accuracy: Some(98.5),
                        misses: 1,
                        legacy_mods: None,
                        mods: vec![Mod {
                            acronym: "DT".into(),
                            settings: [("speed_change".into(), serde_json::json!(1.2))].into(),
                        }],
                        unstable_rate: Some(84.2),
                        resting: Some(false),
                        pp: Some(312.4),
                        pp_clean: Some(421.0),
                    }),
                },
            },
        )
    }

    #[test]
    fn stable_and_lazer_share_the_wire_contract_without_losing_mod_settings() {
        for client in [Client::Stable, Client::Lazer] {
            let mut receiver = Receiver::default();
            let now = Instant::now();
            receiver.accept(hello(1, client), now).unwrap();
            let original = snapshot(1, 2);
            let encoded = original.encode().unwrap();
            let decoded = read_frame(&mut encoded.as_bytes()).unwrap().unwrap();
            assert_eq!(decoded, original);
            receiver.accept(decoded, now).unwrap();
            assert_eq!(receiver.visible(now).unwrap().client, client);
            assert_eq!(
                receiver
                    .visible(now)
                    .unwrap()
                    .snapshot
                    .gameplay
                    .as_ref()
                    .unwrap()
                    .mods[0]
                    .settings["speed_change"],
                1.2
            );
        }
    }

    #[test]
    fn missing_handshake_wrong_version_and_foreign_or_repeated_frames_are_rejected() {
        let now = Instant::now();
        let mut receiver = Receiver::default();
        assert!(receiver.accept(snapshot(1, 2), now).is_err());
        let mut incompatible = hello(1, Client::Stable);
        incompatible.version += 1;
        assert!(receiver.accept(incompatible, now).is_err());
        receiver.accept(hello(1, Client::Stable), now).unwrap();
        receiver.accept(snapshot(1, 2), now).unwrap();
        assert!(receiver.accept(snapshot(1, 2), now).is_err());
        assert!(receiver.accept(snapshot(2, 3), now).is_err());
        assert!(receiver.accept(hello(1, Client::Stable), now).is_err());
    }

    #[test]
    fn stale_disconnected_and_reconnected_streams_never_show_old_gameplay() {
        let now = Instant::now();
        let mut receiver = Receiver::default();
        receiver.accept(hello(1, Client::Stable), now).unwrap();
        receiver.accept(snapshot(1, 2), now).unwrap();
        assert!(receiver.visible(now + STALE_AFTER).is_none());
        receiver
            .accept(Frame::new(1, 3, Message::Disconnected), now)
            .unwrap();
        assert!(receiver.visible(now).is_none());
        assert!(receiver.accept(snapshot(1, 4), now).is_err());
        receiver.accept(hello(2, Client::Lazer), now).unwrap();
        assert!(receiver.visible(now).is_none());
        receiver.accept(snapshot(2, 2), now).unwrap();
        assert_eq!(receiver.visible(now).unwrap().client, Client::Lazer);
    }

    #[test]
    fn latest_snapshot_replaces_backlog_and_render_thread_never_waits_on_reader() {
        let mailbox = Mailbox::default();
        let now = Instant::now();
        mailbox.accept(hello(1, Client::Stable), now).unwrap();
        for sequence in 2..1000 {
            let mut frame = snapshot(1, sequence);
            if let Message::Snapshot { snapshot } = &mut frame.message {
                snapshot.gameplay.as_mut().unwrap().score = sequence;
            }
            mailbox.accept(frame, now).unwrap();
        }
        assert_eq!(
            mailbox
                .try_view(now)
                .unwrap()
                .unwrap()
                .snapshot
                .gameplay
                .unwrap()
                .score,
            999
        );
        let lock = mailbox.0.lock().unwrap();
        assert!(mailbox.try_view(now).unwrap().is_none());
        drop(lock);
        mailbox.reset().unwrap();
        assert!(mailbox.try_view(now).unwrap().is_none());
        mailbox.accept(hello(1, Client::Stable), now).unwrap();
    }

    #[test]
    fn bounded_reader_rejects_oversized_truncated_and_invalid_data() {
        assert!(read_frame(&mut &b""[..]).unwrap().is_none());
        let encoded = hello(1, Client::Stable).encode().unwrap();
        assert!(read_frame(&mut encoded.trim_end().as_bytes()).is_err());
        let oversized = vec![b'x'; MAX_FRAME_BYTES * 4];
        let mut reader = oversized.as_slice();
        assert!(read_frame(&mut reader).is_err());
        assert_eq!(reader.len(), oversized.len() - MAX_FRAME_BYTES - 1);
        assert!(Frame::decode(b"not json").is_err());
        let mut frame = snapshot(1, 2);
        if let Message::Snapshot { snapshot } = &mut frame.message {
            snapshot.gameplay.as_mut().unwrap().accuracy = Some(f64::NAN);
        }
        assert!(frame.encode().is_err());
    }
}
