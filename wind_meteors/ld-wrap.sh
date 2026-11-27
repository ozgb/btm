#!/bin/sh
# rustc passes --stack-first and --allow-undefined; the ABI forbids both.
# ABI 3.1 wants --no-stack-first; a linker whose help doesn't list it already puts the stack after the data.
for a in "$@"; do
  shift
  case "$a" in --stack-first|--allow-undefined|--export=__heap_base|--export=__data_end) ;; *) set -- "$@" "$a";; esac
done
if wasm-ld --help | grep -q -e '--no-stack-first'; then set -- "$@" --no-stack-first; fi
exec wasm-ld "$@"
