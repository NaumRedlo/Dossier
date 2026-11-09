#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
from="${1:-../1984/tests/contract/wire}"
cp "$from"/duel_*.json native/tests/fixtures/wire/
ls native/tests/fixtures/wire
