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
import json, os, re, sys

builtin_dir, user_dir = sys.argv[1], sys.argv[2]

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
        m["dir"] = os.path.join(base, name)
        m["user"] = is_user
        qml = os.path.join(base, name, "Widget.qml")
        if is_user and not m.get("source") and os.path.isfile(qml):
            m["restrictedIssues"] = inspect(qml)
        out.append(m)
print(json.dumps(out))
