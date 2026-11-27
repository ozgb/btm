"""Compares the core's `host` replay with the lines a full recording of tests/replay/wind_meteors/ holds.

    python3 -I tools/host_check.py CORE MODULE.wasm
"""
import json, os, shutil, subprocess, sys, tempfile

KIT = "/data"
core, module = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])


def expected(path):
    calls = []
    for line in open(path).read().split("\n"):
        if line.startswith("call "):
            calls.append([])
        elif line.startswith("  w ") or line.startswith("  o params."):
            calls[-1].append(line[2:])
    return calls


bad = 0
for name in ["console-features", "meteors-earthquake", "meteors-hailstorm"]:
    with tempfile.TemporaryDirectory() as d:
        os.makedirs(os.path.join(d, "host"))
        shutil.copy(os.path.join(KIT, "reference/core/host", name + ".txt"), os.path.join(d, "host"))
        shutil.copy(module, os.path.join(d, "module.wasm"))
        open(os.path.join(d, "CALLS"), "w").write(f"replay\thost/{name}.txt\tmodule.wasm\n")
        r = subprocess.run([core, "host", "CALLS", "OUT"], cwd=d, capture_output=True)
        out = open(os.path.join(d, "OUT")).read().strip()
        got = json.loads(out.split("\t", 1)[1])
    want = expected(os.path.join(KIT, "tests/replay/wind_meteors", name + ".txt"))
    if isinstance(got, dict):
        print(name, "error", got); bad += 1; continue
    first = next((i for i in range(max(len(got), len(want))) if i >= len(got) or i >= len(want) or got[i] != want[i]), None)
    if first is None:
        print(name, "equal", len(got), "calls")
    else:
        bad += 1
        print(name, "differs at call", first + 1, "of", len(want))
        print("  got ", got[first] if first < len(got) else None)
        print("  want", want[first] if first < len(want) else None)
sys.exit(1 if bad else 0)
