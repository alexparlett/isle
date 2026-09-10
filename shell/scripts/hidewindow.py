#!/usr/bin/env python3
"""Minimise the active window: park it on the hidden special workspace and leave the desktop usable.

Moving the focused window there brings that workspace up over the desktop with the window still focused, so
the workspace is closed again and focus goes to the top window left on the current one.
"""
import json, subprocess, sys


def hyprctl(*args):
    return json.loads(subprocess.run(["hyprctl", "-j", *args], capture_output=True, text=True).stdout or "null")


def dispatch(lua):
    subprocess.run(["hyprctl", "dispatch", lua], capture_output=True)


if len(sys.argv) > 1:
    addr = sys.argv[1] if sys.argv[1].startswith("0x") else "0x" + sys.argv[1]
    win = next((c for c in hyprctl("clients") or [] if c.get("address") == addr), None)
else:
    win = hyprctl("activewindow")
if not win or not win.get("address"):
    raise SystemExit
dispatch('hl.dsp.window.move({ workspace = "special:hidden", follow = false, window = "address:%s" })' % win["address"])
mon = next((m for m in hyprctl("monitors") or [] if m.get("focused")), None)
if mon and (mon.get("specialWorkspace") or {}).get("name") == "special:hidden":
    dispatch('hl.dsp.workspace.toggle_special("hidden")')
ws = (hyprctl("activeworkspace") or {}).get("id")
rest = [c for c in hyprctl("clients") or [] if c.get("mapped") and (c.get("workspace") or {}).get("id") == ws]
rest.sort(key=lambda c: c.get("focusHistoryID", 99))
if rest:
    dispatch('hl.dsp.focus({ window = "address:%s" })' % rest[0]["address"])
    dispatch('hl.dsp.window.bring_to_top({ window = "address:%s" })' % rest[0]["address"])
