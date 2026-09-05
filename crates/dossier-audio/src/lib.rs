mod kit;
mod samples;
mod synth;
mod track;

pub use kit::{Kit, Timbre};
pub use samples::{decode_wav, Found, SamplePack, SampleSet};
pub use synth::Voice;
pub use track::Track;

pub const SAMPLE_RATE: u32 = 44_100;
