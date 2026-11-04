"""Apply fuel rule v1 of the module ABI to a linked module.

usage: meter.py IN.wasm OUT.wasm

A mutable i64 global, defined by the module and not exported, starts at the largest i64. The
counter drops by 1 at the start of every defined function and of every loop body, and the module
traps when it goes below zero. The module exports __fuel_set(i64) and __fuel_get() -> i64.
Needs wasm-tools (on PATH, or named by WASM_TOOLS).
"""
import os
import re
import subprocess
import sys

TOOL = os.environ.get("WASM_TOOLS", "wasm-tools")
METER = [
    "global.get $__fuel",
    "i64.const 1",
    "i64.sub",
    "global.set $__fuel",
    "global.get $__fuel",
    "i64.const 0",
    "i64.lt_s",
    "if",
    "unreachable",
    "end",
]


def balance(line):
    line = re.sub(r"\(;.*?;\)", "", line)
    line = line.split(";;")[0]
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    return line.count("(") - line.count(")")


def meter(text):
    out = []
    lines = text.split("\n")
    last_global = max(i for i, l in enumerate(lines) if l.startswith("  (global "))
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("  (func "):
            if balance(line) == 0:
                out.append(line[:-1])
                out.extend("    " + m for m in METER)
                out.append("  )")
                i += 1
                continue
            out.append(line)
            i += 1
            while lines[i].startswith("    (local "):
                out.append(lines[i])
                i += 1
            out.extend("    " + m for m in METER)
            continue
        out.append(line)
        if re.match(r"\s+loop( |$)", line):
            indent = re.match(r"\s*", line).group(0)
            out.extend(indent + "  " + m for m in METER)
        if i == last_global:
            out.append("  (global $__fuel (mut i64) i64.const 9223372036854775807)")
        i += 1
    while out[-1] == "":
        out.pop()
    assert out[-1] == ")"
    out[-1:] = [
        '  (func $__fuel_set (param i64) local.get 0 global.set $__fuel)',
        '  (func $__fuel_get (result i64) global.get $__fuel)',
        '  (export "__fuel_set" (func $__fuel_set))',
        '  (export "__fuel_get" (func $__fuel_get))',
        ")",
    ]
    return "\n".join(out) + "\n"


def main():
    src, dst = sys.argv[1:]
    text = subprocess.run([TOOL, "print", src], check=True, capture_output=True, text=True).stdout
    if "$__fuel" in text:
        sys.exit("meter.py: module is already metered")
    subprocess.run([TOOL, "parse", "-o", dst], input=meter(text), check=True, text=True)
    subprocess.run([TOOL, "validate", dst], check=True)


main()
