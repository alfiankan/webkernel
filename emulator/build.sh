#!/usr/bin/env bash
# Builds the Game Boy core to WebAssembly and copies it into web/pkg.
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/gbcore.wasm ../web/pkg/gbcore.wasm
echo "Copied gbcore.wasm -> web/pkg/gbcore.wasm"
