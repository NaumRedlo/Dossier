pub mod events;
pub mod font;
pub mod halt;
pub mod hitsounds;
pub mod json;
pub mod locate;
pub mod notes;
pub mod reel;
pub mod render;
pub mod scenery;
pub mod skin;
pub mod video;

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
