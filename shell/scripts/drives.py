#!/usr/bin/env python3
"""A cloud drive as a provider (the contract: docs/ARCHITECTURE.md, "Providers"): the accounts signed in to
one service, whether each is mounted, and what can be done with it. rclone is the engine for every service,
so the manifest says which of its backends this provider is.

    drives.py --backend protondrive status | list | fields | action <id>

`fields` says what this service needs to add an account, taken from rclone's own description of the backend:
a browser for the ones that authenticate that way, a few values for the ones that do not.

An account is mounted by a systemd user unit, `isle-drive@<remote>.service`, so a mount outlives the shell
and can start at login. Adding one needs a browser and a prompt, so it runs in a terminal window.
"""
import json, os, re, shutil, subprocess, sys

RCLONE = shutil.which("rclone")
ROOT = os.path.expanduser("~/Drives")
UNIT = "isle-drive@%s.service"
# rclone's own name for a backend, and what a person calls it.
# What a field is called, where its own name reads badly.
PRETTY = {"2fa": "2FA", "apple_id": "Apple ID", "mailbox_password": "Mailbox password", "otp_secret_key": "OTP secret"}
KNOWN = {"protondrive": "Proton Drive", "drive": "Google Drive", "dropbox": "Dropbox",
         "onedrive": "OneDrive", "iclouddrive": "iCloud Drive", "box": "Box", "s3": "S3", "webdav": "WebDAV"}

args = sys.argv[1:]
backend = ""
if args and args[0] == "--backend":
    backend, args = args[1], args[2:]
cmd = args[0] if args else "status"


def run(*a, **kw):
    return subprocess.run(list(a), capture_output=True, text=True, timeout=kw.get("timeout", 20))


def remotes():
    """The configured accounts of this backend: [{ name, mounted, at }]."""
    if not RCLONE:
        return []
    try:
        p = run(RCLONE, "config", "dump")
        conf = json.loads(p.stdout or "{}")
    except Exception:
        return []
    out = []
    for name, cfg in sorted(conf.items()):
        if backend and cfg.get("type") != backend:
            continue
        out.append({"name": name, "type": cfg.get("type", ""), "mounted": mounted(name), "at": os.path.join(ROOT, name)})
    return out


def mounted(name):
    at = os.path.join(ROOT, name)
    try:
        with open("/proc/self/mountinfo") as f:
            return any(" " + at + " " in line for line in f)
    except OSError:
        return False


def unit_state(name):
    p = run("systemctl", "--user", "is-active", UNIT % name)
    return p.stdout.strip()


def write_unit():
    """The unit that mounts one account, written once, named by the remote."""
    d = os.path.expanduser("~/.config/systemd/user")
    os.makedirs(d, exist_ok=True)
    path = os.path.join(d, "isle-drive@.service")
    text = """# Written by Isle's drives provider. One mount per account, named by its rclone remote.
[Unit]
Description=Isle drive mount for %i
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStartPre=/usr/bin/mkdir -p %h/Drives/%i
ExecStart=/usr/bin/rclone mount %i: %h/Drives/%i --vfs-cache-mode writes --dir-cache-time 30s --poll-interval 1m --umask 077
ExecStop=/bin/fusermount3 -uz %h/Drives/%i
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
"""
    if not os.path.exists(path) or open(path).read() != text:
        open(path, "w").write(text)
        run("systemctl", "--user", "daemon-reload")
    return path


def fields():
    """What this backend asks for: [{ name, label, password, required }], and whether a browser does the rest."""
    if not RCLONE:
        return {"fields": [], "browser": False, "error": "rclone is not on this machine"}
    try:
        p = run(RCLONE, "config", "providers")
        all_of = json.loads(p.stdout or "[]")
    except Exception:
        return {"fields": [], "browser": False, "error": "rclone would not say what it supports"}
    spec = next((x for x in all_of if x.get("Name") == backend), None)
    if not spec:
        return {"fields": [], "browser": False, "error": "rclone does not know %s" % backend}
    wanted, browser = [], False
    for o in spec.get("Options") or []:
        if o.get("Name") in ("token", "client_id", "client_secret"):
            browser = True
            continue
        if o.get("Advanced"):
            continue
        if not o.get("Required") and o.get("Name") not in ("2fa", "mailbox_password"):
            continue
        wanted.append({"name": o["Name"],
                       "label": PRETTY.get(o["Name"], o["Name"].replace("_", " ").capitalize()),
                       "help": (o.get("Help") or "").split("\n")[0],
                       "password": bool(o.get("IsPassword")),
                       "required": bool(o.get("Required"))})
    # Nothing to fill in means the browser is the whole of it.
    return {"fields": wanted, "browser": browser and not [f for f in wanted if f["required"]], "error": ""}


def service_name():
    return KNOWN.get(backend, backend or "Cloud drive")


if cmd == "fields":
    out = fields()
    out["service"] = service_name()
    out["backend"] = backend
    print(json.dumps(out))

elif cmd == "status":
    out = {"installed": bool(RCLONE), "ready": False, "state": "", "actions": [], "error": ""}
    if not RCLONE:
        out["state"] = "rclone is not on this machine"
    else:
        accounts = remotes()
        out["ready"] = True
        out["state"] = ("%d account%s" % (len(accounts), "" if len(accounts) == 1 else "s")) if accounts \
            else "No account yet"
        out["actions"] = [{"id": "add", "label": "Add account", "terminal": True, "primary": not accounts},
                          {"id": "refresh", "label": "Look again"}]
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    for r in remotes():
        state = unit_state(r["name"])
        where = r["at"].replace(os.path.expanduser("~"), "~")
        sub = ("Mounted at " + where) if r["mounted"] else ("Starting" if state == "activating" else "Not mounted")
        acts = ([{"id": "open", "label": "Open"}, {"id": "unmount", "label": "Unmount"}]
                if r["mounted"] else [{"id": "mount", "label": "Mount", "primary": True}])
        acts.append({"id": "atlogin", "label": "At login", "on": run("systemctl", "--user", "is-enabled", UNIT % r["name"]).stdout.strip() == "enabled"})
        acts.append({"id": "forget", "label": "Remove", "danger": True})
        out["items"].append({"id": r["name"], "title": r["name"], "subtitle": sub, "actions": acts})
    print(json.dumps(out))

elif cmd == "action":
    what = args[1] if len(args) > 1 else ""
    data = sys.stdin.read().strip()
    name = data.split("\n")[0].strip()
    if what == "add":
        # The values the sheet collected, as JSON: { name, values }. A service that authenticates in a browser
        # sends nothing but a name, and rclone opens the browser itself.
        try:
            asked = json.loads(data or "{}")
        except ValueError:
            asked = {}
        name = str(asked.get("name") or "").strip()
        if not re.match(r"^[A-Za-z0-9_-]+$", name):
            sys.stderr.write("A name of letters, digits, dashes or underscores\n")
            sys.exit(2)
        pairs = []
        for k, v in (asked.get("values") or {}).items():
            if str(v).strip():
                pairs += [str(k), str(v)]
        p = run(RCLONE, "config", "create", name, backend or "webdav", *pairs, "--obscure", timeout=180)
        if p.returncode != 0:
            sys.stderr.write((p.stderr.strip().split("\n")[-1] or "rclone would not add it") + "\n")
        sys.exit(p.returncode)
    if what == "browser":
        # The consent page: rclone runs its own flow, which needs a terminal to say what it is doing.
        name = data.split("\n")[0].strip() or backend
        os.execvp(RCLONE, [RCLONE, "config", "create", name, backend or "webdav"])
    if what == "refresh":
        sys.exit(0)
    if not name:
        sys.stderr.write("No account named\n")
        sys.exit(2)
    if what == "mount":
        write_unit()
        p = run("systemctl", "--user", "start", UNIT % name, timeout=60)
        if p.returncode != 0:
            sys.stderr.write((p.stderr.strip().split("\n")[-1] or "The mount would not start") + "\n")
        sys.exit(p.returncode)
    if what == "unmount":
        p = run("systemctl", "--user", "stop", UNIT % name, timeout=60)
        sys.exit(p.returncode)
    if what == "atlogin":
        on = run("systemctl", "--user", "is-enabled", UNIT % name).stdout.strip() == "enabled"
        write_unit()
        run("systemctl", "--user", "disable" if on else "enable", UNIT % name)
        sys.exit(0)
    if what == "open":
        subprocess.Popen(["xdg-open", os.path.join(ROOT, name)], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        sys.exit(0)
    if what == "forget":
        run("systemctl", "--user", "stop", UNIT % name, timeout=60)
        run("systemctl", "--user", "disable", UNIT % name)
        p = run(RCLONE, "config", "delete", name)
        if p.returncode != 0:
            sys.stderr.write("rclone would not remove it\n")
        sys.exit(p.returncode)
    sys.stderr.write("No such action\n")
    sys.exit(2)
