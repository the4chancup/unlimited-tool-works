#!/bin/bash
# Removes the VPS clone's build cache, ~/studio-mutants/tree/target, when `just gates-remote`'s
# whole gates end, or by hand (RUNBOOK "The scratch folder"): only the gates and `just remote`
# build there, while mutation runs build in their own copy of the tree under
# ~/studio-mutants/tmp. Nothing else is touched.
# Runs on the host: `ssh <host> "bash -s" < scripts/mutation/remote_clean_target.sh` for a dry run,
# `ssh <host> "bash -s -- --delete" < scripts/mutation/remote_clean_target.sh` to remove. Refuses
# while a remote half, the gates or any cargo process runs.
set -u
TARGET="$HOME/studio-mutants/tree/target"
[ -d "$TARGET" ] || { echo "no $TARGET"; exit 0; }
[ -f "$TARGET/CACHEDIR.TAG" ] || { echo "$TARGET has no CACHEDIR.TAG; not a cargo target"; exit 1; }
for unit in studio-mutants studio-gates; do
  if systemctl is-active --quiet "$unit"; then
    echo "$unit is running; nothing done"; exit 1
  fi
done
if pgrep -u "$(id -u)" -x cargo >/dev/null || pgrep -u "$(id -u)" -x rustc >/dev/null; then
  echo "a cargo or rustc process runs; nothing done"; exit 1
fi
du -sh "$TARGET"
if [ "${1:-}" = "--delete" ]; then
  rm -rf -- "$TARGET"
  echo "removed $TARGET"
else
  echo "would remove $TARGET"
fi
df -h "$HOME" | tail -1
