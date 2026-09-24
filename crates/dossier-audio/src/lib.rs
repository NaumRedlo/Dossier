mod kit;
mod samples;
mod synth;
mod track;

pub use kit::{Kit, Timbre};
pub use samples::{decode_wav, Found, SamplePack, SampleSet};
pub use synth::Voice;
pub use track::Track;

pub const SAMPLE_RATE: u32 = 44_100;

pub fn quiet(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let command = std::process::Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000);
        command
    };
    command
}
