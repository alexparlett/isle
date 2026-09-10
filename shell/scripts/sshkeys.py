#!/usr/bin/env python3
"""SSH keys in ~/.ssh and the agent at SSH_AUTH_SOCK.

    sshkeys.py list                                   [{path, name, type, bits, fingerprint, comment, pubkey, locked, loaded}]
    sshkeys.py agent                                  {sock, alive, loaded}
    sshkeys.py load <private-key> [passphrase-file]   ssh-add, the passphrase read once through askpass.sh
    sshkeys.py unload <private-key>
    sshkeys.py generate <name> <type> <comment> [passphrase-file]
"""
import json, os, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
ssh_dir = os.path.expanduser("~/.ssh")
# A session started before the agent socket was exported still finds gcr's.
gcr_sock = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/run/user/" + str(os.getuid())), "gcr", "ssh")
if not os.environ.get("SSH_AUTH_SOCK") and os.path.exists(gcr_sock):
    os.environ["SSH_AUTH_SOCK"] = gcr_sock

def run(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)

def loaded_fingerprints():
    r = run(["ssh-add", "-l"])
    return {line.split()[1] for line in r.stdout.splitlines() if len(line.split()) > 1} if r.returncode == 0 else set()

def read_passphrase(path):
    try:
        with open(path) as f:
            return f.read().rstrip("\n")
    except OSError:
        return ""

def askpass_env(passfile):
    env = dict(os.environ)
    if passfile:
        env.update({"SSH_ASKPASS": os.path.join(here, "askpass.sh"), "SSH_ASKPASS_REQUIRE": "force", "ISLE_ASKPASS_FILE": passfile})
    else:
        env["SSH_ASKPASS_REQUIRE"] = "never"
    return env

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
arg = sys.argv[2:]

if cmd == "list":
    loaded = loaded_fingerprints()
    out = []
    if os.path.isdir(ssh_dir):
        for f in sorted(os.listdir(ssh_dir)):
            if not f.endswith(".pub"):
                continue
            pub = os.path.join(ssh_dir, f)
            priv = pub[:-4]
            info = run(["ssh-keygen", "-lf", pub]).stdout.split()
            if len(info) < 2:
                continue
            with open(pub) as fh:
                pubkey = fh.read().strip()
            has_private = os.path.isfile(priv)
            locked = has_private and run(["ssh-keygen", "-y", "-P", "", "-f", priv]).returncode != 0
            out.append({"path": priv, "name": os.path.basename(priv), "type": info[-1].strip("()"), "bits": int(info[0]),
                        "fingerprint": info[1], "comment": " ".join(info[2:-1]), "pubkey": pubkey,
                        "hasPrivate": has_private, "locked": locked, "loaded": info[1] in loaded})
    print(json.dumps(out))
elif cmd == "agent":
    sock = os.environ.get("SSH_AUTH_SOCK", "")
    r = run(["ssh-add", "-l"])
    print(json.dumps({"sock": sock, "alive": r.returncode in (0, 1), "loaded": len(loaded_fingerprints())}))
elif cmd == "load":
    r = run(["ssh-add", arg[0]], env=askpass_env(arg[1] if len(arg) > 1 else ""))
    if len(arg) > 1 and os.path.exists(arg[1]):
        os.remove(arg[1])
    sys.stderr.write(r.stderr)
    sys.exit(r.returncode)
elif cmd == "unload":
    r = run(["ssh-add", "-d", arg[0]])
    sys.stderr.write(r.stderr)
    sys.exit(r.returncode)
elif cmd == "generate":
    name, ktype, comment = arg[0], arg[1], arg[2]
    passphrase = read_passphrase(arg[3]) if len(arg) > 3 else ""
    if len(arg) > 3 and os.path.exists(arg[3]):
        os.remove(arg[3])
    os.makedirs(ssh_dir, mode=0o700, exist_ok=True)
    path = os.path.join(ssh_dir, name)
    if os.path.exists(path):
        sys.stderr.write("A key called " + name + " exists\n")
        sys.exit(1)
    args = ["ssh-keygen", "-q", "-t", ktype, "-C", comment, "-f", path, "-N", passphrase]
    if ktype == "rsa":
        args += ["-b", "4096"]
    r = run(args)
    sys.stderr.write(r.stderr)
    sys.exit(r.returncode)
