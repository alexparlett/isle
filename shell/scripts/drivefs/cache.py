"""What of the drive is on this disk, and what is kept there (D65).

rclone's cache mirrors the remote under one directory, a file per object, sparse where only part of it has
been read, so what a drive costs locally is the blocks those files occupy rather than what they say they
weigh. rclone evicts by size on its own; pinning is ours: a pinned file is fetched whole and read from time
to time, which keeps it at the top of the order rclone evicts from.
"""
import os


class Cache:
    def __init__(self, remote):
        home = os.environ.get("XDG_CACHE_HOME", os.path.expanduser("~/.cache"))
        self.data = os.path.join(home, "rclone", "vfs", remote)
        self.meta = os.path.join(home, "rclone", "vfsMeta", remote)

    def at(self, path):
        return os.path.join(self.data, path)

    def on_disk(self, path):
        """Blocks the cached copy occupies, which for a part-read file is less than the file's size."""
        try:
            return os.stat(self.at(path)).st_blocks * 512
        except OSError:
            return 0

    def whole(self, path, size):
        """Whether the whole of it is here: a sparse copy holds fewer blocks than the file needs, and a file
        of no bytes needs none."""
        return size == 0 or self.on_disk(path) >= size

    def usage(self):
        """(bytes, files) the cache occupies."""
        total, count = 0, 0
        for root, _, names in os.walk(self.data):
            for n in names:
                try:
                    total += os.stat(os.path.join(root, n)).st_blocks * 512
                    count += 1
                except OSError:
                    pass
        return total, count

    def state(self, path, size, pinned):
        if pinned:
            return "pinned"
        return "cached" if self.whole(path, size) else "cloud"


    def drop(self, path):
        """The cached copy, gone. rclone fetches it again when something asks for it."""
        freed = self.on_disk(path)
        for where in (self.at(path), os.path.join(self.meta, path)):
            try:
                os.remove(where)
            except OSError:
                pass
        return freed


def warm(where, whole=False):
    """Read a file through the drive, so the service is asked for it: the whole of it to bring it down, a
    byte to say it is still wanted."""
    try:
        with open(where, "rb") as f:
            if not whole:
                f.read(1)
                return True
            while f.read(4 << 20):
                pass
        return True
    except OSError:
        return False
