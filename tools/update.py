#!/usr/bin/env python3
"""Isle's own update: the look that changes nothing, and the run that pulls, installs and rebuilds.

    tools/update.py check    what origin has that this copy does not, as JSON on stdout
    tools/update.py run      pull, then install; progress written to the state file throughout
    tools/update.py state    print the state file
    tools/update.py plugins  exit 0 when the built compositor plugins are older than their sources
    tools/update.py stamp    record that they have just been rebuilt

`run` must be started detached from the shell (D78).
"""
import hashlib, json, os, pwd, re, shutil, subprocess, sys, time

REPO = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
RUNTIME = os.environ.get("XDG_RUNTIME_DIR") or "/tmp"
CACHE = os.environ.get("XDG_CACHE_HOME") or os.path.expanduser("~/.cache")
USER = os.environ.get("USER") or pwd.getpwuid(os.getuid()).pw_name
PLUGIN_STORE = "/var/cache/hyprpm/" + USER
PLUGIN_STAMP = os.path.join(CACHE, "isle", "plugins-built")
STATE = os.path.join(RUNTIME, "isle", "update.json")
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
GIT_ENV = dict(os.environ, GIT_TERMINAL_PROMPT="0",
               GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=15")


def git(*args, timeout=180):
    p = subprocess.run(["git"] + list(args), cwd=REPO, env=GIT_ENV,
                       capture_output=True, text=True, timeout=timeout)
    return p.returncode, p.stdout.strip(), p.stderr.strip()


# An SSH remote this machine has no key for still serves reads over https; the same repository, spelled
# the other way, so a fetch works where the push it was cloned for would not.
def public_url(remote):
    if remote.startswith("http"):
        return remote
    m = re.match(r"^(?:ssh://)?(?:[\w.-]+@)?([\w.-]+)[:/]+(.+?)(?:\.git)?/?$", remote)
    return "https://%s/%s.git" % (m.group(1), m.group(2)) if m else remote


def head():
    code, out, _ = git("rev-parse", "--short", "HEAD")
    if code != 0:
        return None
    d = {"head": out}
    d["headDate"] = git("log", "-1", "--format=%cs")[1]
    d["remote"] = git("remote", "get-url", "origin")[1]
    d["branch"] = git("rev-parse", "--abbrev-ref", "HEAD")[1]
    d["publicUrl"] = public_url(d["remote"])
    return d


# origin first, then its https spelling. Returns the one that served it, and the reason when neither did.
def fetch(branch, url):
    last = ""
    for via, where in (("origin", "origin"), ("https", url)):
        code, _, err = git("fetch", "-q", where, branch)
        if code == 0:
            return via, ""
        last = err
    lines = last.splitlines()
    reason = next((l for l in lines if re.search(r"Permission denied|fatal:|error:", l)), "")
    return "", reason or (lines[0] if lines else "fetch failed")


# What a built plugin was built from: its sources, and the Hyprland whose headers it was built against.
def plugin_hash():
    h = hashlib.sha256()
    root = os.path.join(REPO, "plugins")
    for dirpath, dirnames, names in sorted(os.walk(root)):
        dirnames.sort()
        for n in sorted(names):
            if n.endswith((".so", ".o")):
                continue
            f = os.path.join(dirpath, n)
            h.update(os.path.relpath(f, root).encode())
            try:
                with open(f, "rb") as fh:
                    h.update(fh.read())
            except OSError:
                pass
    p = subprocess.run(["pacman", "-Q", "hyprland"], capture_output=True, text=True)
    return h.hexdigest() + " " + (p.stdout.strip() if p.returncode == 0 else "none")


# Only what bootstrap registered is ever rebuilt: a repository hyprpm holds is a directory of its own in
# the store, beside the headers it built against.
def plugins_registered():
    try:
        return any(d != "headersRoot" and os.path.isdir(os.path.join(PLUGIN_STORE, d))
                   for d in os.listdir(PLUGIN_STORE))
    except OSError:
        return False


def plugins_stale():
    if not shutil.which("hyprpm") or not plugins_registered():
        return False
    try:
        with open(PLUGIN_STAMP) as f:
            return f.read().strip() != plugin_hash()
    except OSError:
        return True


def check():
    d = head()
    if d is None:
        print(json.dumps({"error": "this copy has no git history, so it cannot pull"}))
        return 1
    via, err = fetch(d["branch"], d["publicUrl"])
    d["viaHttps"] = via == "https"
    d["error"] = err
    d["incoming"] = [] if err else [l for l in git("log", "--format=%s", "HEAD..FETCH_HEAD")[1].splitlines() if l]
    d["checkedAt"] = time.strftime("%H:%M")
    d["pluginsStale"] = plugins_stale()
    print(json.dumps(d))
    return 0


class State:
    """The run as it stands, rewritten whole on every change so a reader never sees half of one."""

    def __init__(self, steps):
        self.d = {"state": "running", "phase": "Starting", "failed": "", "log": [],
                  "steps": [{"name": n, "state": "waiting"} for n in steps],
                  "from": "", "to": "", "startedAt": int(time.time()), "finishedAt": 0,
                  "pid": os.getpid()}
        # The last line that read like something going wrong. Only a non-zero exit makes it the reason:
        # install.sh says "! xremap not installed" about a machine it is perfectly happy to finish on.
        self.reason = ""
        self.write()

    def write(self):
        os.makedirs(os.path.dirname(STATE), exist_ok=True)
        tmp = STATE + ".new"
        with open(tmp, "w") as f:
            json.dump(self.d, f)
        os.replace(tmp, STATE)

    def step(self, name, state):
        for s in self.d["steps"]:
            if s["name"] == name:
                s["state"] = state
        self.write()

    def say(self, phase):
        self.d["phase"] = phase
        self.write()

    def took(self, line):
        l = ANSI.sub("", line).rstrip()
        if not l:
            return
        self.d["log"] = (self.d["log"] + [l])[-200:]
        # install.sh says what it is about to do with →, and what it did with ✓.
        if l.lstrip().startswith("→"):
            self.d["phase"] = l.lstrip()[1:].strip()
        elif re.match(r"^(fatal|error):", l, re.I):
            self.reason = re.sub(r"^(fatal|error):\s*", "", l, flags=re.I)
        elif l.lstrip().startswith(("!", "✖")):
            self.reason = l.lstrip()[1:].strip()
        self.write()

    def finish(self, failed=""):
        self.d["failed"] = failed
        self.d["state"] = "failed" if failed else "done"
        self.d["phase"] = "Stopped" if self.d["failed"] else "Done"
        self.d["finishedAt"] = int(time.time())
        for s in self.d["steps"]:
            if s["state"] in ("waiting", "running"):
                s["state"] = "stopped" if self.d["failed"] else "done"
        self.write()


def stream(st, cmd):
    p = subprocess.Popen(cmd, cwd=REPO, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, text=True, bufsize=1)
    for line in p.stdout:
        st.took(line)
    return p.wait()


# A run already going is left alone: its pid is in the state file and the process is still there.
def already_running():
    try:
        with open(STATE) as f:
            d = json.load(f)
    except (OSError, ValueError):
        return False
    if d.get("state") != "running":
        return False
    try:
        os.kill(d.get("pid", 0), 0)
        return True
    except OSError:
        return False


def run():
    if already_running():
        return 1
    d = head()
    st = State(["Pull", "Install"])
    if d is None:
        st.finish("this copy has no git history, so it cannot pull")
        return 1
    st.d["from"] = d["head"]

    st.step("Pull", "running")
    st.say("Pulling")
    via, err = fetch(d["branch"], d["publicUrl"])
    if not via:
        st.step("Pull", "stopped")
        st.finish(err)
        return 1
    code = stream(st, ["git", "pull", "--ff-only",
                       d["publicUrl"] if via == "https" else "origin", d["branch"]])
    if code != 0:
        st.step("Pull", "stopped")
        st.finish(st.reason or "the pull stopped with code %d" % code)
        return 1
    st.step("Pull", "done")
    st.d["to"] = git("rev-parse", "--short", "HEAD")[1]

    # From here the files under the shell have already changed and it has reloaded; this process is not
    # its child and carries on.
    st.step("Install", "running")
    st.say("Installing")
    code = stream(st, [os.path.join(REPO, "tools/install.sh")])
    if code != 0:
        st.step("Install", "stopped")
        # pkexec's own codes: the dialog was dismissed, or there was no agent to put it up.
        st.finish("Permission was not given" if code in (126, 127)
                  else st.reason or "the install stopped with code %d" % code)
        return 1
    st.step("Install", "done")
    st.finish()
    return 0


def main():
    what = sys.argv[1] if len(sys.argv) > 1 else "check"
    if what == "check":
        return check()
    if what == "run":
        return run()
    if what == "plugins":
        return 0 if plugins_stale() else 1
    if what == "stamp":
        os.makedirs(os.path.dirname(PLUGIN_STAMP), exist_ok=True)
        with open(PLUGIN_STAMP, "w") as f:
            f.write(plugin_hash())
        return 0
    if what == "state":
        try:
            with open(STATE) as f:
                sys.stdout.write(f.read())
        except OSError:
            print("{}")
        return 0
    print(__doc__.strip(), file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
