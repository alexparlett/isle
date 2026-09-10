#!/usr/bin/env python3
"""The keychain, over libsecret.

    secrets.py list                          every item, as JSON lines
    secrets.py reveal <item-path>            the secret, on stdout
    secrets.py delete <item-path>
    secrets.py store <label> <k=v>... -      the secret on stdin
    secrets.py lock | unlock
"""
import json, sys
import gi
gi.require_version("Secret", "1")
from gi.repository import Secret, GLib

service = Secret.Service.get_sync(Secret.ServiceFlags.OPEN_SESSION | Secret.ServiceFlags.LOAD_COLLECTIONS, None)

def items():
    for col in service.get_collections():
        for it in col.get_items():
            yield col, it

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"

if cmd == "list":
    for col, it in items():
        print(json.dumps({
            "path": it.get_object_path(), "label": it.get_label(), "collection": col.get_label(),
            "attributes": dict(it.get_attributes()), "locked": it.get_locked(),
            "created": it.get_created(), "modified": it.get_modified(),
        }))
elif cmd == "reveal":
    path = sys.argv[2]
    for col, it in items():
        if it.get_object_path() == path:
            if it.get_locked():
                service.unlock_sync([col], None)
            it.load_secret_sync(None)
            v = it.get_secret()
            print(v.get_text() if v else "", end="")
            break
elif cmd == "delete":
    path = sys.argv[2]
    for col, it in items():
        if it.get_object_path() == path:
            it.delete_sync(None)
            break
elif cmd == "store":
    label = sys.argv[2]
    attrs = dict(a.split("=", 1) for a in sys.argv[3:] if a != "-" and "=" in a)
    secret = sys.stdin.read().rstrip("\n")
    schema = Secret.Schema.new("org.isle.Keychain", Secret.SchemaFlags.DONT_MATCH_NAME, {k: Secret.SchemaAttributeType.STRING for k in attrs})
    Secret.password_store_sync(schema, attrs, Secret.COLLECTION_DEFAULT, label, secret, None)
elif cmd == "lock":
    service.lock_sync(service.get_collections(), None)
elif cmd == "unlock":
    service.unlock_sync(service.get_collections(), None)
