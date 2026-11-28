#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo rustc --release --target wasm32-unknown-unknown -- -C linker="$PWD/ld-wrap.sh" -C target-cpu=mvp -C target-feature=+sign-ext,+bulk-memory \
 -C link-arg=--import-memory -C link-arg=--import-table -C link-arg=--global-base=655360 -C link-arg=--table-base=496 \
 -C link-arg=--no-entry -C link-arg=-zstack-size=4096 -C link-arg=--gc-sections
FUEL_TOOL=${FUEL_TOOL:-/data/tools/wasm_fuel.py}
python3 -I "$FUEL_TOOL" target/wasm32-unknown-unknown/release/mission.wasm -o module.wasm
python3 -I "$FUEL_TOOL" --check module.wasm
wasm-tools print module.wasm | grep -q '(import "env" "__indirect_function_table"' || { echo "table import missing" >&2; exit 1; }
