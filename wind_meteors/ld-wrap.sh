#!/bin/sh
# rustc passes --stack-first and --allow-undefined; the ABI forbids both
for a in "$@"; do
  shift
  case "$a" in --stack-first|--allow-undefined|--export=__heap_base|--export=__data_end) ;; *) set -- "$@" "$a";; esac
done
exec wasm-ld "$@"
