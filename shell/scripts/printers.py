#!/usr/bin/env python3
"""Printers and jobs from CUPS, as JSON: {available, printers: [{name, state, reason, default, uri}], jobs: [{id, printer, user, size, time, title}]}"""
import json, re, subprocess

def run(args):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=3).stdout
    except Exception:
        return ""

out = {"available": False, "printers": [], "jobs": []}
if "scheduler is running" in run(["lpstat", "-r"]):
    out["available"] = True
    default = run(["lpstat", "-d"]).strip().split(":")[-1].strip() if ":" in run(["lpstat", "-d"]) else ""
    uris = {}
    for line in run(["lpstat", "-v"]).splitlines():
        m = re.match(r"device for (\S+): (.*)", line)
        if m:
            uris[m.group(1)] = m.group(2)
    for line in run(["lpstat", "-p"]).splitlines():
        m = re.match(r"printer (\S+) is (\w+)\.?\s*(.*)", line)
        if not m:
            continue
        name, state, rest = m.groups()
        reason = ""
        rm = re.search(r"-\s*(.*)$", rest)
        out["printers"].append({"name": name, "state": state, "reason": rm.group(1) if rm else "", "default": name == default, "uri": uris.get(name, "")})
    for line in run(["lpstat", "-o"]).splitlines():
        f = line.split(None, 4)
        if len(f) < 4:
            continue
        printer, _, jid = f[0].rpartition("-")
        out["jobs"].append({"id": jid, "printer": printer, "user": f[1], "size": int(f[2]) if f[2].isdigit() else 0, "time": f[3] + (" " + f[4] if len(f) > 4 else ""), "title": ""})
print(json.dumps(out))
