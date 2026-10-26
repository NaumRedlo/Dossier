#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-/private/tmp/dossier-design}"
frames="${DOSSIER_DESIGN_FRAMES:-main-rest main-pools-editor main-community-balance-catalog main-community-balance-catalog-add}"
sizes="${DOSSIER_DESIGN_SIZES:-980x720 1280x800 1600x1000}"
languages="${DOSSIER_DESIGN_LANGUAGES:-ru-RU en-US}"
for lang in $languages; do
    for size in $sizes; do
        for frame in $frames; do
            DOSSIER_PREVIEW_LANG="$lang" DOSSIER_PREVIEW_SIZE="$size" \
                cargo run --manifest-path native/Cargo.toml --example ui_preview -- "$frame" "$out"
        done
    done
done
