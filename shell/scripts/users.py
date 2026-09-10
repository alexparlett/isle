#!/usr/bin/env python3
"""User accounts through AccountsService. Anything beyond your own name and picture asks polkit.

    users.py list                                 [{path, name, realName, admin, icon, locked, autoLogin, uid, me}]
    users.py set-name <user> <real name>
    users.py set-icon <user> <path>
    users.py set-admin <user> 0|1
    users.py set-password <user> -                 the new password on stdin
    users.py add <user> <real name> 0|1
    users.py remove <user> 0|1                     1 removes the home directory too
"""
import json, os, subprocess, sys
import gi
from gi.repository import Gio, GLib

BUS = "org.freedesktop.Accounts"
bus = Gio.bus_get_sync(Gio.BusType.SYSTEM, None)

def call(path, iface, method, args=None):
    return bus.call_sync(BUS, path, iface, method, args, None, Gio.DBusCallFlags.NONE, -1, None).unpack()

def users():
    out = []
    for path in call("/org/freedesktop/Accounts", BUS, "ListCachedUsers")[0]:
        p = call(path, "org.freedesktop.DBus.Properties", "GetAll", GLib.Variant("(s)", (BUS + ".User",)))[0]
        out.append({"path": path, "name": p["UserName"], "realName": p["RealName"], "admin": p["AccountType"] == 1,
                    "icon": p["IconFile"] if os.path.isfile(p["IconFile"]) else "", "locked": p["Locked"], "autoLogin": p["AutomaticLogin"],
                    "uid": p["Uid"], "me": p["Uid"] == os.getuid()})
    return sorted(out, key=lambda u: (not u["me"], u["name"]))

def path_of(name):
    return call("/org/freedesktop/Accounts", BUS, "FindUserByName", GLib.Variant("(s)", (name,)))[0]

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
a = sys.argv[2:]
if cmd == "list":
    print(json.dumps(users()))
elif cmd == "set-name":
    call(path_of(a[0]), BUS + ".User", "SetRealName", GLib.Variant("(s)", (a[1],)))
elif cmd == "set-icon":
    call(path_of(a[0]), BUS + ".User", "SetIconFile", GLib.Variant("(s)", (a[1],)))
elif cmd == "set-admin":
    call(path_of(a[0]), BUS + ".User", "SetAccountType", GLib.Variant("(i)", (1 if a[1] == "1" else 0,)))
elif cmd == "set-password":
    password = sys.stdin.read().rstrip("\n")
    crypted = subprocess.run(["openssl", "passwd", "-6", "-stdin"], input=password, capture_output=True, text=True).stdout.strip()
    call(path_of(a[0]), BUS + ".User", "SetPassword", GLib.Variant("(ss)", (crypted, "")))
elif cmd == "add":
    call("/org/freedesktop/Accounts", BUS, "CreateUser", GLib.Variant("(ssi)", (a[0], a[1], 1 if a[2] == "1" else 0)))
elif cmd == "remove":
    uid = next(u["uid"] for u in users() if u["name"] == a[0])
    call("/org/freedesktop/Accounts", BUS, "DeleteUser", GLib.Variant("(xb)", (uid, a[1] == "1")))
