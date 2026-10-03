#!/usr/bin/env bash
# Builds the web core to web-core/demo/pkg/ (wasm + JS glue).
#
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <same as Cargo.lock>
# Serve web-core/demo/ over http(s) and open it in a WebGPU browser.
set -euo pipefail
cd "$(dirname "$0")"

target_dir="${CARGO_TARGET_DIR:-target}"
cargo build --release --target wasm32-unknown-unknown

want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3; exit }' Cargo.lock)
have=$(wasm-bindgen --version | awk '{ print $2 }')
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen CLI $have does not match the crate ($want):" >&2
  echo "  cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

wasm-bindgen --target web --no-typescript --out-dir demo/pkg \
  "$target_dir/wasm32-unknown-unknown/release/ghost_web_core.wasm"
ls -la demo/pkg
