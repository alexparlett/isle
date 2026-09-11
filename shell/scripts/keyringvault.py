#!/usr/bin/env python3
"""The system keyring (the freedesktop secret service, over libsecret) as a vault provider, so the launcher
searches it beside the password managers. The Keychain window edits the keyring itself; this only reads.

    keyringvault.py status | list | field <collection-path> <item-path> <field> | action lock|unlock
"""
import json, re, sys

cmd = sys.argv[1] if len(sys.argv) > 1 else "status"
try:
    import gi
    gi.require_version("Secret", "1")
    from gi.repository import Secret
    service = Secret.Service.get_sync(Secret.ServiceFlags.OPEN_SESSION | Secret.ServiceFlags.LOAD_COLLECTIONS, None)
except Exception as e:  # no secret service: the provider is not installed
    service = None
    if cmd == "status":
        print(json.dumps({"installed": False, "ready": False, "state": "No secret service", "actions": [], "error": ""}))
    elif cmd == "list":
        print(json.dumps({"items": [], "error": "No secret service"}))
    sys.exit(0 if cmd in ("status", "list") else 1)


def category(attrs):
    schema = attrs.get("xdg:schema", "")
    if attrs.get("category"):
        return attrs["category"]
    if "NetworkManager" in schema or attrs.get("connection.type") == "802-11-wireless":
        return "wifi"
    if re.search(r"chrom|vivaldi|brave|mozilla|firefox|libsecret_os_crypt", schema + " " + attrs.get("application", ""), re.I):
        return "browser"
    if "ssh" in schema or attrs.get("unique", "").startswith("ssh"):
        return "ssh"
    if schema == "org.gnome.keyring.NetworkPassword" or attrs.get("user") or attrs.get("username"):
        return "logins"
    return "apps" if schema else "other"


TYPES = {"wifi": "wifi", "ssh": "ssh_key", "logins": "login", "browser": "login", "apps": "custom", "other": "note"}
collections = service.get_collections()
locked = [c for c in collections if c.get_locked()]

if cmd == "status":
    if locked and len(locked) == len(collections):
        print(json.dumps({"installed": True, "ready": False, "state": "Locked", "actions": [{"id": "unlock", "label": "Unlock", "primary": True}], "error": ""}))
    else:
        n = sum(len(c.get_items()) for c in collections if not c.get_locked())
        print(json.dumps({"installed": True, "ready": True, "state": "%d item%s" % (n, "" if n == 1 else "s") + ("  ·  a collection is locked" if locked else ""), "actions": [{"id": "lock", "label": "Lock"}], "error": ""}))

elif cmd == "list":
    out = []
    for c in collections:
        if c.get_locked():
            continue
        for it in c.get_items():
            attrs = dict(it.get_attributes())
            out.append({"id": it.get_object_path(), "shareId": c.get_object_path(), "vault": c.get_label() or "Keyring", "title": it.get_label() or "(unnamed)", "type": TYPES.get(category(attrs), "custom")})
    out.sort(key=lambda i: i["title"].lower())
    print(json.dumps({"items": out, "error": ""}))

elif cmd == "field":
    col, path, field = sys.argv[2:5]
    for c in collections:
        for it in c.get_items():
            if it.get_object_path() != path:
                continue
            attrs = dict(it.get_attributes())
            if field in ("username", "email"):
                sys.stdout.write(attrs.get("user") or attrs.get("username") or attrs.get("email") or "")
            elif field == "urls":
                sys.stdout.write(attrs.get("server") or attrs.get("origin_url") or attrs.get("url") or "")
            elif field in ("password", "note", "totp"):
                if it.get_locked():
                    service.unlock_sync([c], None)
                it.load_secret_sync(None)
                v = it.get_secret()
                sys.stdout.write(v.get_text() if v else "")
            sys.exit(0)
    sys.stderr.write("No such item\n")
    sys.exit(1)

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    if what == "unlock":
        service.unlock_sync(collections, None)
    elif what == "lock":
        service.lock_sync(collections, None)
    else:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
