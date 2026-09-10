#!/usr/bin/env python3
"""Fingerprints through fprintd, and where a finger may stand in for the password.

    fingerprint.py status                 {installed, device, fingers, login, sudo}
    fingerprint.py enroll <finger>        runs fprintd-enroll, one line per stage: "stage N", "done", or "retry <why>"
    fingerprint.py delete <finger>
    fingerprint.py set login|sudo on|off  edits /etc/pam.d/greetd or /etc/pam.d/sudo; the shell runs it through pkexec

The lock screen needs no PAM change: it verifies through fprintd itself.
"""
import json, os, re, shutil, subprocess, sys

FINGERS = ["left-thumb", "left-index-finger", "left-middle-finger", "left-ring-finger", "left-little-finger",
           "right-thumb", "right-index-finger", "right-middle-finger", "right-ring-finger", "right-little-finger"]
PAM = {"login": "/etc/pam.d/greetd", "sudo": "/etc/pam.d/sudo"}
# At login a short wait, so a typed password is not held long; sudo has no such wait since the prompt is the reader.
LINE = {"login": "auth       sufficient   pam_fprintd.so timeout=8 max-tries=1\n",
        "sudo": "auth\t\tsufficient\tpam_fprintd.so\n"}


def user():
    return os.environ.get("SUDO_USER") or os.environ.get("USER") or os.getlogin()


def has_line(kind):
    try:
        return "pam_fprintd" in open(PAM[kind], encoding="utf-8").read()
    except OSError:
        return False


def status():
    out = {"installed": shutil.which("fprintd-enroll") is not None, "device": "", "fingers": [],
           "login": has_line("login"), "sudo": has_line("sudo"), "fingerNames": FINGERS}
    if out["installed"]:
        try:
            r = subprocess.run(["fprintd-list", user()], capture_output=True, text=True, timeout=20)
            text = r.stdout + r.stderr
            m = re.search(r"on (.+?) \((?:press|swipe)\)", text) or re.search(r"enrolled for (.+?)\.", text) or re.search(r"Using device (\S+)", text)
            if m and "No devices available" not in text:
                out["device"] = m.group(1).strip()
            out["fingers"] = re.findall(r"- #\d+: (\S+)", text)
        except (OSError, subprocess.TimeoutExpired):
            pass
    print(json.dumps(out))


def enroll(finger):
    if finger not in FINGERS:
        print("retry unknown finger"); return 2
    stage = 0
    with subprocess.Popen(["fprintd-enroll", "-f", finger, user()], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1) as p:
        for line in p.stdout:
            line = line.strip()
            if "enroll-stage-passed" in line:
                stage += 1; print(f"stage {stage}", flush=True)
            elif "enroll-completed" in line:
                print("done", flush=True)
            elif "enroll-" in line:
                why = re.sub(r".*enroll-", "", line).replace(" (done)", "").replace("-", " ")
                print(f"retry {why}", flush=True)
            elif "failed" in line.lower() or "error" in line.lower():
                print(f"retry {line[:100]}", flush=True)
        return p.wait()


def delete(finger):
    r = subprocess.run(["fprintd-delete", user(), "-f", finger], capture_output=True, text=True, timeout=20)
    if r.returncode != 0:
        sys.stderr.write((r.stderr or r.stdout).strip() + "\n")
    return r.returncode


def set_pam(kind, on):
    path = PAM[kind]
    try:
        lines = open(path, encoding="utf-8").read().splitlines(keepends=True)
    except OSError as e:
        sys.stderr.write(f"{path}: {e}\n"); return 1
    lines = [l for l in lines if "pam_fprintd" not in l]
    if on:
        # Before the first auth line that consults the password, after any nologin/securetty gates.
        idx = next((i for i, l in enumerate(lines) if l.startswith("auth") and ("include" in l or "pam_unix" in l or "pam_systemd_home" in l)), None)
        if idx is None:
            idx = next((i + 1 for i, l in enumerate(lines) if l.startswith("#%PAM")), 0)
        lines.insert(idx, LINE[kind])
    tmp = path + ".isle"
    with open(tmp, "w", encoding="utf-8") as f:
        f.writelines(lines)
    os.chmod(tmp, 0o644)
    os.replace(tmp, path)
    return 0


def main(argv):
    if len(argv) < 2:
        print(__doc__); return 2
    cmd = argv[1]
    if cmd == "status":
        status(); return 0
    if cmd == "enroll" and len(argv) > 2:
        return enroll(argv[2])
    if cmd == "delete" and len(argv) > 2:
        return delete(argv[2])
    if cmd == "set" and len(argv) > 3 and argv[2] in PAM and argv[3] in ("on", "off"):
        if os.geteuid() != 0:
            sys.stderr.write("set needs root\n"); return 1
        return set_pam(argv[2], argv[3] == "on")
    print(__doc__); return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
