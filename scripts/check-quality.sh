#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
# Qlty's invocation status cannot replace this required independent Rust gate.
CARGO_BUILD_JOBS=1 cargo +1.98.1 clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets
quality_output=$(mktemp -d)
trap 'rm -rf -- "$quality_output"' EXIT
quality_status=0
qlty check --all --no-fix --jobs 1 --no-progress --print-errors "$@" > "$quality_output/stdout" 2> "$quality_output/stderr" || quality_status=$?
cat "$quality_output/stdout"
cat "$quality_output/stderr" >&2
# Qlty 0.662 can call a failed Clippy execution successful. Inspect the actual exit.
python3 - "$quality_output/stderr" <<'PY'
import pathlib, re, sys
output = pathlib.Path(sys.argv[1]).read_text()
for invocation in output.split('# .qlty/out/'):
    if 'pluginName: clippy\n' not in invocation:
        continue
    result = re.search(r"exitCode: ['\"]?(\d+)", invocation)
    if not result or result.group(1) != '0':
        raise SystemExit('Quality gate failed: Qlty Clippy did not execute successfully')
PY
exit "$quality_status"
