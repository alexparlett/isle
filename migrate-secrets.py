#!/usr/bin/env python3
"""
Move secrets out of KWallet and into gnome-keyring, so KWallet can be removed.

    ./migrate-secrets.py              list what would move, change nothing
    ./migrate-secrets.py --run        actually migrate
    ./migrate-secrets.py --run --keep-kwalletd   don't stop kwalletd afterwards

Why this exists: KWallet and gnome-keyring both provide org.freedesktop.secrets,
and on this machine PAM starts and unlocks both at login. Whichever claims the
bus name first wins, which is why "my password isn't where I expect" is such a
common complaint on mixed systems. Removing KWallet resolves the ambiguity, but
only if what it holds gets moved first.

How it works:
  1. Enumerate KWallet over D-Bus and read every password entry into memory.
  2. Stop kwalletd, so gnome-keyring is unambiguously the Secret Service.
  3. Write each entry back through python-keyring's SecretService backend —
     the same code path Proton uses to read them, so the attribute schema is
     right by construction rather than by my guessing at it.
  4. Read every one back and compare before declaring success.

Nothing is deleted from KWallet. The .kwl file stays on disk untouched, so a
failed migration costs you nothing and can simply be re-run.

Secrets are held in memory only — never written to disk, never logged, never
passed as command-line arguments (which would be visible in ps).
"""

import argparse
import os
import subprocess
import sys

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

APPID = "hypr-migrate-secrets"
WALLET = "kdewallet"

# KWallet entry types, from KWallet::EntryType.
ENTRY_UNKNOWN, ENTRY_PASSWORD, ENTRY_STREAM, ENTRY_MAP = 0, 1, 2, 3

c = {
    "reset": "\033[0m", "bold": "\033[1m", "dim": "\033[2m",
    "green": "\033[32m", "yellow": "\033[33m", "red": "\033[31m", "blue": "\033[34m",
}


def info(msg): print(f"{c['blue']}{c['bold']}==>{c['reset']} {msg}")
def ok(msg): print(f"  {c['green']}✓{c['reset']} {msg}")
def warn(msg): print(f"  {c['yellow']}!{c['reset']} {msg}")
def err(msg): print(f"  {c['red']}✗{c['reset']} {msg}", file=sys.stderr)
def skip(msg): print(f"  {c['dim']}·{c['reset']} {msg}")


# --------------------------------------------------------------------------
# KWallet, over D-Bus. python-keyring's kwallet backend can only fetch a
# secret you already know the name of, so enumeration has to be done directly.
# --------------------------------------------------------------------------

class KWallet:
    def __init__(self, bus):
        self.proxy = Gio.DBusProxy.new_sync(
            bus, Gio.DBusProxyFlags.NONE, None,
            "org.kde.kwalletd6", "/modules/kwalletd6", "org.kde.KWallet", None,
        )
        self.handle = -1

    def call(self, method, sig, *args):
        return self.proxy.call_sync(
            method, GLib.Variant(sig, args), Gio.DBusCallFlags.NONE, 5000, None
        ).unpack()[0]

    def open(self):
        self.handle = self.call("open", "(sxs)", WALLET, 0, APPID)
        if self.handle < 0:
            raise RuntimeError("kwalletd refused to open the wallet (dismissed prompt?)")
        return self.handle

    def folders(self):
        return self.call("folderList", "(is)", self.handle, APPID)

    def entries(self, folder):
        return self.call("entryList", "(iss)", self.handle, folder, APPID)

    def entry_type(self, folder, key):
        return self.call("entryType", "(isss)", self.handle, folder, key, APPID)

    def read_password(self, folder, key):
        return self.call("readPassword", "(isss)", self.handle, folder, key, APPID)

    def close(self):
        if self.handle >= 0:
            try:
                self.call("close", "(ibs)", self.handle, False, APPID)
            except GLib.Error:
                pass


def secrets_owner(bus):
    """Which process currently owns org.freedesktop.secrets, if any."""
    try:
        dbus = Gio.DBusProxy.new_sync(
            bus, Gio.DBusProxyFlags.NONE, None,
            "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", None,
        )
        name = dbus.call_sync("GetNameOwner",
                              GLib.Variant("(s)", ("org.freedesktop.secrets",)),
                              Gio.DBusCallFlags.NONE, 3000, None).unpack()[0]
        pid = dbus.call_sync("GetConnectionUnixProcessID",
                             GLib.Variant("(s)", (name,)),
                             Gio.DBusCallFlags.NONE, 3000, None).unpack()[0]
        with open(f"/proc/{pid}/comm") as fh:
            return fh.read().strip(), pid
    except (GLib.Error, OSError):
        return None, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", action="store_true", help="actually migrate")
    ap.add_argument("--keep-kwalletd", action="store_true",
                    help="leave kwalletd running afterwards (it will keep owning the bus)")
    args = ap.parse_args()

    try:
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    except GLib.Error as exc:
        err(f"no session D-Bus: {exc.message}")
        err("run this from inside a desktop session, as the user who owns the wallet")
        return 1

    # --- read side --------------------------------------------------------

    info("Reading KWallet")
    try:
        wallet = KWallet(bus)
        wallet.open()
    except (GLib.Error, RuntimeError) as exc:
        err(f"cannot open KWallet: {exc}")
        err("is kwalletd6 running and unlocked? log in normally first.")
        return 1

    collected, skipped = [], []
    try:
        for folder in wallet.folders():
            for key in wallet.entries(folder):
                kind = wallet.entry_type(folder, key)
                if kind != ENTRY_PASSWORD:
                    skipped.append((folder, key, kind))
                    continue
                collected.append((folder, key, wallet.read_password(folder, key)))
    except GLib.Error as exc:
        err(f"failed while reading: {exc}")
        wallet.close()
        return 1

    ok(f"{len(collected)} password entries readable")
    for folder, key, _ in collected:
        print(f"      {folder} / {key}")
    if skipped:
        warn(f"{len(skipped)} non-password entries cannot be migrated automatically:")
        for folder, key, kind in skipped:
            kindname = {ENTRY_STREAM: "binary", ENTRY_MAP: "map"}.get(kind, "unknown")
            print(f"      {folder} / {key}  ({kindname})")
        warn("copy those by hand from kwalletmanager before removing KWallet")

    if not collected:
        wallet.close()
        info("Nothing to migrate.")
        return 0

    if not args.run:
        wallet.close()
        print()
        info("Dry run — no secrets were written. Re-run with --run.")
        skip("values were read into memory to verify they are readable, "
             "but never printed or stored")
        return 0

    # --- switch the Secret Service ---------------------------------------

    info("Handing the Secret Service to gnome-keyring")

    owner, pid = secrets_owner(bus)
    if owner:
        skip(f"org.freedesktop.secrets is currently served by {owner} (pid {pid})")

    if not args.keep_kwalletd:
        # Stopping kwalletd is what frees the bus name. It restarts on demand,
        # which is fine — by then the entries live in gnome-keyring.
        for cmd in (["kquitapp6", "kwalletd6"], ["pkill", "-x", "kwalletd6"]):
            if subprocess.run(cmd, capture_output=True).returncode == 0:
                ok(f"stopped kwalletd via {cmd[0]}")
                break
        else:
            warn("could not stop kwalletd; it may reclaim the bus name")

    subprocess.run(
        ["gnome-keyring-daemon", "--start", "--components=secrets"],
        capture_output=True,
    )

    owner, pid = secrets_owner(bus)
    if owner and "gnome-keyring" not in owner:
        err(f"org.freedesktop.secrets is still served by {owner} — refusing to write")
        err("writing now would put the secrets straight back into KWallet")
        wallet.close()
        return 1
    ok(f"gnome-keyring owns the Secret Service ({owner or 'activatable'})")

    # --- write side -------------------------------------------------------
    #
    # Going through python-keyring rather than secret-tool: it is the same code
    # path Proton reads with, so the attribute schema matches by construction.

    info("Writing to gnome-keyring")
    try:
        from keyring.backends import SecretService
        backend = SecretService.Keyring()
    except Exception as exc:  # noqa: BLE001 - any import/init failure is fatal here
        err(f"python-keyring SecretService backend unavailable: {exc}")
        wallet.close()
        return 1

    written, failed = 0, []
    for folder, key, value in collected:
        # KWallet's folder/key is python-keyring's service/username.
        try:
            backend.set_password(folder, key, value)
            written += 1
        except Exception as exc:  # noqa: BLE001
            failed.append((folder, key, exc))

    ok(f"{written} entries written")
    for folder, key, exc in failed:
        err(f"failed: {folder} / {key}: {exc}")

    # --- verify -----------------------------------------------------------

    info("Verifying")
    mismatched = []
    for folder, key, value in collected:
        try:
            if backend.get_password(folder, key) != value:
                mismatched.append((folder, key))
        except Exception:  # noqa: BLE001
            mismatched.append((folder, key))

    wallet.close()

    if mismatched or failed:
        err(f"{len(mismatched)} entries did not read back correctly")
        for folder, key in mismatched:
            print(f"      {folder} / {key}")
        err("KWallet is untouched — fix the above and re-run before removing it")
        return 1

    ok(f"all {written} entries verified in gnome-keyring")
    print()
    info("Done. KWallet still holds its copy; nothing was deleted.")
    skip("remove it with: ./remove-plasma.sh --run --apps --drop-kwallet")
    if skipped:
        warn(f"remember the {len(skipped)} non-password entries listed above")
    return 0


if __name__ == "__main__":
    os.umask(0o077)
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print()
        sys.exit(130)
