#!/usr/bin/env python3
"""Refuse game data and foreign source in the commits of a pull request.

usage: refuse_foreign.py [--allow FILE] BASE HEAD

Walks every commit in BASE..HEAD, merges included, against each parent, and
lists the files each commit adds, copies or renames to. A file is flagged when
it has a game data extension (any case), starts with the archive magic HAPI,
is Go source (.go, go.mod, go.sum), or has a third_party path component.
Looking at each commit, not the final tree, also catches a file that a pull
request adds and later removes.

Each finding prints "<commit> <path>: <reason>". Exit status: 0 clean, 1 a
finding is not allowlisted, 2 usage or git error or a bad allowlist.
"""
import argparse
import json
import subprocess
import sys

GAME_EXT = (
    ".hpi", ".gp3", ".ccx", ".ufo", ".gaf", ".3do", ".cob", ".bos", ".tnt",
    ".ota", ".fbi", ".tdf", ".gui", ".pcx", ".fnt", ".pal", ".wav", ".zrb",
    ".sct",
)
GO_NAMES = ("go.mod", "go.sum")
MAGIC = b"HAPI"
GITLINK = "160000"


class Fatal(Exception):
    pass


def git(*args):
    p = subprocess.run(("git",) + args, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode != 0:
        raise Fatal("git %s: %s" % (" ".join(args), p.stderr.decode("utf-8", "replace").strip()))
    return p.stdout


def text(b):
    return b.decode("utf-8", "surrogateescape")


def blob_head(sha):
    """The first bytes of a blob. Only that much is read."""
    p = subprocess.Popen(("git", "cat-file", "blob", sha),
                         stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        return p.stdout.read(len(MAGIC))
    finally:
        p.stdout.close()
        p.kill()
        p.wait()


def parse_allow(path):
    allowed = {}
    try:
        f = open(path, encoding="utf-8", errors="surrogateescape")
    except FileNotFoundError:
        return allowed
    with f:
        for n, line in enumerate(f, 1):
            s = line.strip()
            if not s or s.startswith("#"):
                continue
            if s.startswith('"'):
                try:
                    name, end = json.JSONDecoder().raw_decode(s)
                except ValueError:
                    raise Fatal("%s:%d: bad quoted path" % (path, n))
                reason = s[end:].strip()
            else:
                parts = s.split(None, 1)
                name = parts[0]
                reason = parts[1].strip() if len(parts) > 1 else ""
            if not reason or reason.startswith("#"):
                raise Fatal("%s:%d: entry for %r has no reason" % (path, n, name))
            allowed[name] = reason
    return allowed


def name_reasons(path):
    low = path.lower()
    base = low.rsplit("/", 1)[-1]
    out = []
    for ext in GAME_EXT:
        if base.endswith(ext):
            out.append("game data extension %s" % ext)
            break
    if base.endswith(".go"):
        out.append("Go source")
    if base in GO_NAMES:
        out.append("Go module file")
    if "third_party" in low.split("/")[:-1] or base == "third_party":
        out.append("third_party path")
    return out


def raw_entries(commit, parent):
    """Yield (mode, blob sha, status, path) for files added, copied or renamed."""
    cmd = ["diff-tree", "-r", "-z", "--no-commit-id", "--diff-filter=ACR", "-M"]
    cmd += [parent, commit] if parent else ["--root", commit]
    toks = git(*cmd).split(b"\0")
    i = 0
    while i < len(toks) and toks[i]:
        head = toks[i].decode("ascii").lstrip(":").split()
        _old_mode, mode, _old_sha, sha, status = head[:5]
        i += 1
        if status[0] in "RC":
            i += 1  # source path
        yield mode, sha, status[0], text(toks[i])
        i += 1


def esc(s):
    return "".join("\\x%02x" % ord(c) if ord(c) < 32 or ord(c) == 127 else c for c in s)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--allow", default="tools/refuse_foreign.allow")
    ap.add_argument("base")
    ap.add_argument("head")
    a = ap.parse_args(argv)

    allowed = parse_allow(a.allow)
    base = text(git("rev-parse", "--verify", "--end-of-options", a.base + "^{commit}")).strip()
    head = text(git("rev-parse", "--verify", "--end-of-options", a.head + "^{commit}")).strip()
    lines = text(git("rev-list", "--parents", "--reverse", "%s..%s" % (base, head))).split("\n")

    magic_cache = {}
    seen = set()
    failures = 0
    allowed_hits = 0
    commits = 0
    for line in lines:
        ids = line.split()
        if not ids:
            continue
        commits += 1
        commit, parents = ids[0], ids[1:]
        for parent in parents or [None]:
            for mode, sha, status, path in raw_entries(commit, parent):
                reasons = name_reasons(path)
                if mode != GITLINK:
                    if sha not in magic_cache:
                        magic_cache[sha] = blob_head(sha) == MAGIC
                    if magic_cache[sha]:
                        reasons.append("starts with HAPI archive magic")
                if not reasons:
                    continue
                key = (commit, path)
                if key in seen:
                    continue
                seen.add(key)
                msg = "%s %s: %s" % (commit, esc(path), "; ".join(reasons))
                if path in allowed:
                    allowed_hits += 1
                    print("%s (allowlisted: %s)" % (msg, allowed[path]))
                else:
                    failures += 1
                    print(msg)
    print("checked %d commit(s) in %s..%s: %d finding(s), %d allowlisted"
          % (commits, base[:12], head[:12], failures, allowed_hits), file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="backslashreplace")
        sys.exit(main(sys.argv[1:]))
    except Fatal as e:
        print("refuse_foreign: %s" % e, file=sys.stderr)
        sys.exit(2)
