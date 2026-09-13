#!/usr/bin/env python3
"""The drives, in the file manager's sidebar, under their own names (D65).

GVfs takes volume monitors of its own: a service on the session bus answering `org.gtk.Private.
RemoteVolumeMonitor`, named by a file in /usr/share/gvfs/remote-volume-monitors, and what it reports appears
among the devices of every GTK file manager. Without one a cloud drive is listed as whatever its mount point
is called, with the icon of a memory stick.

The interface is gvfs's own and unpublished; the copy beside this file was taken from the monitor gvfs ships.
"""
import json, os, socket, sys

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

BUS = "org.gtk.vfs.IsleVolumeMonitor"
PATH = "/org/gtk/Private/RemoteVolumeMonitor"
IFACE = "org.gtk.Private.RemoteVolumeMonitor"
RUNTIME = os.environ.get("XDG_RUNTIME_DIR", "/run/user/%d" % os.getuid())
SOCKETS = os.path.join(RUNTIME, "isle", "drives")
HERE = os.path.dirname(os.path.abspath(__file__))


def ask(name, **q):
    """One question of a drive's filesystem."""
    where = os.path.join(SOCKETS, name + ".sock")
    try:
        s = socket.socket(socket.AF_UNIX)
        s.settimeout(5)
        s.connect(where)
        s.sendall((json.dumps(q) + "\n").encode())
        said = json.loads(s.recv(1 << 16).decode().strip() or "{}")
        s.close()
        return said
    except (OSError, ValueError):
        return {}


def icon_of(names):
    return Gio.ThemedIcon.new_from_names(names).to_string()


class Monitor:
    def __init__(self):
        self.mounts = {}

    def drives(self):
        """The accounts whose filesystem is up, as the sidebar should show them."""
        found = {}
        try:
            names = sorted(n[:-5] for n in os.listdir(SOCKETS) if n.endswith(".sock"))
        except OSError:
            names = []
        for name in names:
            said = ask(name, op="status")
            at = said.get("at")
            if not at or not os.path.ismount(at):
                continue
            found[name] = {
                "id": "isle-" + name,
                "name": said.get("label") or name,
                "icon": icon_of(said.get("icon") or ["folder-cloud", "folder-remote", "folder"]),
                "symbolic": icon_of(said.get("icon_symbolic") or ["folder-cloud-symbolic", "folder-remote-symbolic"]),
                "uri": Gio.File.new_for_path(at).get_uri(),
            }
        return found

    def as_mount(self, m):
        """(ssssssbsassa{sv}): id, name, icon, symbolic icon, uuid, root, can unmount, volume, content, sort, rest.
        A drive is not ejectable: it is there for as long as the account is."""
        return GLib.Variant("(ssssssbsassa{sv})",
                            (m["id"], m["name"], m["icon"], m["symbolic"], "", m["uri"],
                             False, "", [], "isle." + m["id"], {}))

    def call(self, conn, sender, path, iface, method, params, invocation):
        if method == "IsSupported":
            invocation.return_value(GLib.Variant("(b)", (True,)))
        elif method == "List":
            self.mounts = self.drives()
            mounts = GLib.Variant("a(ssssssbsassa{sv})", [self.as_mount(m).unpack() for m in self.mounts.values()])
            invocation.return_value(GLib.Variant.new_tuple(
                GLib.Variant("a(ssssbbbbbbbbuasa{ss}sa{sv})", []),
                GLib.Variant("a(ssssssbbssa{ss}sa{sv})", []), mounts))
        elif method == "CancelOperation":
            invocation.return_value(GLib.Variant("(b)", (False,)))
        else:
            # Nothing here is mounted, ejected or started by the file manager: the shell owns that.
            invocation.return_error_literal(Gio.io_error_quark(), Gio.IOErrorEnum.NOT_SUPPORTED,
                                            "the shell looks after this drive")

    def watch(self, conn):
        """Mounts that came or went, told to whoever is listening."""
        now = self.drives()
        for name, m in now.items():
            if name not in self.mounts:
                conn.emit_signal(None, PATH, IFACE, "MountAdded", GLib.Variant.new_tuple(self.as_mount(m)))
        for name, m in self.mounts.items():
            if name not in now:
                conn.emit_signal(None, PATH, IFACE, "MountRemoved", GLib.Variant.new_tuple(self.as_mount(m)))
        self.mounts = now
        return True


def main():
    xml = open(os.path.join(HERE, "drivefs", "remotevolumemonitor.xml")).read()
    node = Gio.DBusNodeInfo.new_for_xml(xml)
    mon = Monitor()
    conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    conn.register_object(PATH, node.interfaces[0], mon.call, None, None)
    Gio.bus_own_name_on_connection(conn, BUS, Gio.BusNameOwnerFlags.NONE, None, None)
    GLib.timeout_add_seconds(3, mon.watch, conn)
    GLib.MainLoop().run()
    return 0


sys.exit(main())
