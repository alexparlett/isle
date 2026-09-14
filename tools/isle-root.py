#!/usr/bin/env python3
"""Everything in an install that root has to do, in one go, so one authorisation covers all of it.

    tools/isle-root.py --due REPO USER          exit 0 when any of it is pending; needs no root
    tools/isle-root.py REPO USER [RUNTIME HIS]  do the parts that are pending

nethogs' capabilities, the portal file, the hidraw rule, the greeter copy, and the compositor
plugins. Each is skipped when it is already done. tools/install.sh runs the question as the user and
the work through sudo on a terminal or pkexec without one (D77).

The plugins are the awkward one: hyprpm refuses to run as root and escalates eight times with `sudo`
of its own. It is run here as the user with tools/pkexec-as-sudo ahead of the real sudo on its path,
and those eight requests come back to this process over a socket that lives only as long as the
rebuild. Each is matched against the shapes hyprpm uses and anything else is refused (D79).
"""
import errno, grp, json, os, pwd, re, shutil, socket, subprocess, sys, time

PORTAL = "/usr/share/xdg-desktop-portal/portals/isle.portal"
HIDRAW = "/etc/udev/rules.d/70-isle-hidraw.rules"
GREETD = "/etc/greetd/hyprland.lua"

STORE = re.compile(r"^/var/cache/hyprpm/[^/]+(/.*)?$")
BUILD = re.compile(r"^/run/user/\d+/hyprpm(/.*)?$")
# make -C '<build dir>' installheaders && chmod -R a+rX <build dir>
HEADERS = re.compile(r"^make -C '(?P<a>[^']+)' installheaders && chmod -R a\+rX (?P<b>\S+)$")


def ok(what):
    print("  \033[32m✓\033[0m %s" % what, flush=True)


def step(what):
    print("  \033[34m→\033[0m %s" % what, flush=True)


def warn(what):
    print("  ! %s" % what, file=sys.stderr, flush=True)


def same(a, b):
    try:
        with open(a, "rb") as x, open(b, "rb") as y:
            return x.read() == y.read()
    except OSError:
        return False


# --- what is pending -------------------------------------------------------------------------
def caps_due():
    n = shutil.which("nethogs")
    if not n:
        return False
    p = subprocess.run(["getcap", n], capture_output=True, text=True)
    return "cap_net_admin" not in p.stdout


def portal_due(repo):
    return not same(os.path.join(repo, "system/portal/isle.portal"), PORTAL)


def hidraw_due(repo):
    return not same(os.path.join(repo, "system/udev/70-isle-hidraw.rules"), HIDRAW)


# The greeter is a copy of shell/, which all but every update changes: due whenever greetd runs it.
def greeter_due():
    try:
        with open(GREETD) as f:
            return "isle-greeter" in f.read()
    except OSError:
        return False


def plugins_due(repo, user):
    p = subprocess.run([sys.executable, os.path.join(repo, "tools/update.py"), "plugins"],
                       capture_output=True, env=user_env(repo, user))
    return p.returncode == 0


def anything_due(repo, user):
    return (caps_due() or portal_due(repo) or hidraw_due(repo) or greeter_due()
            or plugins_due(repo, user))


# --- the escalations hyprpm makes -------------------------------------------------------------
def under(pattern, *paths):
    return all(pattern.match(os.path.normpath(p)) for p in paths)


def allowed(argv):
    if not argv:
        return False
    what, args = argv[0], argv[1:]
    if what == "echo":
        return True
    if what == "rm" and args[:1] == ["-fr"] and len(args) == 2:
        return under(STORE, args[1])
    if what == "mkdir" and args[:2] == ["-p", "-m"] and len(args) == 4:
        return under(STORE, args[3])
    if what == "install":
        rest = [a for a in args if not a.startswith("-") and a != "0"]
        return len(rest) == 2 and under(BUILD, rest[0]) and under(STORE, rest[1])
    if what in ("/bin/sh", "sh") and args[:1] == ["-c"] and len(args) == 2:
        m = HEADERS.match(args[1])
        return bool(m) and under(BUILD, m.group("a"), m.group("b"))
    return False


def serve(conn):
    with conn:
        conn.settimeout(30)
        data = b""
        while not data.endswith(b"\n"):
            chunk = conn.recv(65536)
            if not chunk:
                return
            data += chunk
        try:
            argv = json.loads(data.decode())["argv"]
        except (ValueError, KeyError, UnicodeDecodeError):
            conn.sendall(b'{"code": 1, "out": "isle-root: unreadable request"}\n')
            return
        if not allowed(argv):
            warn("refused as root: %s" % " ".join(argv))
            conn.sendall(json.dumps({"code": 1, "out": "isle-root: refused"}).encode() + b"\n")
            return
        p = subprocess.run(argv, capture_output=True, text=True)
        conn.sendall(json.dumps({"code": p.returncode, "out": p.stdout + p.stderr}).encode() + b"\n")


# --- running as the user again ----------------------------------------------------------------
def user_env(repo, user, runtime="", his=""):
    e = pwd.getpwnam(user)
    env = {"HOME": e.pw_dir, "USER": user, "LOGNAME": user, "SHELL": e.pw_shell or "/bin/sh",
           "PATH": os.path.join(repo, "tools/pkexec-as-sudo") + ":/usr/local/bin:/usr/bin:/bin",
           "XDG_RUNTIME_DIR": runtime or "/run/user/%d" % e.pw_uid,
           "XDG_CACHE_HOME": os.path.join(e.pw_dir, ".cache")}
    if his:
        env["HYPRLAND_INSTANCE_SIGNATURE"] = his
    return env


def as_user(user, env, argv):
    """Fork, become the user, and run argv. Returns the child's pid."""
    e = pwd.getpwnam(user)
    pid = os.fork()
    if pid:
        return pid
    try:
        os.setgroups([g.gr_gid for g in grp.getgrall() if user in g.gr_mem] + [e.pw_gid])
        os.setgid(e.pw_gid)
        os.setuid(e.pw_uid)
        os.chdir(e.pw_dir)
        os.execvpe(argv[0], argv, env)
    except Exception as exc:  # the child must never return into the caller's code
        print("  ! could not run as %s: %s" % (user, exc), file=sys.stderr, flush=True)
    os._exit(127)


# --- the work --------------------------------------------------------------------------------
def rebuild_plugins(repo, user, runtime, his):
    step("Rebuilding the compositor plugins")
    env = user_env(repo, user, runtime, his)
    sockdir = os.path.join(env["XDG_RUNTIME_DIR"], "isle")
    path = os.path.join(sockdir, "root.sock")
    e = pwd.getpwnam(user)
    os.makedirs(sockdir, exist_ok=True)
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    os.chown(path, e.pw_uid, e.pw_gid)
    os.chmod(path, 0o600)
    srv.listen(8)
    srv.settimeout(0.2)
    env["ISLE_ROOT_SOCK"] = path

    # hyprpm's exit code is the truth in neither direction: it exits 0 with a dependency missing and
    # non-zero after a build that worked, so its own lines decide.
    out = os.path.join(sockdir, "hyprpm.out")
    pid = as_user(user, env, ["sh", "-c", 'hyprpm update > "$1" 2>&1', "_", out])
    try:
        while True:
            try:
                conn, _ = srv.accept()
                serve(conn)
            except socket.timeout:
                pass
            if os.waitpid(pid, os.WNOHANG)[0]:
                break
    finally:
        srv.close()
        try:
            os.unlink(path)
        except OSError:
            pass

    said = ""
    try:
        with open(out, errors="replace") as f:
            said = f.read()
        os.unlink(out)
    except OSError:
        pass
    for line in said.replace("\r", "\n").splitlines():
        if line.strip() and "━" not in line:
            print(line, flush=True)
    if re.search(r"failed to build|Could not update|Failed to|failed to remove|refused", said):
        warn("the plugins did not rebuild; the compositor keeps the ones it has")
        return
    as_user(user, env, [sys.executable, os.path.join(repo, "tools/update.py"), "stamp"])
    os.wait()
    ok("compositor plugins rebuilt (a restart loads them)")


def main():
    argv = sys.argv[1:]
    due = argv[:1] == ["--due"]
    if due:
        argv = argv[1:]
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    repo, user = argv[0], argv[1]
    runtime = argv[2] if len(argv) > 2 else ""
    his = argv[3] if len(argv) > 3 else ""

    if due:
        return 0 if anything_due(repo, user) else 1
    if os.geteuid() != 0:
        print("  ! run as root: sudo %s %s %s" % (sys.argv[0], repo, user), file=sys.stderr)
        return 1

    # nethogs counts network traffic per process; it needs two capabilities rather than root.
    if caps_due():
        subprocess.run(["setcap", "cap_net_admin,cap_net_raw+ep", shutil.which("nethogs")], check=True)
        ok("nethogs can read the network (per-process traffic in Monitor)")
    # The shell answers org.freedesktop.impl.portal.FileChooser; xdg-desktop-portal reads who can from here (D67).
    if portal_due(repo):
        subprocess.run(["install", "-Dm644", os.path.join(repo, "system/portal/isle.portal"), PORTAL], check=True)
        ok("the file chooser is the shell's (isle.portal)")
    # Raw HID for the browser keyboard configurators (Keychron Launcher, VIA): the rule is root's.
    if hidraw_due(repo):
        subprocess.run(["install", "-Dm644", os.path.join(repo, "system/udev/70-isle-hidraw.rules"), HIDRAW], check=True)
        subprocess.run(["udevadm", "control", "--reload"], check=True)
        subprocess.run(["udevadm", "trigger", "--subsystem-match=hidraw", "--subsystem-match=usb"], check=True)
        ok("hidraw devices are the logged-in user's (keyboard configurators)")
    # The login screen is a root-owned copy; after a pull it is refreshed.
    if greeter_due():
        step("Refreshing the login screen")
        subprocess.run([os.path.join(repo, "tools/install-greeter.sh"), user], check=True)
        ok("greeter copy refreshed")
    if plugins_due(repo, user):
        rebuild_plugins(repo, user, runtime, his)
    return 0


if __name__ == "__main__":
    sys.exit(main())
