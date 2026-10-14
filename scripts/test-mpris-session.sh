#!/bin/bash
# Run only the explicitly opted-in MPRIS integration test on an isolated bus.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
KAZE_MPRIS_TEST_DIR=$(mktemp -d /tmp/kaze-mpris-session.XXXXXX)
trap 'rm -rf -- "$KAZE_MPRIS_TEST_DIR"' EXIT
export XDG_DATA_HOME="$KAZE_MPRIS_TEST_DIR/data"
export XDG_CACHE_HOME="$KAZE_MPRIS_TEST_DIR/cache"
export KAZE_PRIVATE_MPRIS_TEST=1
dbus-run-session -- cargo test --lib --manifest-path src-tauri/Cargo.toml mpris_private_bus -- --ignored --nocapture
