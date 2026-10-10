use anyhow::{Context, bail};
use asdf_overlay_client::{
    OverlayDll,
    common::{
        event::{
            OverlayEvent,
            surface::{GpuLuid, SurfaceEvent},
        },
        request::surface::{SetPosition, UpdateSharedHandle},
    },
    inject,
};
use asdf_overlay_surface_util::surface::OverlaySurface;
use dossier_overlay::{Mailbox, read_frame};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1};

struct Surface {
    texture: OverlaySurface,
    width: u32,
    height: u32,
    scene: dossier_overlay_host::Scene,
}

fn adapter(gpu: GpuLuid) -> anyhow::Result<IDXGIAdapter> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    for index in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters(index) }) else {
            break;
        };
        let desc = unsafe { adapter.GetDesc()? };
        if desc.AdapterLuid.LowPart == gpu.low && desc.AdapterLuid.HighPart == gpu.high {
            return Ok(adapter);
        }
    }
    bail!("The game's GPU adapter is unavailable")
}

pub async fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 && args.len() != 4 {
        bail!(
            "Usage: dossier-overlay-host <stable-pid> <x86-dll> <x64-dll> [context-json]; pipe Witness --overlay JSON into stdin"
        )
    }
    let context_path = args.get(3).map(PathBuf::from);
    let mut context_packet = None;
    let mut context_read_at = None;
    let pid: u32 = args[0].to_str().context("invalid PID")?.parse()?;
    anyhow::ensure!(pid != 0, "PID must be nonzero");
    let x86 = PathBuf::from(&args[1]).canonicalize()?;
    let x64 = PathBuf::from(&args[2]).canonicalize()?;
    let mailbox = Arc::new(Mailbox::default());
    let ended = Arc::new(AtomicBool::new(false));
    std::thread::spawn({
        let mailbox = mailbox.clone();
        let ended = ended.clone();
        move || {
            let input = std::io::stdin();
            let mut input = input.lock();
            loop {
                match read_frame(&mut input) {
                    Ok(Some(frame)) => {
                        if let Err(error) = mailbox.accept(frame, Instant::now()) {
                            eprintln!("Rejected overlay frame: {error}");
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!("Overlay input failed: {error}");
                        break;
                    }
                }
            }
            let _ = mailbox.reset();
            ended.store(true, Ordering::Release);
        }
    });
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            anyhow::ensure!(
                !ended.load(Ordering::Acquire),
                "Witness input ended before a snapshot"
            );
            if let Some(view) = mailbox.try_view(Instant::now())? {
                anyhow::ensure!(
                    view.pid == pid && view.client == dossier_overlay::Client::Stable,
                    "Witness must be connected to the selected stable process"
                );
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("Witness did not supply telemetry within 15 seconds")??;
    let (mut conn, mut events) = inject(
        pid,
        OverlayDll {
            x86: Some(&x86),
            x64: Some(&x64),
            ..Default::default()
        },
        Some(Duration::from_secs(10)),
    )
    .await?;
    let mut surfaces: HashMap<u64, Surface> = HashMap::new();
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let result: anyhow::Result<()> = async {
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                event = events.recv() => match event {
                    Some(OverlayEvent::Surface { id, event }) => match event {
                        SurfaceEvent::Added { width, height, info } => {
                            let adapter = adapter(info.gpu_id)?;
                            let texture = OverlaySurface::new(Some(&adapter), info.keyed_mutex)?;
                            surfaces.insert(id, Surface { texture, width, height, scene: dossier_overlay_host::Scene::default() });
                            eprintln!("Overlay surface {id}: {width}x{height}, {:?}", info.api);
                        },
                        SurfaceEvent::Resized { width, height } => if let Some(surface) = surfaces.get_mut(&id) {
                            surface.width = width;
                            surface.height = height;
                        },
                        SurfaceEvent::Destroyed => { surfaces.remove(&id); },
                    },
                    None => break,
                    _ => {},
                },
                _ = tick.tick() => {
                    if ended.load(Ordering::Acquire) { break; }
                    let now = Instant::now();
                    let view = mailbox.try_view(now)?;
                    if context_read_at.is_none_or(|at| now.duration_since(at) >= Duration::from_secs(1)) {
                        context_packet = context_path.as_ref().and_then(|path| dossier_hud::context::read(path).ok());
                        context_read_at = Some(now);
                    }
                    for (&id, surface) in &mut surfaces {
                        surface.scene.supply(context_packet.clone());
                        match surface.scene.update(view.as_ref(), pid, surface.width, surface.height, now) {
                            dossier_overlay_host::Update::Draw(sprite) => {
                                tokio::time::timeout(Duration::from_secs(2), conn.surface(id).request(SetPosition { x: sprite.x, y: sprite.y })).await??;
                                if let Some(handle) = surface.texture.update_bitmap(sprite.width, &sprite.pixels)? {
                                    tokio::time::timeout(Duration::from_secs(2), conn.surface(id).request(handle)).await??;
                                }
                            }
                            dossier_overlay_host::Update::Hide => {
                                tokio::time::timeout(Duration::from_secs(2), conn.surface(id).request(UpdateSharedHandle::None)).await??;
                                surface.texture.clear();
                            }
                            dossier_overlay_host::Update::Unchanged => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }.await;
    for &id in surfaces.keys() {
        let _ = tokio::time::timeout(
            Duration::from_secs(1),
            conn.surface(id).request(UpdateSharedHandle::None),
        )
        .await;
    }
    result
}
