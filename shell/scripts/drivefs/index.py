"""The tree of a drive, kept locally, so a listing never asks the service (D65).

A row per file and folder, the rowid standing as the inode, which is why rows are never reused: a file the
kernel still holds a number for must keep it. `dirs` says when a folder was last listed, so a stale one is
served as it is and refreshed behind the reader.
"""
import os, sqlite3, time

SCHEMA = """
CREATE TABLE IF NOT EXISTS nodes (
    ino     INTEGER PRIMARY KEY,
    parent  INTEGER NOT NULL,
    name    TEXT NOT NULL,
    is_dir  INTEGER NOT NULL,
    size    INTEGER NOT NULL DEFAULT 0,
    mtime   REAL NOT NULL DEFAULT 0,
    sha1    TEXT,
    state   TEXT NOT NULL DEFAULT 'cloud',
    emblem  TEXT,
    pinned  INTEGER NOT NULL DEFAULT 0,
    used    REAL NOT NULL DEFAULT 0,
    UNIQUE(parent, name)
);
CREATE INDEX IF NOT EXISTS nodes_parent ON nodes(parent);
CREATE TABLE IF NOT EXISTS dirs (
    ino       INTEGER PRIMARY KEY,
    listed_at REAL NOT NULL DEFAULT 0
);
"""
ROOT = 1


class Index:
    def __init__(self, path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.db = sqlite3.connect(path, check_same_thread=False)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.execute("PRAGMA synchronous=NORMAL")
        self.db.executescript(SCHEMA)
        # A drive indexed before the badges existed has no column for them.
        if "emblem" not in {r["name"] for r in self.db.execute("PRAGMA table_info(nodes)")}:
            self.db.execute("ALTER TABLE nodes ADD COLUMN emblem TEXT")
        # The root is its own parent, so the walk up from any node ends.
        self.db.execute("INSERT OR IGNORE INTO nodes (ino, parent, name, is_dir) VALUES (?, ?, '', 1)", (ROOT, ROOT))
        self.db.commit()

    def node(self, ino):
        return self.db.execute("SELECT * FROM nodes WHERE ino = ?", (ino,)).fetchone()

    def child(self, parent, name):
        return self.db.execute("SELECT * FROM nodes WHERE parent = ? AND name = ? AND ino != parent",
                               (parent, name)).fetchone()

    def children(self, parent):
        # The root is its own parent, so it would otherwise be its own child.
        return self.db.execute("SELECT * FROM nodes WHERE parent = ? AND ino != parent ORDER BY ino",
                               (parent,)).fetchall()

    def path(self, ino):
        """The remote path of a node, walked up to the root."""
        parts = []
        while ino != ROOT:
            r = self.node(ino)
            if r is None:
                return None
            parts.append(r["name"])
            ino = r["parent"]
        return "/".join(reversed(parts))

    def listed_at(self, ino):
        r = self.db.execute("SELECT listed_at FROM dirs WHERE ino = ?", (ino,)).fetchone()
        return r["listed_at"] if r else 0

    def fresh(self, ino, within):
        return (time.time() - self.listed_at(ino)) < within

    def add_entries(self, parent, entries):
        """Names as they arrive. A row that stays keeps its inode, so an open file does not change identity
        under its reader."""
        for e in entries:
            row = self.child(parent, e["name"])
            if row is None:
                self.db.execute(
                    "INSERT INTO nodes (parent, name, is_dir, size, mtime) VALUES (?, ?, ?, ?, ?)",
                    (parent, e["name"], int(e["is_dir"]), e["size"], e["mtime"]))
            elif (row["size"], row["mtime"], bool(row["is_dir"])) != (e["size"], e["mtime"], e["is_dir"]):
                self.db.execute("UPDATE nodes SET size = ?, mtime = ?, is_dir = ? WHERE ino = ?",
                                (e["size"], e["mtime"], int(e["is_dir"]), row["ino"]))
        self.db.commit()

    def finish_listing(self, parent, seen):
        """What the folder no longer holds, gone. Only ever called for a listing that arrived in full: a
        partial one would take the names that had not arrived yet for names that had been deleted."""
        for row in self.children(parent):
            if row["name"] not in seen:
                self.forget(row["ino"])
        self.db.execute("INSERT INTO dirs (ino, listed_at) VALUES (?, ?) "
                        "ON CONFLICT(ino) DO UPDATE SET listed_at = excluded.listed_at", (parent, time.time()))
        self.db.commit()

    def put_listing(self, parent, entries):
        self.add_entries(parent, entries)
        self.finish_listing(parent, {e["name"] for e in entries})

    def insert(self, parent, name, is_dir, size=0, mtime=None, state="cached"):
        cur = self.db.execute(
            "INSERT INTO nodes (parent, name, is_dir, size, mtime, state) VALUES (?, ?, ?, ?, ?, ?)",
            (parent, name, int(is_dir), size, mtime if mtime is not None else time.time(), state))
        self.db.commit()
        return self.node(cur.lastrowid)

    def stat_is(self, ino, size, mtime, state=None):
        if state is None:
            self.db.execute("UPDATE nodes SET size = ?, mtime = ? WHERE ino = ?", (size, mtime, ino))
        else:
            self.db.execute("UPDATE nodes SET size = ?, mtime = ?, state = ? WHERE ino = ?",
                            (size, mtime, state, ino))
        self.db.commit()

    def move(self, ino, parent, name):
        """A rename keeps the inode: the service moves the object rather than making another one."""
        old = self.child(parent, name)
        if old is not None and old["ino"] != ino:
            self.forget(old["ino"])
        self.db.execute("UPDATE nodes SET parent = ?, name = ? WHERE ino = ?", (parent, name, ino))
        self.db.commit()

    def drop(self, ino):
        self.forget(ino)
        self.db.commit()

    def forget(self, ino):
        """A node and everything under it, gone from the index."""
        for r in self.children(ino):
            self.forget(r["ino"])
        self.db.execute("DELETE FROM nodes WHERE ino = ?", (ino,))
        self.db.execute("DELETE FROM dirs WHERE ino = ?", (ino,))

    def set_pinned(self, ino, on):
        """A folder is pinned with everything under it: what a person points at is what they mean."""
        self.db.execute("UPDATE nodes SET pinned = ? WHERE ino = ?", (int(on), ino))
        for row in self.children(ino):
            self.set_pinned(row["ino"], on)
        self.db.commit()

    def pinned_files(self):
        return self.db.execute("SELECT * FROM nodes WHERE pinned = 1 AND is_dir = 0").fetchall()

    def by_path(self, path):
        """The node at a path under the drive, or None."""
        ino = ROOT
        for part in [p for p in path.strip("/").split("/") if p]:
            row = self.child(ino, part)
            if row is None:
                return None
            ino = row["ino"]
        return self.node(ino)

    def under(self, ino):
        """A node and everything below it."""
        out = [self.node(ino)]
        for row in self.children(ino):
            out.extend(self.under(row["ino"]) if row["is_dir"] else [row])
        return [r for r in out if r is not None]

    def used_now(self, ino):
        self.db.execute("UPDATE nodes SET used = ? WHERE ino = ?", (time.time(), ino))

    def wears(self, ino, badge):
        self.db.execute("UPDATE nodes SET emblem = ? WHERE ino = ?", (badge, ino))
        self.db.commit()

    def counts(self):
        r = self.db.execute("SELECT COUNT(*) n, COALESCE(SUM(size), 0) b FROM nodes WHERE is_dir = 0").fetchone()
        return r["n"], r["b"]
