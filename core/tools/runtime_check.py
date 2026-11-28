"""Check the runtime group against /data/runtime/vectors.

    python3 -I tools/runtime_check.py CORE [entry ...]
"""
import os, struct, subprocess, sys, tempfile, time

VEC = "/data/runtime/vectors"


def read_fields(buf, p, types):
    out = []
    for t in types:
        if t == "i32":
            out.append(str(struct.unpack_from("<i", buf, p)[0])); p += 4
        elif t == "u32":
            out.append(str(struct.unpack_from("<I", buf, p)[0])); p += 4
        elif t == "f64":
            out.append(str(struct.unpack_from("<Q", buf, p)[0])); p += 8
        elif t == "str":
            n = struct.unpack_from("<I", buf, p)[0]; p += 4
            out.append('"%s"' % buf[p:p + n].hex()); p += n
        elif t.startswith("bytes"):
            n = int(t[5:])
            out.append('"%s"' % buf[p:p + n].hex()); p += n
        else:
            raise ValueError(t)
    return out, p


def main():
    core = sys.argv[1]
    only = sys.argv[2:]
    layouts = {}
    for line in open(os.path.join(VEC, "layouts.txt")):
        n, i, o = line.split()
        layouts[n] = (i.split(","), o.split(","))
    tot_bad = 0
    for name, (ins, outs) in layouts.items():
        if only and name not in only:
            continue
        buf = open(os.path.join(VEC, name + ".bin"), "rb").read()
        p = 0
        calls, exp = [], []
        while p < len(buf):
            a, p = read_fields(buf, p, ins)
            o, p = read_fields(buf, p, outs)
            calls.append("%s\t[%s]" % (name, ", ".join(a)))
            exp.append("%s\t[%s]" % (name, ", ".join(o)))
        with tempfile.TemporaryDirectory() as d:
            cf, of = os.path.join(d, "CALLS"), os.path.join(d, "OUT")
            open(cf, "w").write("\n".join(calls) + "\n")
            t = time.time()
            r = subprocess.run([core, "runtime", cf, of], capture_output=True)
            dt = time.time() - t
            got = open(of).read().split("\n")[:-1] if os.path.exists(of) else []
        bad = [i for i in range(len(exp)) if i >= len(got) or got[i] != exp[i]]
        tot_bad += len(bad)
        print("%-16s %6d/%-6d %.1fs rc=%d" % (name, len(exp) - len(bad), len(exp), dt, r.returncode))
        for i in bad[:2]:
            print("   call", calls[i][:150]); print("   want", exp[i][:150]); print("   got ", (got[i] if i < len(got) else "<missing>")[:150])
    print("total bad", tot_bad)


main()
