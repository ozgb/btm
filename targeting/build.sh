#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo rustc --release --target wasm32-unknown-unknown -- -C linker="$PWD/ld-wrap.sh" -C target-cpu=mvp -C target-feature=+sign-ext,+bulk-memory \
 -C link-arg=--import-memory -C link-arg=--import-table -C link-arg=--global-base=393216 -C link-arg=--table-base=240 \
 -C link-arg=--no-entry -C link-arg=-zstack-size=4096 -C link-arg=--gc-sections
python3 -I /data/tools/wasm_fuel.py target/wasm32-unknown-unknown/release/targeting.wasm -o module.wasm
