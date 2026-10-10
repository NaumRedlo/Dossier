use dossier_overlay::{read_frame, Mailbox};
use std::time::Instant;

fn main() -> std::io::Result<()> {
    let mailbox = Mailbox::default();
    let mut input = std::io::stdin().lock();
    while let Some(frame) = read_frame(&mut input)? {
        mailbox.accept(frame, Instant::now())?;
        if let Some(view) = mailbox.try_view(Instant::now())? {
            println!("{:?} pid={} {:?}", view.client, view.pid, view.snapshot);
        }
    }
    mailbox.reset()?;
    Ok(())
}
