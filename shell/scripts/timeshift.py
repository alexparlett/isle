#!/usr/bin/env python3
"""Timeshift as a snapshots provider: everything it does needs root and it has a window of its own, so the
provider says so and opens it.

    timeshift.py status | list | action open
"""
import json, shutil, subprocess, sys

CLI = shutil.which("timeshift")
GUI = shutil.which("timeshift-launcher") or shutil.which("timeshift-gtk")
cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    print(json.dumps({"installed": bool(CLI), "ready": False, "state": "Snapshots and restores happen in Timeshift's window" if CLI else "timeshift is not on this machine",
                      "actions": [{"id": "open", "label": "Open Timeshift", "primary": True}] if CLI and GUI else [], "error": ""}))
elif cmd == "list":
    print(json.dumps({"items": [], "error": ""}))
elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    if what == "open" and GUI:
        subprocess.Popen([GUI], start_new_session=True)
    else:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
