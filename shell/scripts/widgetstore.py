#!/usr/bin/env python3
"""Widget registries: fetch each source's index, and install, update or remove a widget from one.

A source is a git repository whose root holds `index.json`:

    { "name": "Isle widgets",
      "widgets": [ { "id": "uptime", "path": "widgets/uptime" },
                   { "id": "nowplaying", "repo": "https://github.com/kit/isle-nowplaying", "ref": "v0.3.1",
                     "name": "Now Playing", "version": "0.3.1", ... } ] }

A `path` entry lives in the registry and its widget.json, README, CHANGELOG and screenshots are read from
there; a `repo` entry is its own repository, cloned at `ref`, and the index carries its listing (with
`screenshots` relative to the registry). Sources are cloned shallow under the cache directory and pulled
on each index; a clone is what makes screenshots local files.

    widgetstore.py index   <cache-dir> [--fresh] <source-url>...
    widgetstore.py install <cache-dir> <user-dir> <source-url> <id>
    widgetstore.py remove  <user-dir> <id>
"""
import hashlib, json, os, re, shutil, subprocess, sys, tempfile, time

GIT_ENV = dict(os.environ, GIT_TERMINAL_PROMPT="0", GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=15")
SHOT_EXT = (".png", ".jpg", ".jpeg", ".webp")
INSTALL_RECORD = ".isle-install.json"


def git(args, cwd=None, timeout=120):
    return subprocess.run(["git"] + args, cwd=cwd, env=GIT_ENV, capture_output=True, text=True, timeout=timeout)


def slug(url):
    tail = re.sub(r"[^A-Za-z0-9]+", "-", url.rstrip("/").rsplit("/", 1)[-1].replace(".git", "")).strip("-") or "source"
    return tail + "-" + hashlib.sha1(url.encode()).hexdigest()[:8]


def git_error(r, fallback):
    """git's last stderr line, in plain words for the two it says badly."""
    line = (r.stderr.strip().splitlines() or [fallback])[-1]
    if "could not read Username" in line or "Authentication failed" in line:
        return "not found, or private"
    if "Could not resolve host" in line or "unable to access" in line:
        return "could not be reached"
    return line.replace("fatal: ", "")


def sync_source(cache, url, fresh):
    """The source's clone under the cache, pulled when `fresh` or when it is older than an hour."""
    d = os.path.join(cache, slug(url))
    os.makedirs(cache, exist_ok=True)
    if not os.path.isdir(os.path.join(d, ".git")):
        r = git(["clone", "-q", "--depth", "1", url, d])
        if r.returncode != 0:
            shutil.rmtree(d, ignore_errors=True)
            return None, git_error(r, "clone failed")
        return d, ""
    stamp = os.path.join(d, ".git", "FETCH_HEAD")
    if fresh or not os.path.exists(stamp) or time.time() - os.path.getmtime(stamp) > 3600:
        r = git(["fetch", "-q", "--depth", "1", "origin", "HEAD"], cwd=d)
        if r.returncode == 0:
            git(["reset", "-q", "--hard", "FETCH_HEAD"], cwd=d)
        else:
            return d, git_error(r, "fetch failed")
    return d, ""


def read_text(path):
    try:
        return open(path, encoding="utf-8", errors="replace").read().strip()
    except OSError:
        return ""


def shots_in(base, names):
    out = []
    for rel in names or []:
        p = os.path.join(base, rel)
        if os.path.isfile(p):
            out.append(os.path.abspath(p))
    return out


def listing(src_dir, url, entry):
    """One catalogue entry, with the widget's own files folded in when it lives in the registry."""
    m = dict(entry)
    wd = os.path.join(src_dir, entry["path"]) if entry.get("path") else ""
    if wd:
        if not os.path.isdir(wd):
            return None
        try:
            m.update(json.load(open(os.path.join(wd, "widget.json"), encoding="utf-8")))
        except (OSError, ValueError):
            return None
        m["id"] = entry.get("id") or m.get("id")
        m["about"] = m.get("about") or scanner().about_text(os.path.join(wd, "README.md"))
        m["changelog"] = read_text(os.path.join(wd, "CHANGELOG.md"))
        shots = shots_in(wd, m.get("screenshots"))
        sd = os.path.join(wd, "screenshots")
        if not shots and os.path.isdir(sd):
            shots = [os.path.join(sd, f) for f in sorted(os.listdir(sd)) if f.lower().endswith(SHOT_EXT)]
        m["screenshots"] = shots
        # The widget's QML is inspected the same way an installed one is, so the badge is right before install.
        m["restrictedIssues"] = inspect_qml(os.path.join(wd, "Widget.qml")) if not m.get("source") else []
    else:
        m["screenshots"] = shots_in(src_dir, m.get("screenshots"))
    if not m.get("id"):
        return None
    m["catalogue"] = True
    m["sourceUrl"] = url
    m["user"] = True
    return m


def scanner():
    """widgets.py, the scanner installed widgets go through, so a listing gets the same reading."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import widgets
    return widgets


def inspect_qml(path):
    """The scanner's verdict on a widget's Widget.qml, so a listing carries the same needs-trust badge."""
    if not os.path.isfile(path):
        return []
    try:
        return scanner().inspect(path)
    except Exception:
        return []


def index(cache, urls, fresh):
    out = []
    for url in urls:
        d, err = sync_source(cache, url, fresh)
        name = url
        widgets = []
        if d:
            try:
                idx = json.load(open(os.path.join(d, "index.json"), encoding="utf-8"))
                name = idx.get("name") or name
                widgets = idx.get("widgets") or []
            except (OSError, ValueError):
                err = err or "no index.json in this source"
        entries = [e for e in (listing(d, url, w) for w in widgets) if e] if d else []
        for e in entries:
            e["sourceName"] = name
        out.append({"url": url, "name": name, "error": err, "widgets": entries})
    print(json.dumps(out))


def install(cache, user_dir, url, wid):
    d, err = sync_source(cache, url, False)
    if not d:
        return fail(err)
    try:
        idx = json.load(open(os.path.join(d, "index.json"), encoding="utf-8"))
    except (OSError, ValueError):
        return fail("no index.json in this source")
    entry = next((w for w in idx.get("widgets") or [] if w.get("id") == wid), None)
    if not entry:
        return fail("not in this source any more")
    os.makedirs(user_dir, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix=".install-", dir=user_dir)
    stage = os.path.join(tmp, wid)
    try:
        if entry.get("path"):
            shutil.copytree(os.path.join(d, entry["path"]), stage, symlinks=False)
        elif entry.get("repo"):
            args = ["clone", "-q", "--depth", "1"]
            if entry.get("ref"):
                args += ["--branch", entry["ref"]]
            r = git(args + [entry["repo"], stage], timeout=300)
            if r.returncode != 0:
                return fail(git_error(r, "clone failed"))
        else:
            return fail("the index names neither a path nor a repo")
        try:
            m = json.load(open(os.path.join(stage, "widget.json"), encoding="utf-8"))
        except (OSError, ValueError):
            return fail("no widget.json in the download")
        record = {"source": url, "version": m.get("version", entry.get("version", "")), "installedAt": int(time.time()),
                  "path": entry.get("path", ""), "repo": entry.get("repo", ""), "ref": entry.get("ref", "")}
        json.dump(record, open(os.path.join(stage, INSTALL_RECORD), "w"), indent=2)
        final = os.path.join(user_dir, wid)
        old = os.path.join(tmp, "old")
        if os.path.lexists(final):
            os.rename(final, old)
        os.rename(stage, final)
        print(json.dumps({"ok": True, "id": wid, "version": record["version"]}))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def remove(user_dir, wid):
    d = os.path.join(user_dir, wid)
    if os.path.isdir(d) and os.path.isfile(os.path.join(d, INSTALL_RECORD)):
        shutil.rmtree(d)
        print(json.dumps({"ok": True, "id": wid}))
    else:
        fail("not installed from a source")


def fail(msg):
    print(json.dumps({"ok": False, "error": msg}))
    sys.exit(1)


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd = argv[1]
    try:
        if cmd == "index":
            fresh = "--fresh" in argv
            args = [a for a in argv[2:] if a != "--fresh"]
            index(args[0], args[1:], fresh)
        elif cmd == "install":
            install(*argv[2:6])
        elif cmd == "remove":
            remove(*argv[2:4])
        else:
            return 2
    except subprocess.TimeoutExpired:
        fail("timed out talking to the source")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
