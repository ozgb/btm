"""A local core gate: runs CORE once per vector of reference/core/<group>.json and compares every line.

    python3 -I tools/gate.py CORE [--group G]... [--vector NAME]... [--module PATH]
"""
import json, lzma, os, shutil, subprocess, sys, tempfile

KIT = os.environ.get("KIT", "/data")
REF = os.path.join(KIT, "reference", "core")


def unpack(src, dst):
    for root, _, files in os.walk(src):
        rel = os.path.relpath(root, src)
        os.makedirs(os.path.join(dst, rel), exist_ok=True)
        for f in files:
            p = os.path.join(root, f)
            if f.endswith(".xz"):
                with lzma.open(p) as i, open(os.path.join(dst, rel, f[:-3]), "wb") as o:
                    shutil.copyfileobj(i, o)
            else:
                shutil.copy(p, os.path.join(dst, rel, f))


def run_vector(core, group, vec, inputs, module):
    with tempfile.TemporaryDirectory() as d:
        if os.path.isdir(inputs):
            unpack(inputs, d)
        shutil.copy(os.path.join(KIT, "data", "trace_schema.json"), d)
        if module:
            shutil.copy(module, os.path.join(d, "module.wasm"))
        with open(os.path.join(d, "CALLS"), "w") as f:
            for c in vec["calls"]:
                f.write("\t".join(c) + "\n")
        r = subprocess.run([core, group, "CALLS", "OUT"], cwd=d, capture_output=True, timeout=120)
        if r.returncode:
            return "failed", f"exit {r.returncode} {r.stderr.decode()[:300]}"
        try:
            lines = open(os.path.join(d, "OUT")).read().split("\n")
        except OSError:
            return "failed", "no OUT"
        lines = [l for l in lines if l]
        for i, (name, val) in enumerate(vec["results"]):
            if i >= len(lines):
                return "differs", f"line {i+1} {name}: missing"
            n, _, v = lines[i].partition("\t")
            try:
                got = json.loads(v)
            except ValueError:
                return "failed", f"malformed line {i+1}: {lines[i][:200]}"
            if n != name or got != val or json.dumps(got) != json.dumps(val) and not eq(got, val):
                return "differs", f"line {i+1} call {vec['calls'][i]}\n  got  {lines[i][:600]}\n  want {name}\t{json.dumps(val)[:600]}"
        return "equal", ""


def eq(a, b):
    if type(a) != type(b):
        return False
    if isinstance(a, list):
        return len(a) == len(b) and all(eq(x, y) for x, y in zip(a, b))
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(eq(a[k], b[k]) for k in a)
    return a == b


def main():
    a = sys.argv[1:]
    core = os.path.abspath(a[0])
    groups, vecs, module = [], [], "/work/wind_meteors/module.wasm"
    i = 1
    while i < len(a):
        if a[i] == "--group": groups.append(a[i+1]); i += 2
        elif a[i] == "--vector": vecs.append(a[i+1]); i += 2
        elif a[i] == "--module": module = a[i+1]; i += 2
        else: sys.exit("bad arg")
    if not groups:
        groups = ["hash", "rand", "events", "commands", "loader", "fuel", "host", "console"]
    bad = 0
    for g in groups:
        data = json.load(open(os.path.join(REF, g + ".json")))
        tally = {}
        for vec in data["vectors"]:
            if vecs and vec["name"] not in vecs:
                continue
            st, msg = run_vector(core, g, vec, os.path.join(REF, g), module)
            tally[st] = tally.get(st, 0) + 1
            if st != "equal":
                bad += 1
                print(f"{g}/{vec['name']}: {st} {msg}")
        print(g, tally)
    sys.exit(1 if bad else 0)

main()
