#!/usr/bin/env python3
"""A cloud drive as a provider (the contract: docs/ARCHITECTURE.md, "Providers"): the accounts signed in to
one service, whether each is mounted, and what can be done with it. rclone is the engine for every service,
so the manifest says which of its backends this provider is.

    drives.py --backend protondrive status | list | fields | action <id>

`fields` says what this service needs to add an account, taken from rclone's own description of the backend:
a browser for the ones that authenticate that way, a few values for the ones that do not.

An account is mounted by a systemd user unit, `isle-drive@<remote>.service`, so a mount outlives the shell
and comes back at every login: an account is there to be a folder, so there is nothing to switch on.
"""
import json, os, re, shutil, subprocess, sys

RCLONE = shutil.which("rclone")
UNIT = "isle-drive@%s.service"
# What a field is called, where its own name reads badly.
PRETTY = {"2fa": "2FA", "apple_id": "Apple ID", "mailbox_password": "Mailbox password", "otp_secret_key": "OTP secret"}
KNOWN = {"protondrive": "Proton Drive", "drive": "Google Drive", "dropbox": "Dropbox",
         "onedrive": "OneDrive", "iclouddrive": "iCloud Drive", "box": "Box", "s3": "S3", "webdav": "WebDAV"}
ROOT = os.path.expanduser("~/Drives")

args = sys.argv[1:]
backend = ""
if args and args[0] == "--backend":
    backend, args = args[1], args[2:]
cmd = args[0] if args else "status"


def run(*a, **kw):
    # Nothing here asks a question, so stdin is closed: a command that did would hang the shell's actor.
    return subprocess.run(list(a), capture_output=True, text=True, stdin=subprocess.DEVNULL,
                          timeout=kw.get("timeout", 20))


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
        out.append({"name": name, "type": cfg.get("type", ""), "mounted": mounted(name),
                    "at": os.path.join(ROOT, name)})
    return out


def responsive(at):
    """Whether the mount answers at all. A stalled one blocks every reader in the session, so it is asked
    in a process that can be given up on rather than in this one."""
    try:
        return subprocess.run(["ls", "-1", at], capture_output=True, stdin=subprocess.DEVNULL,
                              timeout=5).returncode == 0
    except subprocess.TimeoutExpired:
        return False
    except OSError:
        return False


def mounted(name):
    at = os.path.join(ROOT, name)
    return any(" " + at + " " in line for line in open("/proc/self/mountinfo"))


def unit_state(name):
    p = run("systemctl", "--user", "is-active", UNIT % name)
    return p.stdout.strip()


def write_unit():
    """The unit that mounts one account, written once, named by the remote. True when the file changed.

    Nothing tells a file manager that this folder is not a local disk, so it reads the first bytes of every
    file it lists to sniff its type: reads are cached on disk, the first chunk of one is a megabyte rather
    than rclone's default 128, and a cached file is recognised by its size and time rather than by a hash the
    service will not give without the file. Listings are held for an hour, since a folder of several thousand
    files takes over a minute to list.
    """
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
ExecStart=/usr/bin/rclone mount %i: %h/Drives/%i --vfs-cache-mode full --vfs-cache-max-size 4G \
    --vfs-cache-max-age 168h --vfs-read-chunk-size 1M --vfs-read-chunk-size-limit 128M \
    --vfs-fast-fingerprint --dir-cache-time 1h --poll-interval 1m --timeout 30s --contimeout 15s --umask 077
ExecStop=/bin/fusermount3 -uz %h/Drives/%i
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
"""
    if os.path.exists(path) and open(path).read() == text:
        return False
    open(path, "w").write(text)
    run("systemctl", "--user", "daemon-reload")
    return True


def start(name):
    """Serve it and mount it now, and at every login: an account is there to be a folder."""
    changed = write_unit()
    what = "restart" if changed and unit_state(name) == "active" else "start"
    p = run("systemctl", "--user", "enable", UNIT % name, timeout=60)
    if p.returncode == 0:
        p = run("systemctl", "--user", what, UNIT % name, timeout=120)
    if p.returncode != 0:
        sys.stderr.write((p.stderr.strip().split("\n")[-1] or "The drive would not start") + "\n")
        return p.returncode
    return 0


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
        # `add` takes the values Settings collected on stdin; `browser` hands the consent flow a terminal.
        out["actions"] = [{"id": "browser", "label": "Sign in", "terminal": True},
                          {"id": "refresh", "label": "Look again"}]
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    for r in remotes():
        state = unit_state(r["name"])
        # An account is always mounted, so the only thing to say about one is that it is, or why it is not.
        if r["mounted"] and responsive(r["at"]):
            sub = "Mounted at " + r["at"].replace(os.path.expanduser("~"), "~")
            acts = [{"id": "open", "label": "Open"}]
        elif r["mounted"]:
            sub = "Not responding"
            acts = [{"id": "restart", "label": "Restart", "primary": True}]
        else:
            sub = {"activating": "Connecting", "failed": "Would not start",
                   "inactive": "Not mounted"}.get(state, "Connecting")
            acts = [{"id": "mount", "label": "Try again", "primary": True}]
        acts.append({"id": "forget", "label": "Remove", "danger": True})
        out["items"].append({"id": r["name"], "title": r["name"], "subtitle": sub, "actions": acts})
    print(json.dumps(out))

elif cmd == "action":
    what = args[1] if len(args) > 1 else ""
    # An action run in a terminal has a person's keyboard on stdin: its input arrives as an argument instead.
    data = args[2] if len(args) > 2 else ("" if sys.stdin.isatty() else sys.stdin.read().strip())
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
        p = run(RCLONE, "config", "create", name, backend or "webdav", *pairs, "--obscure", timeout=60)
        if p.returncode != 0:
            sys.stderr.write((p.stderr.strip().split("\n")[-1] or "rclone would not add it") + "\n")
            sys.exit(p.returncode)
        sys.exit(start(name))
    if what == "browser":
        # The consent page: rclone runs its own flow, which needs a terminal to say what it is doing.
        if not re.match(r"^[A-Za-z0-9_-]+$", name):
            sys.stderr.write("A name of letters, digits, dashes or underscores\n")
            sys.exit(2)
        p = subprocess.run([RCLONE, "config", "create", name, backend or "webdav"])
        if p.returncode != 0:
            sys.exit(p.returncode)
        sys.exit(start(name))
    if what == "refresh":
        sys.exit(0)
    if not name:
        sys.stderr.write("No account named\n")
        sys.exit(2)
    if what == "mount":
        sys.exit(start(name))
    if what == "restart":
        # A stalled mount does not come down on its own: the lazy unmount frees everything waiting on it.
        write_unit()
        run("fusermount3", "-uz", os.path.join(ROOT, name))
        p = run("systemctl", "--user", "restart", UNIT % name, timeout=90)
        if p.returncode != 0:
            sys.stderr.write((p.stderr.strip().split("\n")[-1] or "The mount would not restart") + "\n")
        sys.exit(p.returncode)
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
