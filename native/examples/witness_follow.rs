use std::sync::Arc;

fn main() {
    let seconds: u64 = std::env::args().nth(1).and_then(|said| said.parse().ok()).unwrap_or(20);
    let control = Arc::new(dossier_native::witness::Control::default());
    let stopper = control.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        stopper.stop();
    });
    println!("launch: {:?}", dossier_native::witness::launch());
    println!("program: {:?}", dossier_native::witness::program());
    let folder = std::env::var_os("WITNESS_KEEP").map(std::path::PathBuf::from);
    dossier_native::witness::run(control, "NaumRedlo".to_owned(), &mut |event| {
        match &event {
            dossier_native::witness::Event::Kept(kept) => {
                let written = folder.as_ref().map(|folder| dossier_native::witness::keep(kept, folder));
                println!("kept {} ({} frames, {} points, passed {}): {written:?}", kept.name, kept.frames, kept.score, kept.passed);
            }
            other => println!("{other:?}"),
        }
        true
    });
}
