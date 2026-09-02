//! Everything between a judged replay and a file somebody can watch.
//!
//! [`locate`] finds the map and the pictures and sounds that came with it,
//! [`hitsounds`] builds the track the play makes, [`video`] drives the encoder
//! and pumps frames at it, [`reel`] cuts several spans together,
//! [`scenery`] puts the map's own artwork, storyboard and video behind the play,, and [`events`]
//! says what is happening in a form a program can read.
//!
//! All five lived in the `dossier` binary until 2026-09-02, which meant nothing
//! but the command line could ever call them. The render client is becoming an
//! application that links the engine in — see `app/` — and a binary crate
//! cannot be linked into anything.

pub mod events;
pub mod hitsounds;
pub mod json;
pub mod locate;
pub mod notes;
pub mod reel;
pub mod render;
pub mod scenery;
pub mod video;
