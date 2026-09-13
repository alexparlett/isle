"""What the service says a folder holds, streamed, so a folder fills as its names arrive.

The backend has no recursive listing (`ListR` is false), so the tree is walked level by level, and a folder
of several thousand files takes a minute and a half to describe. Its names are read as they come rather than
at the end, so what has arrived can be shown while the rest is still coming.
"""
import csv, io, subprocess, time
from datetime import datetime

BATCH = 200


def stream(remote, path, take, timeout=600):
    """Call `take` with batches of [{name, is_dir, size, mtime}]. True when the folder was described in full:
    a listing that failed part way must not be taken for the whole of one."""
    where = remote + ":" + path
    try:
        p = subprocess.Popen(["rclone", "lsf", where, "--format", "spt", "--csv", "--absolute"],
                             stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                             stdin=subprocess.DEVNULL, text=True)
    except OSError:
        return False
    end = time.time() + timeout
    batch = []
    try:
        for row in csv.reader(p.stdout):
            if time.time() > end:
                p.kill()
                return False
            if len(row) < 3:
                continue
            size, name, when_ = row[0], row[1], row[2]
            # --absolute writes a leading slash, and a folder a trailing one.
            name = name[1:] if name.startswith("/") else name
            is_dir = name.endswith("/")
            name = name[:-1] if is_dir else name
            if not name:
                continue
            batch.append({"name": name, "is_dir": is_dir,
                          "size": 0 if is_dir else max(0, int(size or 0)), "mtime": when(when_)})
            if len(batch) >= BATCH:
                take(batch)
                batch = []
    finally:
        if batch:
            take(batch)
        p.stdout.close()
    return p.wait() == 0


def when(text):
    """rclone's timestamp as seconds. A file whose time the service does not give is taken as now."""
    if not text:
        return time.time()
    for form in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(text.split(".")[0].replace("Z", ""), form).timestamp()
        except ValueError:
            pass
    return time.time()
