#!/usr/bin/env bash
set -uo pipefail
for attempt in 1 2 3; do
  if timeout 300 sudo apt-get update && timeout 600 sudo apt-get install -y "$@"; then
    exit 0
  fi
  echo "apt-get attempt $attempt failed" >&2
  sleep 20
done
exit 1
