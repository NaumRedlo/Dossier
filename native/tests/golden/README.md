# Golden frames

Every interface test uses `wgpu` on a hardware GPU. `tiny-skia` is not part of
the app's build; CPU adapters (software Vulkan included) are refused. Without a
GPU the test fails instead of switching renderers.

```sh
DOSSIER_GOLDEN_REVIEW=/private/tmp/dossier-review cargo test --manifest-path native/Cargo.toml --test golden -- --test-threads=1
```

The current frames are written separately. A missing, damaged or different
golden is an error; a test never creates or updates goldens. Without
`DOSSIER_GOLDEN_REVIEW` a temporary folder named after the process id is used.

To look at one state:

```sh
cargo run --manifest-path native/Cargo.toml --example ui_preview -- main-prefs /private/tmp/dossier-gpu
```

The example takes `DOSSIER_PREVIEW_SIZE=980x720` and `DOSSIER_PREVIEW_LANG=ru-RU`
(or `en-US`), and after the path `scroll 500`, `drag 220 105 100 400`
or `scroll-drag 1200 250 240 70 500`.
For the matrix of sizes and languages: `bash tools/preview-design.sh /private/tmp/dossier-design`.

Before replacing goldens, look at the new frames and check that the changes
are the ones the task asked for. Then move the reviewed PNGs into this folder
by hand, keeping the `-wgpu` suffix. Never rename old CPU frames into GPU
goldens. Export finishes the PNG on the side and only then replaces the target.

In CI the ordinary Windows, Linux and macOS runners compile every test. The full
native run is a separate job for a runner labelled `self-hosted` and
`dossier-gpu`: it needs a hardware GPU, its driver and Dossier's build
dependencies (`libasound2-dev` on Linux as well). No such runner is registered
yet, so since 7 November 2026 the job runs only when started by hand through
`workflow_dispatch`; pull request code never runs on that machine. Until a
runner is connected the GPU checks are run locally, along the same path.
