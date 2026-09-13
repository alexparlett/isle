"""The drive as a filesystem: listings from the index, contents from the mount underneath (D65).

Everything the kernel asks about a name, a folder or a file's attributes is answered from SQLite, so a file
manager that walks every folder to count what is in it pays nothing. Only reading a file goes to the service,
and that has a deadline: a passed-through call that does not answer becomes an error rather than a process
nobody can kill.
"""
import errno, os, stat, time

import pyfuse3
import trio

import index as idx

DEADLINE = 30
# How long a first visit to a folder nobody has listed waits before it shows what it has and fills in behind.
FIRST_WAIT = 3
STALE = 300


class Drive(pyfuse3.Operations):
    supports_dot_lookup = False
    enable_writeback_cache = False

    def __init__(self, ix, under, remote, lister):
        super().__init__()
        self.ix = ix
        # The private mount, where the transport lives and no file manager looks.
        self.under = under
        self.remote = remote
        self.lister = lister
        self.open_files = {}
        self.next_fh = 1
        self.nursery = None
        # A folder is listed once at a time, whoever asks: the event is set when the listing lands.
        self.listing = {}
        # Folders someone has opened, whose badges are worth painting.
        self.seen = set()

    # --- attributes ------------------------------------------------------
    def attrs(self, row):
        a = pyfuse3.EntryAttributes()
        a.st_mode = (stat.S_IFDIR | 0o700) if row["is_dir"] else (stat.S_IFREG | 0o600)
        a.st_size = 0 if row["is_dir"] else row["size"]
        ns = int(row["mtime"] * 1e9)
        a.st_atime_ns = a.st_ctime_ns = a.st_mtime_ns = ns
        a.st_nlink = 2 if row["is_dir"] else 1
        a.st_uid = os.getuid()
        a.st_gid = os.getgid()
        a.st_ino = row["ino"]
        a.st_blksize = 512
        a.st_blocks = (a.st_size + 511) // 512
        a.entry_timeout = a.attr_timeout = 5
        return a

    async def getattr(self, inode, ctx=None):
        row = self.ix.node(inode)
        if row is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        return self.attrs(row)

    async def lookup(self, parent, name, ctx=None):
        await self.ensure(parent)
        row = self.ix.child(parent, os.fsdecode(name))
        if row is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        return self.attrs(row)

    # --- folders ---------------------------------------------------------
    async def ensure(self, ino):
        """A folder nobody has listed is waited on briefly and then shown as it stands, filling in behind:
        a folder of several thousand files takes the service a minute and a half, which is not a wait to put
        a person in. A stale one is served as it is and refreshed behind the reader."""
        if self.ix.listed_at(ino) == 0:
            done = self.start_refresh(ino)
            if done is not None:
                with trio.move_on_after(FIRST_WAIT):
                    await done.wait()
        elif not self.ix.fresh(ino, STALE):
            self.start_refresh(ino)

    def start_refresh(self, ino):
        """The listing of one folder, at most one at a time. The event it returns is set when it lands."""
        if ino in self.listing:
            return self.listing[ino]
        if not self.nursery:
            return None
        done = trio.Event()
        self.listing[ino] = done
        self.nursery.start_soon(self.refresh, ino)
        return done

    async def refresh(self, ino):
        path = self.ix.path(ino)
        try:
            if path is not None:
                seen = set()

                def take(batch):
                    seen.update(e["name"] for e in batch)
                    self.ix.add_entries(ino, batch)

                whole = await trio.to_thread.run_sync(self.lister, self.remote, path, take)
                if whole:
                    self.ix.finish_listing(ino, seen)
        finally:
            done = self.listing.pop(ino, None)
            if done is not None:
                done.set()

    async def opendir(self, inode, ctx):
        await self.ensure(inode)
        return inode

    async def readdir(self, fh, start, token):
        self.seen.add(fh)
        rows = self.ix.children(fh)
        for i, row in enumerate(rows):
            if i < start:
                continue
            if not pyfuse3.readdir_reply(token, os.fsencode(row["name"]), self.attrs(row), i + 1):
                return

    async def releasedir(self, fh):
        pass

    # --- files -----------------------------------------------------------
    def below(self, ino):
        path = self.ix.path(ino)
        return None if path is None else os.path.join(self.under, path)

    async def open(self, inode, flags, ctx):
        where = self.below(inode)
        if where is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        try:
            with trio.fail_after(DEADLINE):
                fd = await trio.to_thread.run_sync(os.open, where, flags & ~os.O_NOFOLLOW,
                                                   abandon_on_cancel=True)
        except trio.TooSlowError:
            raise pyfuse3.FUSEError(errno.EIO)
        except OSError as e:
            raise pyfuse3.FUSEError(e.errno or errno.EIO)
        return pyfuse3.FileInfo(fh=self.hold(fd, inode))

    async def read(self, fh, off, size):
        held = self.open_files.get(fh)
        if held is None:
            raise pyfuse3.FUSEError(errno.EBADF)
        fd = held[0]
        try:
            with trio.fail_after(DEADLINE):
                return await trio.to_thread.run_sync(os.pread, fd, size, off, abandon_on_cancel=True)
        except trio.TooSlowError:
            raise pyfuse3.FUSEError(errno.EIO)
        except OSError as e:
            raise pyfuse3.FUSEError(e.errno or errno.EIO)

    def holding(self, ino):
        """Whether something has this file open, which is reason enough not to take its copy away."""
        return any(held[1] == ino for held in self.open_files.values())

    def hold(self, fd, ino):
        # [fd, inode, written to]: a handle nothing wrote through leaves the file's state alone.
        fh = self.next_fh
        self.next_fh += 1
        self.open_files[fh] = [fd, ino, False]
        return fh

    async def write(self, fh, off, buf):
        held = self.open_files.get(fh)
        if held is None:
            raise pyfuse3.FUSEError(errno.EBADF)
        try:
            with trio.fail_after(DEADLINE):
                held[2] = True
                return await trio.to_thread.run_sync(os.pwrite, held[0], buf, off, abandon_on_cancel=True)
        except trio.TooSlowError:
            raise pyfuse3.FUSEError(errno.EIO)
        except OSError as e:
            raise pyfuse3.FUSEError(e.errno or errno.EIO)

    async def create(self, parent, name, mode, flags, ctx):
        where = self.below(parent)
        if where is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        text = os.fsdecode(name)
        try:
            with trio.fail_after(DEADLINE):
                fd = await trio.to_thread.run_sync(os.open, os.path.join(where, text),
                                                  flags | os.O_CREAT, 0o600, abandon_on_cancel=True)
        except trio.TooSlowError:
            raise pyfuse3.FUSEError(errno.EIO)
        except OSError as e:
            raise pyfuse3.FUSEError(e.errno or errno.EIO)
        row = self.ix.child(parent, text) or self.ix.insert(parent, text, False, 0, state="uploading")
        return pyfuse3.FileInfo(fh=self.hold(fd, row["ino"])), self.attrs(row)

    async def mkdir(self, parent, name, mode, ctx):
        where = self.below(parent)
        if where is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        text = os.fsdecode(name)
        await self.down(os.mkdir, os.path.join(where, text), 0o700)
        row = self.ix.child(parent, text) or self.ix.insert(parent, text, True)
        return self.attrs(row)

    async def unlink(self, parent, name, ctx):
        await self.gone(parent, name, os.unlink)

    async def rmdir(self, parent, name, ctx):
        await self.gone(parent, name, os.rmdir)

    async def gone(self, parent, name, how):
        where = self.below(parent)
        text = os.fsdecode(name)
        row = self.ix.child(parent, text)
        if where is None or row is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        await self.down(how, os.path.join(where, text))
        self.ix.drop(row["ino"])

    async def rename(self, parent_old, name_old, parent_new, name_new, flags, ctx):
        if flags != 0:
            raise pyfuse3.FUSEError(errno.EINVAL)
        a, b = self.below(parent_old), self.below(parent_new)
        old, new = os.fsdecode(name_old), os.fsdecode(name_new)
        row = self.ix.child(parent_old, old)
        if a is None or b is None or row is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        # The service moves the object rather than copying it, so this is cheap whatever the file weighs.
        await self.down(os.rename, os.path.join(a, old), os.path.join(b, new))
        self.ix.move(row["ino"], parent_new, new)

    async def setattr(self, inode, attr, fields, fh, ctx):
        row = self.ix.node(inode)
        where = self.below(inode)
        if row is None or where is None:
            raise pyfuse3.FUSEError(errno.ENOENT)
        if fields.update_size:
            await self.down(os.truncate, where, attr.st_size)
            self.ix.stat_is(inode, attr.st_size, time.time(), "uploading")
        if fields.update_mtime:
            when = attr.st_mtime_ns / 1e9
            await self.down(os.utime, where, (when, when))
            self.ix.stat_is(inode, row["size"], when)
        return await self.getattr(inode)

    async def down(self, how, *args):
        """One call to the mount underneath, with a deadline: a service that stops answering becomes an
        error here rather than a process nobody can kill."""
        try:
            with trio.fail_after(DEADLINE):
                return await trio.to_thread.run_sync(how, *args, abandon_on_cancel=True)
        except trio.TooSlowError:
            raise pyfuse3.FUSEError(errno.EIO)
        except OSError as e:
            raise pyfuse3.FUSEError(e.errno or errno.EIO)

    async def flush(self, fh):
        await self.settle(fh)

    async def fsync(self, fh, datasync):
        await self.settle(fh)

    async def settle(self, fh):
        """What the file is now, into the index: whoever lists the folder next should see the size the write
        left behind rather than the one it had."""
        held = self.open_files.get(fh)
        if held is None:
            return
        try:
            st = await trio.to_thread.run_sync(os.fstat, held[0], abandon_on_cancel=True)
        except OSError:
            return
        row = self.ix.node(held[1])
        if row is not None and not row["is_dir"]:
            self.ix.stat_is(held[1], st.st_size, st.st_mtime, "uploading" if held[2] else None)

    async def release(self, fh):
        await self.settle(fh)
        held = self.open_files.pop(fh, None)
        if held is not None:
            await trio.to_thread.run_sync(os.close, held[0])

    async def statfs(self, ctx):
        s = pyfuse3.StatvfsData()
        count, used = self.ix.counts()
        s.f_bsize = s.f_frsize = 4096
        s.f_blocks = max(1, used // 4096)
        s.f_bfree = s.f_bavail = 0
        s.f_files = count
        s.f_ffree = s.f_favail = 0
        s.f_namemax = 255
        return s
