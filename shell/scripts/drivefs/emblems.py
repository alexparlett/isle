"""The state of a file as the file manager shows it (D65).

A GTK file manager draws whatever `metadata::emblems` says, which is a file attribute anyone may set, so the
badge needs no plugin: the daemon stamps what it knows. Dolphin reads none of this and is told by its own
overlay plugin instead, over the control socket.
"""
import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

BADGE = {"cloud": "emblem-remote", "cached": "emblem-default", "pinned": "emblem-favorite",
         "uploading": "emblem-synchronizing", "conflict": "emblem-important"}


def stamp(where, state):
    """The badge for one file. False when the attribute would not take, which is not worth retrying."""
    name = BADGE.get(state)
    if name is None:
        return False
    try:
        info = Gio.FileInfo()
        info.set_attribute_stringv("metadata::emblems", [name])
        Gio.File.new_for_path(where).set_attributes_from_info(info, Gio.FileQueryInfoFlags.NONE, None)
        return True
    except GLib.Error:
        return False
