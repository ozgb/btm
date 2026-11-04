"""Fuel metering pass, rule v1 (ABI 11.1), over wasm-tools text. Usage: fuel.py in.wasm out.wasm"""
import re, subprocess, sys

src, dst = sys.argv[1], sys.argv[2]
text = subprocess.run(["wasm-tools", "print", src], check=True, capture_output=True, text=True).stdout
lines = text.split("\n")
ng = sum(1 for l in lines if re.match(r"\s*\(global ", l))
ni = sum(1 for l in lines if re.match(r"\s*\(import .*\(global", l))
assert ni == 0
STEP = ["global.get $__fuel", "i64.const 1", "i64.sub", "global.set $__fuel",
        "global.get $__fuel", "i64.const 0", "i64.lt_s", "if", "unreachable", "end"]
out = []
i = 0
last_close = max(k for k, l in enumerate(lines) if l.strip())
while i < len(lines):
    l = lines[i]
    out.append(l)
    if re.match(r"\s*\(func ", l) and "(import" not in l:
        i += 1
        while re.match(r"\s*\((local|param|result)", lines[i]):
            out.append(lines[i]); i += 1
        out.extend("    " + s for s in STEP)
        continue
    if re.match(r"\s*loop\b", l):
        out.extend("    " + s for s in STEP)
    i += 1
# append module-level items before the final ")"
idx = max(k for k, l in enumerate(out) if l.strip() == ")")
extra = [
    "  (global $__fuel (mut i64) i64.const 9223372036854775807)",
    "  (func $__fuel_set (param i64) local.get 0 global.set $__fuel)",
    "  (func $__fuel_get (result i64) global.get $__fuel)",
    '  (export "__fuel_set" (func $__fuel_set))',
    '  (export "__fuel_get" (func $__fuel_get))',
]
out[idx:idx] = extra
open("/tmp/fuel.wat", "w").write("\n".join(out))
subprocess.run(["wasm-tools", "parse", "/tmp/fuel.wat", "-o", dst], check=True)
