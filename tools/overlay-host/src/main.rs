#[cfg(windows)]
mod windows;

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(windows::run())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The experimental overlay host requires Windows.");
    std::process::exit(1);
}
