"""What a drive's folder is called, which is also what the file manager calls it in the sidebar (D65).

A GTK file manager names a mount after its mount point, so the folder is named for the service rather than
for the short name rclone knows it by. Two accounts of one service are told apart by that short name.
"""
import json, os, subprocess

ROOT = os.path.expanduser("~/Drives")
KNOWN = {"protondrive": "Proton Drive", "drive": "Google Drive", "dropbox": "Dropbox",
         "onedrive": "OneDrive", "iclouddrive": "iCloud Drive", "box": "Box", "s3": "S3", "webdav": "WebDAV"}


def config():
    try:
        out = subprocess.run(["rclone", "config", "dump"], capture_output=True, text=True,
                             stdin=subprocess.DEVNULL, timeout=20).stdout
        return json.loads(out or "{}")
    except (OSError, ValueError, subprocess.SubprocessError):
        return {}


def folder_for(remote, conf=None):
    """The name of this account's folder under ~/Drives."""
    conf = config() if conf is None else conf
    kind = conf.get(remote, {}).get("type", "")
    label = KNOWN.get(kind, remote)
    sharing = [r for r, c in conf.items() if c.get("type") == kind and r != remote]
    return "%s (%s)" % (label, remote) if sharing else label


def at(remote, conf=None):
    return os.path.join(ROOT, folder_for(remote, conf))
