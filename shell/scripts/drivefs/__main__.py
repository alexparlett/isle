"""The drive's filesystem, and the private mount it reads through (D65).

    python3 drivefs <remote> [--at ~/Drives/<name>]

Two mounts: rclone's own, under the runtime directory where no file manager looks, and ours over it at the
place a person opens. Ours answers every listing from the index and passes reads down.
"""
import argparse, json, os, subprocess, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import pyfuse3
import trio

import cache as ca
import emblems as em
import index as idx
import naming
import remote as rm
from fs import Drive

RUNTIME = os.environ.get("XDG_RUNTIME_DIR", "/run/user/%d" % os.getuid())
# What the sidebar calls a drive, and what it draws beside it, by the service behind it.
SERVICES = {"protondrive": ("Proton Drive", ["proton-drive", "folder-cloud", "folder-remote"]),
            "drive": ("Google Drive", ["google-drive", "folder-cloud", "folder-remote"]),
            "dropbox": ("Dropbox", ["dropbox", "folder-cloud", "folder-remote"]),
            "onedrive": ("OneDrive", ["onedrive", "folder-cloud", "folder-remote"]),
            "iclouddrive": ("iCloud Drive", ["icloud", "folder-cloud", "folder-remote"])}


def service_of(remote):
    """The name a person knows the service by, from rclone's own configuration."""
    try:
        out = subprocess.run(["rclone", "config", "dump"], capture_output=True, text=True,
                             stdin=subprocess.DEVNULL, timeout=20).stdout
        kind = json.loads(out or "{}").get(remote, {}).get("type", "")
    except (OSError, ValueError, subprocess.SubprocessError):
        kind = ""
    return SERVICES.get(kind, (remote, ["folder-cloud", "folder-remote", "folder"]))
STATE = os.path.expanduser("~/.local/state/isle/drives")


def private_mount(name):
    """rclone's mount, which holds the cache and does the transport."""
    at = os.path.join(RUNTIME, "isle", "drives", name, "data")
    os.makedirs(at, exist_ok=True)
    p = subprocess.Popen(["rclone", "mount", name + ":", at,
                          "--vfs-cache-mode", "full", "--vfs-cache-max-size", os.environ.get("ISLE_DRIVE_BUDGET", "8G"),
                          "--vfs-cache-max-age", "168h", "--vfs-read-chunk-size", "1M",
                          "--vfs-read-chunk-size-limit", "128M", "--vfs-fast-fingerprint",
                          "--dir-cache-time", "1h", "--timeout", "30s", "--contimeout", "15s",
                          "--umask", "077"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(120):
        if os.path.ismount(at):
            return p, at
        if p.poll() is not None:
            return None, at
        time.sleep(0.25)
    return None, at


async def crawl(drive, ix):
    """The tree, walked once in the background so the first visit to a folder is not a wait. Breadth first,
    so the folders nearest the top are ready first."""
    queue = [idx.ROOT]
    while queue:
        ino = queue.pop(0)
        if ix.listed_at(ino) == 0:
            await drive.refresh(ino)
        for row in ix.children(ino):
            if row["is_dir"]:
                queue.append(row["ino"])
        await trio.sleep(0)


async def keep(ix, cache, at):
    """A pinned file is fetched whole when it is not here, and read from time to time when it is, which keeps
    it where rclone evicts from last. Pins are looked for often, since a person waits on one; the reading is
    rare, since nothing waits on that."""
    touched = 0
    while True:
        for row in ix.pinned_files():
            path = ix.path(row["ino"])
            if path is None:
                continue
            if not cache.whole(path, row["size"]):
                await trio.to_thread.run_sync(ca.warm, os.path.join(at, path), True, abandon_on_cancel=True)
            elif time.time() - touched > 300:
                await trio.to_thread.run_sync(ca.warm, os.path.join(at, path), False, abandon_on_cancel=True)
            await trio.sleep(0)
        if time.time() - touched > 300:
            touched = time.time()
        await trio.sleep(15)


async def paint(ix, cache, at, drive):
    """The badges of a folder someone has just looked at. Only folders that were opened, so the work is what
    a person can see rather than the whole drive."""
    while True:
        while drive.seen:
            ino = drive.seen.pop()
            for row in ix.children(ino):
                path = ix.path(row["ino"])
                if path is None:
                    continue
                state = "pinned" if row["pinned"] else (
                    row["state"] if row["state"] in ("uploading", "conflict")
                    else ("folder" if row["is_dir"] else cache.state(path, row["size"], False)))
                if state == "folder" or state == row["emblem"]:
                    continue
                if await trio.to_thread.run_sync(em.stamp, os.path.join(at, path), state,
                                                 abandon_on_cancel=True):
                    ix.wears(row["ino"], state)
                await trio.sleep(0)
        await trio.sleep(2)


async def answer(stream, ix, cache, at, drive):
    """One question from the shell or a file manager's menu, as a line of JSON."""
    try:
        line = await stream.receive_some(4096)
        if not line:
            return
        ask = json.loads(line.decode())
        op, path = ask.get("op", ""), ask.get("path", "")
        row = ix.by_path(path) if path else None
        if op == "state":
            # A folder says whether it is kept, not whether it is here: counting what is under it is work
            # the badge on every visible row cannot afford.
            if row is None:
                said = {"state": "unknown"}
            elif row["is_dir"]:
                said = {"state": "pinned" if row["pinned"] else "folder"}
            else:
                said = {"state": cache.state(path, row["size"], row["pinned"])}
        elif op in ("pin", "unpin") and row is not None:
            ix.set_pinned(row["ino"], op == "pin")
            said = {"ok": True}
        elif op == "evict" and row is not None:
            # Only what nothing is holding and nothing has still to send: the rest keeps its copy.
            freed, kept = 0, 0
            for node in ([row] if not row["is_dir"] else ix.under(row["ino"])):
                if node["is_dir"]:
                    continue
                where = ix.path(node["ino"])
                if node["pinned"] or node["state"] == "uploading" or drive.holding(node["ino"]):
                    kept += 1
                    continue
                freed += cache.drop(where)
            said = {"freed": freed, "kept": kept}
        elif op == "status":
            files, described = ix.counts()
            held, count = cache.usage()
            label, icon = drive.service
            said = {"files": files, "described": described, "cached_bytes": held, "cached_files": count,
                    "pinned": len(ix.pinned_files()), "at": at, "label": label, "icon": icon}
        else:
            said = {"error": "no such question"}
        await stream.send_all((json.dumps(said) + "\n").encode())
    except (OSError, ValueError, trio.BrokenResourceError):
        pass


async def listen(ix, cache, at, drive):
    where = os.path.join(RUNTIME, "isle", "drives")
    os.makedirs(where, exist_ok=True)
    sock = os.path.join(where, os.path.basename(at) + ".sock")
    if os.path.exists(sock):
        os.unlink(sock)
    s = trio.socket.socket(trio.socket.AF_UNIX, trio.socket.SOCK_STREAM)
    await s.bind(sock)
    s.listen(16)
    os.chmod(sock, 0o600)
    listener = trio.SocketListener(s)
    await trio.serve_listeners(lambda st: answer(st, ix, cache, at, drive), [listener])


async def run(drive, ix, cache, at):
    async with trio.open_nursery() as nursery:
        drive.nursery = nursery
        nursery.start_soon(crawl, drive, ix)
        nursery.start_soon(keep, ix, cache, at)
        nursery.start_soon(listen, ix, cache, at, drive)
        nursery.start_soon(paint, ix, cache, at, drive)
        await pyfuse3.main()
        nursery.cancel_scope.cancel()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("remote")
    ap.add_argument("--at", default="")
    a = ap.parse_args()
    at = a.at or naming.at(a.remote)
    try:
        os.makedirs(at, exist_ok=True)
    except OSError:
        # What a filesystem that went away left behind answers to nothing until it is let go of.
        subprocess.run(["fusermount3", "-uz", at], capture_output=True)
        os.makedirs(at, exist_ok=True)

    child, under = private_mount(a.remote)
    if child is None:
        sys.stderr.write("the drive's own mount would not start\n")
        return 1
    ix = idx.Index(os.path.join(STATE, a.remote, "index.db"))
    cache = ca.Cache(a.remote)
    drive = Drive(ix, under, a.remote, rm.stream)
    drive.service = service_of(a.remote)
    opts = set(pyfuse3.default_options)
    opts.add("fsname=isle-" + a.remote)
    pyfuse3.init(drive, at, opts)
    try:
        trio.run(run, drive, ix, cache, at)
    except KeyboardInterrupt:
        pass
    finally:
        pyfuse3.close(unmount=True)
        child.terminate()
        subprocess.run(["fusermount3", "-uz", under], capture_output=True)
    return 0


sys.exit(main())
