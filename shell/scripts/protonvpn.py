#!/usr/bin/env python3
"""Proton VPN through its CLI, as a VPN provider (the contract: docs/ARCHITECTURE.md, "Providers").

    protonvpn.py status    {installed, ready, state, active: {name, detail} | null, actions, error}
    protonvpn.py list      {choices: [{id, title, subtitle}], error}   the countries; "" is the fastest server
    protonvpn.py action <id>   signin (terminal), signout, connect (a country code on stdin, or nothing),
                               disconnect, killswitch (toggles), netshield (a value on stdin)
"""
import json, os, re, shutil, subprocess, sys

CLI = shutil.which("protonvpn")
ENV = dict(os.environ, PYTHONWARNINGS="ignore")


def run(*args, stdin=None, timeout=90):
    try:
        p = subprocess.run([CLI, *args], capture_output=True, text=True, input=stdin, timeout=timeout, env=ENV)
    except subprocess.TimeoutExpired:
        return 124, "", "Proton VPN did not answer"
    lines = [l for l in p.stderr.strip().split("\n") if l and not re.search(r"eventlet|deprecat|migration|monkey|framework", l, re.I)]
    err = (lines[-1] if lines else "").replace("Error: ", "")
    return p.returncode, p.stdout, err


cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI), "ready": False, "state": "", "active": None, "actions": [], "error": ""}
    if not CLI:
        out["state"] = "protonvpn is not on this machine"
    else:
        code, so, se = run("info", timeout=30)
        m = re.search(r"Account: '(.*)'", so)
        signed = code == 0 and m is not None and m.group(1) not in ("", "None")
        if not signed:
            out.update(state="Not signed in", actions=[{"id": "signin", "label": "Sign in", "terminal": True, "primary": True}], error=se if code not in (0, None) else "")
        else:
            out["ready"] = True
            code, so, se = run("status", timeout=30)
            st = dict(re.findall(r"^(\w+): (.*)$", so, re.M))
            if st.get("Status") == "Connected":
                server = st.get("Server", "")
                at = server.find(" in ")
                name, where = (server[:at], server[at + 4:]) if at >= 0 else (server, "")
                detail = "  ·  ".join(x for x in [where, st.get("Protocol", ""), ("load " + st["Load"]) if st.get("Load") else ""] if x)
                out["active"] = {"name": name, "detail": detail}
                out["state"] = "Connected to " + name
                actions = [{"id": "disconnect", "label": "Disconnect", "primary": True}]
            else:
                out["state"] = "Signed in as " + m.group(1)
                actions = [{"id": "connect", "label": "Fastest server", "primary": True}]
            code, so, se = run("config", "list", timeout=30)
            cfg = dict(re.findall(r"^([a-z0-9-]+)\s{2,}(.+?)\s*$", so, re.M))
            if "kill-switch" in cfg:
                actions.append({"id": "killswitch", "label": "Kill switch", "on": cfg["kill-switch"] == "standard"})
            if "netshield" in cfg:
                actions.append({"id": "netshield", "label": "NetShield", "options": [["off", "Off"], ["malware-only", "Malware"], ["malware-ads-trackers", "Malware, ads and trackers"]], "value": cfg["netshield"]})
            actions.append({"id": "signout", "label": "Sign out"})
            out["actions"] = actions
    print(json.dumps(out))

elif cmd == "list":
    out = {"choices": [], "error": ""}
    if CLI:
        code, so, se = run("countries", "list", timeout=60)
        for line in so.split("\n"):
            m = re.match(r"^(.+?)\s{2,}([A-Z]{2})\s*$", line)
            if m and m.group(1) != "Country":
                out["choices"].append({"id": m.group(2), "title": m.group(1).strip(), "subtitle": m.group(2)})
        if code != 0:
            out["error"] = se
    print(json.dumps(out))

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    data = sys.stdin.read().strip() if what in ("connect", "netshield") else ""
    if what == "signin":
        sys.exit(subprocess.call([CLI, "signin"], env=ENV))
    args = {"signout": ["signout"], "connect": ["connect", "--country", data] if data else ["connect"], "disconnect": ["disconnect"],
            "netshield": ["config", "set", "netshield", data]}.get(what)
    if what == "killswitch":
        code, so, se = run("config", "list", timeout=30)
        cfg = dict(re.findall(r"^([a-z0-9-]+)\s{2,}(.+?)\s*$", so, re.M))
        args = ["config", "set", "kill-switch", "off" if cfg.get("kill-switch") == "standard" else "standard"]
    if not args:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
    code, so, se = run(*args)
    if code != 0:
        sys.stderr.write((se or "Proton VPN failed") + "\n")
    sys.exit(code)
