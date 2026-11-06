#!/usr/bin/env bash
set -euo pipefail
sha="$1"
deadline=$(( $(date +%s) + 60 * 60 ))
seen_any=0
while [ "$(date +%s)" -lt "$deadline" ]; do
  runs=$(gh run list --workflow ci --commit "$sha" --event push --json status,conclusion --jq '.[] | "\(.status) \(.conclusion)"')
  if [ -n "$runs" ]; then
    seen_any=1
    if echo "$runs" | grep -q "^completed success$"; then
      echo "CI passed on $sha"
      exit 0
    fi
    if ! echo "$runs" | grep -qv "^completed "; then
      echo "CI did not pass on $sha:" >&2
      echo "$runs" >&2
      exit 1
    fi
  elif [ "$seen_any" -eq 0 ] && [ "$(date +%s)" -gt $(( deadline - 55 * 60 )) ]; then
    echo "No CI run for $sha. Push the commit to continuum before tagging it." >&2
    exit 1
  fi
  sleep 30
done
echo "CI on $sha did not finish in an hour" >&2
exit 1
