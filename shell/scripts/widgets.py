#!/usr/bin/env python3
"""The widget library as JSON: every widget.json under the built-in and user directories, and for a
user's own QML widget, whether it is safe to run restricted.

A restricted widget may import only pure-UI Qt modules and must not use the escape hatches that reach
outside the drawing: processes, files, sockets, the network, dynamic QML, or external URLs. One that
breaks either rule needs `trust: true` in its manifest to run, because it reaches past the sandbox.
This is a source inspection, not a hard boundary: it stops a widget's overreach, not hostile code
written to defeat it.

    widgets.py <builtin-dir> <user-dir>
"""
import json, os, re, subprocess, sys

# Only these imports for a restricted widget: drawing, layout, shapes, effects, animation, the QML core.
ALLOWED_IMPORTS = {
    "QtQuick", "QtQuick.Layouts", "QtQuick.Shapes", "QtQuick.Effects",
    "QtQuick.Particles", "QtQml", "QtQml.Models",
}
IMPORT_RE = re.compile(r'^\s*import\s+([A-Za-z_][\w.]*)', re.M)
# Tokens that reach outside the drawing, refused for a restricted widget.
FORBIDDEN_TOKENS = [
    "XMLHttpRequest",         # network
    "Qt.createQmlObject",     # code from a string
    "Qt.createComponent",     # code from a file
    "Qt.openUrlExternally",   # hand a URL to the system
    "Qt.exit", "Qt.quit",
    "Loader",                 # can load external QML that is not scanned
    "WorkerScript",           # code off-thread
]
# A relative import pulls in a .qml or .js file that this scan never saw.
RELATIVE_IMPORT_RE = re.compile(r'^\s*import\s+["\']', re.M)
DOT_IMPORT_RE = re.compile(r'^\s*\.import\s', re.M)


def inspect(qml_path):
    reasons = []
    try:
        src = open(qml_path, encoding="utf-8", errors="replace").read()
    except OSError:
        return ["its Widget.qml could not be read"]
    for name in IMPORT_RE.findall(src):
        if name not in ALLOWED_IMPORTS:
            reasons.append("imports " + name)
    if RELATIVE_IMPORT_RE.search(src) or DOT_IMPORT_RE.search(src):
        reasons.append("imports another file")
    for tok in FORBIDDEN_TOKENS:
        if tok in src:
            reasons.append("uses " + tok)
    # De-duplicate, keep order.
    seen, out = set(), []
    for r in reasons:
        if r not in seen:
            seen.add(r); out.append(r)
    return out


def about_text(path):
    """A README as the listing's text: its first heading dropped, since the name is already shown."""
    try:
        text = open(path, encoding="utf-8", errors="replace").read().strip()
    except OSError:
        return ""
    lines = text.split("\n")
    if lines and lines[0].startswith("#"):
        lines = lines[1:]
    return "\n".join(lines).strip()


def scan(builtin_dir, user_dir):
    out = []
    for base, is_user in ((builtin_dir, False), (user_dir, True)):
        if not os.path.isdir(base):
            continue
        for name in sorted(os.listdir(base)):
            mpath = os.path.join(base, name, "widget.json")
            if not os.path.isfile(mpath):
                continue
            try:
                m = json.load(open(mpath, encoding="utf-8"))
            except (OSError, ValueError):
                continue
            d = os.path.join(base, name)
            m["dir"] = d
            m["user"] = is_user
            # A widget installed from a source carries the record the store wrote: where from, which version.
            rec = os.path.join(d, ".isle-install.json")
            if is_user and os.path.isfile(rec):
                try:
                    m["installed"] = json.load(open(rec, encoding="utf-8"))
                except (OSError, ValueError):
                    pass
            qml = os.path.join(d, "Widget.qml")
            if is_user and not m.get("source") and os.path.isfile(qml):
                m["restrictedIssues"] = inspect(qml)
            # The listing: a README stands in for `about`, a CHANGELOG is shown per version, and screenshots are
            # the images in screenshots/ or those the manifest names, as absolute paths.
            if not m.get("about"):
                for rd in ("README.md", "README"):
                    rp = os.path.join(d, rd)
                    if os.path.isfile(rp):
                        m["about"] = about_text(rp)
                        break
            cl = os.path.join(d, "CHANGELOG.md")
            if os.path.isfile(cl):
                m["changelog"] = open(cl, encoding="utf-8", errors="replace").read().strip()
            shots = []
            for rel in m.get("screenshots", []) or []:
                ap = os.path.join(d, rel)
                if os.path.isfile(ap):
                    shots.append(ap)
            sd = os.path.join(d, "screenshots")
            if not shots and os.path.isdir(sd):
                shots = [os.path.join(sd, f) for f in sorted(os.listdir(sd)) if f.lower().endswith((".png", ".jpg", ".jpeg", ".webp"))]
            m["screenshots"] = shots
            # Where it came from: a git checkout carries its origin, which is what updates come from.
            git = os.path.join(d, ".git")
            if os.path.isdir(git) or os.path.isfile(git):
                try:
                    r = subprocess.run(["git", "-C", d, "remote", "get-url", "origin"], capture_output=True, text=True, timeout=3)
                    if r.returncode == 0 and r.stdout.strip():
                        m["origin"] = r.stdout.strip()
                    t = subprocess.run(["git", "-C", d, "describe", "--tags", "--exact-match"], capture_output=True, text=True, timeout=3)
                    if t.returncode == 0:
                        m["installedTag"] = t.stdout.strip()
                except Exception:
                    pass
            out.append(m)
    return out


if __name__ == "__main__":
    print(json.dumps(scan(sys.argv[1], sys.argv[2])))
