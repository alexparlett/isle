#!/usr/bin/env python3
"""Author tooling for a user's widget: scaffold one, check it against the contract, and tag a release.

    widgetauthor.py new      <user-dir> <id> <name>      a QML widget with a full manifest, README and CHANGELOG
    widgetauthor.py validate <dir>                       JSON { ok, errors, warnings }
    widgetauthor.py share    <dir>                       git init if needed, commit, tag v<version>; JSON { ok, tag, remote, note }
"""
import json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import widgets  # noqa: E402  the scanner: the same sandbox reading the shell applies

CATEGORIES = ["Shell", "System", "Hardware", "Media", "Yours"]
PERMISSIONS = ["audio", "audio.write", "network", "media", "media.write", "system", "notifications", "notifications.write", "power", "bluetooth", "vpn", "notify"]
FETCH = re.compile(r"^fetch:[a-z0-9.-]+\.[a-z]{2,}$", re.I)
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")
SIZE = re.compile(r"^(\d+)x(\d+)$")
ID = re.compile(r"^[a-z][a-z0-9-]*$")


def git(args, cwd):
    return subprocess.run(["git"] + args, cwd=cwd, capture_output=True, text=True, timeout=60)


def git_user():
    name = git(["config", "user.name"], os.path.expanduser("~")).stdout.strip()
    return name or os.environ.get("USER", "")


def icon_names():
    try:
        return set(open(os.path.join(HERE, "..", "assets", "icons.txt"), encoding="utf-8").read().split())
    except OSError:
        return set()


def out(obj, code=0):
    print(json.dumps(obj))
    return code


def new(user_dir, wid, name):
    if not ID.match(wid):
        return out({"ok": False, "error": "an id is lower-case letters, digits and dashes"}, 1)
    d = os.path.join(user_dir, wid)
    if os.path.exists(d):
        return out({"ok": False, "error": "a widget with that id already exists"}, 1)
    os.makedirs(os.path.join(d, "screenshots"))
    manifest = {
        "id": wid, "name": name, "glyph": "layout-grid", "category": "Yours",
        "description": "What it shows, in a line",
        "version": "0.1.0", "author": {"name": git_user(), "url": ""}, "license": "MIT", "homepage": "",
        "sizes": ["3x2", "3x4"], "default": "3x2",
        "permissions": [],
        "settings": [],
    }
    json.dump(manifest, open(os.path.join(d, "widget.json"), "w", encoding="utf-8"), indent=2)
    open(os.path.join(d, "Widget.qml"), "w", encoding="utf-8").write(f'''import QtQuick

// {name}: draws inside the card; the card supplies the title and the chrome.
// `host` is the shell, gated by the manifest's permissions; `host.theme` is always there.
Item {{
    id: root
    property var manifest
    property string size: "3x2"
    property var settings: ({{}})
    property var host
    property string title: "{name}"
    property string meta: ""

    Text {{
        anchors.centerIn: parent
        text: "Hello from {name}"
        color: root.host ? root.host.theme.text : "white"
        font.pixelSize: 16
        font.family: root.host ? root.host.theme.fontUi : ""
    }}
}}
''')
    open(os.path.join(d, "README.md"), "w", encoding="utf-8").write(f"# {name}\n\nWhat it shows, and why someone would want it on their dashboard.\n")
    open(os.path.join(d, "CHANGELOG.md"), "w", encoding="utf-8").write("## 0.1.0\n- First release.\n")
    open(os.path.join(d, ".gitignore"), "w", encoding="utf-8").write(".isle-install.json\n")
    return out({"ok": True, "id": wid, "dir": d})


def validate(d):
    errors, warnings = [], []
    mpath = os.path.join(d, "widget.json")
    try:
        m = json.load(open(mpath, encoding="utf-8"))
    except OSError:
        return out({"ok": False, "errors": ["no widget.json"], "warnings": []})
    except ValueError as e:
        return out({"ok": False, "errors": [f"widget.json does not parse: {e}"], "warnings": []})
    wid = os.path.basename(os.path.normpath(d))
    if m.get("id") != wid:
        errors.append(f'"id" must be "{wid}", the folder name')
    elif not ID.match(wid):
        errors.append('"id" is lower-case letters, digits and dashes')
    for key in ("name", "description"):
        if not str(m.get(key, "")).strip():
            errors.append(f'"{key}" is missing')
    if not SEMVER.match(str(m.get("version", ""))):
        errors.append('"version" must be three numbers, like 1.2.0')
    if m.get("category") not in CATEGORIES:
        errors.append('"category" is one of ' + ", ".join(CATEGORIES))
    sizes = m.get("sizes") or []
    if not sizes:
        errors.append('"sizes" is empty')
    for s in sizes:
        mm = SIZE.match(str(s))
        if not mm or not (1 <= int(mm.group(1)) <= 12 and 1 <= int(mm.group(2)) <= 8):
            errors.append(f'size "{s}" is not columns x rows within 12 x 8')
    if sizes and m.get("default") not in sizes:
        errors.append('"default" must be one of "sizes"')
    icons = icon_names()
    if icons and m.get("glyph") and m["glyph"] not in icons:
        errors.append(f'glyph "{m["glyph"]}" is not in the shell\'s icon set')
    for p in m.get("permissions") or []:
        if p not in PERMISSIONS and not FETCH.match(str(p)):
            errors.append(f'permission "{p}" is not one the host offers (' + ", ".join(PERMISSIONS) + ', or fetch:<host>)')
    author = m.get("author")
    if not (isinstance(author, dict) and str(author.get("name", "")).strip()) and not (isinstance(author, str) and author.strip()):
        warnings.append('"author" has no name; the store will show "You"')
    if not m.get("license"):
        warnings.append('"license" is missing; others cannot tell what they may do with it')
    for st in m.get("settings") or []:
        if not isinstance(st, dict) or not st.get("key") or st.get("type") not in ("toggle", "choice", "number", "text"):
            errors.append('each setting needs a "key" and a "type" of toggle, choice, number or text')
    if m.get("source"):
        if not isinstance(m["source"], dict) or not (m["source"].get("command") or m["source"].get("file")):
            errors.append('"source" needs a "command" or a "file"')
    else:
        qml = os.path.join(d, "Widget.qml")
        if not os.path.isfile(qml):
            errors.append("no Widget.qml, and no \"source\" for the built-in renderer")
        else:
            issues = widgets.inspect(qml)
            if issues and m.get("trust") is not True:
                errors.append("Widget.qml reaches past the sandbox (" + ", ".join(issues) + '); drop that, or ask for full access with "trust": true')
            elif issues:
                warnings.append("asks for full access (" + ", ".join(issues) + "); people will be asked to trust it before it runs")
    if not os.path.isfile(os.path.join(d, "README.md")):
        warnings.append("no README.md; the listing will show only the description")
    shots = [f for f in (m.get("screenshots") or []) if os.path.isfile(os.path.join(d, f))]
    sd = os.path.join(d, "screenshots")
    if not shots and not (os.path.isdir(sd) and any(f.lower().endswith((".png", ".jpg", ".jpeg", ".webp")) for f in os.listdir(sd))):
        warnings.append("no screenshot; the store will draw it live instead, which needs it installed")
    cl = os.path.join(d, "CHANGELOG.md")
    if not os.path.isfile(cl):
        warnings.append("no CHANGELOG.md")
    elif str(m.get("version", "")) not in open(cl, encoding="utf-8", errors="replace").read():
        warnings.append(f'CHANGELOG.md has no entry for {m.get("version")}')
    return out({"ok": not errors, "errors": errors, "warnings": warnings})


def share(d):
    try:
        m = json.load(open(os.path.join(d, "widget.json"), encoding="utf-8"))
    except (OSError, ValueError):
        return out({"ok": False, "error": "no readable widget.json"}, 1)
    version = str(m.get("version", ""))
    if not SEMVER.match(version):
        return out({"ok": False, "error": "set a version like 1.0.0 first"}, 1)
    tag = "v" + version
    if not os.path.isdir(os.path.join(d, ".git")):
        r = git(["init", "-q"], d)
        if r.returncode != 0:
            return out({"ok": False, "error": r.stderr.strip() or "git init failed"}, 1)
    git(["add", "-A"], d)
    if git(["diff", "--cached", "--quiet"], d).returncode != 0:
        r = git(["-c", "user.name=" + (git_user() or "isle"), "-c", "user.email=" + (git(["config", "user.email"], d).stdout.strip() or "isle@localhost"),
                 "commit", "-q", "-m", tag], d)
        if r.returncode != 0:
            return out({"ok": False, "error": (r.stderr.strip().splitlines() or ["commit failed"])[-1]}, 1)
    if git(["rev-parse", "-q", "--verify", "refs/tags/" + tag], d).returncode == 0:
        note = f"{tag} was already tagged; bump the version for a new release."
    else:
        git(["tag", tag], d)
        note = f"Tagged {tag}."
    remote = git(["remote", "get-url", "origin"], d).stdout.strip()
    return out({"ok": True, "tag": tag, "remote": remote, "note": note})


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "new" and len(argv) >= 5:
        return new(argv[2], argv[3], " ".join(argv[4:]))
    if cmd == "validate":
        return validate(argv[2])
    if cmd == "share":
        return share(argv[2])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
