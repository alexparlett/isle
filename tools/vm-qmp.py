#!/usr/bin/env python3
"""
Drive the test VM through QEMU's QMP socket.

    tools/vm-qmp.py shot [out.png]        screenshot the guest's display
    tools/vm-qmp.py key ctrl-alt-f2       press a chord
    tools/vm-qmp.py drag x1 y1 x2 y2      drag with the left button
    tools/vm-qmp.py type "hello world"    type a string
    tools/vm-qmp.py click 640 400         click at guest pixel coordinates
    tools/vm-qmp.py move 640 400          move the pointer without clicking
    tools/vm-qmp.py info                  display size and VM status

This exists so the guest can be driven and inspected without anyone looking at
the QEMU window. Everything an agent claims about the VM should come from a
screenshot taken this way or by grim inside the guest.

Needs `-qmp unix:...` on the QEMU command line; tools/test-vm.sh adds it.
"""
import json, os, socket, subprocess, sys, tempfile, time

SOCK = os.path.join(
    os.environ.get("XDG_DATA_HOME", os.path.expanduser("~/.local/share")),
    "hypr-testvm", "qmp.sock")

# QEMU's absolute axes are always 0..32767 regardless of the real resolution.
ABS_MAX = 32767


class Qmp:
    def __init__(self, path=SOCK):
        if not os.path.exists(path):
            sys.exit(f"no QMP socket at {path}\n"
                     "The VM is not running, or it predates the -qmp flag —\n"
                     "restart it with tools/test-vm.sh run")
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.settimeout(10)
        try:
            self.s.connect(path)
        except ConnectionRefusedError:
            # The socket file outlives the QEMU that made it, so "refused" here
            # almost always means a dead VM rather than a permissions problem.
            os.unlink(path)
            sys.exit(f"stale socket at {path} (removed)\n"
                     "The VM that created it is gone. Start one with\n"
                     "  tools/test-vm.sh run")
        except OSError as e:
            sys.exit(f"cannot connect to {path}: {e}")
        self.f = self.s.makefile("rw", encoding="utf-8", newline="\n")
        self._read()                       # greeting
        self.cmd("qmp_capabilities")

    def _read(self):
        while True:
            line = self.f.readline()
            if not line:
                sys.exit("QMP closed the connection")
            msg = json.loads(line)
            if "event" in msg:             # asynchronous, not our reply
                continue
            return msg

    def cmd(self, execute, **args):
        payload = {"execute": execute}
        if args:
            payload["arguments"] = args
        self.f.write(json.dumps(payload) + "\n")
        self.f.flush()
        reply = self._read()
        if "error" in reply:
            sys.exit(f"{execute} failed: {reply['error'].get('desc', reply['error'])}")
        return reply.get("return", {})


# --- keys --------------------------------------------------------------------
# QEMU qcodes. Only what is needed to drive an installer and a shell.

NAMED = {
    " ": "spc", "\n": "ret", "\t": "tab",
    "-": "minus", "=": "equal", "[": "bracket_left", "]": "bracket_right",
    "\\": "backslash", ";": "semicolon", "'": "apostrophe", "`": "grave_accent",
    ",": "comma", ".": "dot", "/": "slash",
}
SHIFTED = {
    "!": "1", "@": "2", "#": "3", "$": "4", "%": "5", "^": "6", "&": "7",
    "*": "8", "(": "9", ")": "0", "_": "-", "+": "=", "{": "[", "}": "]",
    "|": "\\", ":": ";", '"': "'", "~": "`", "<": ",", ">": ".", "?": "/",
}
ALIASES = {
    "enter": "ret", "return": "ret", "esc": "esc", "escape": "esc",
    "space": "spc", "super": "meta_l", "win": "meta_l", "cmd": "meta_l",
    "ctrl": "ctrl", "alt": "alt", "shift": "shift", "backspace": "backspace",
    "del": "delete", "up": "up", "down": "down", "left": "left",
    "right": "right", "tab": "tab", "pgup": "pgup", "pgdn": "pgdn",
    # The media keys by their X keysym names, which is how hypr/keymap.lua
    # spells them; QEMU's qcodes are the short forms.
    "xf86audioraisevolume": "volumeup", "xf86audiolowervolume": "volumedown",
    "xf86audiomute": "audiomute", "xf86audioplay": "audioplay",
    "xf86audionext": "audionext", "xf86audioprev": "audioprev",
}


def qcodes_for_char(ch):
    """Return the qcode list for one character, with shift where needed."""
    if ch in NAMED:
        return [NAMED[ch]]
    if ch in SHIFTED:
        base = SHIFTED[ch]
        return ["shift", NAMED.get(base, base)]
    if ch.isupper():
        return ["shift", ch.lower()]
    if ch.isalnum():
        return [ch]
    raise SystemExit(f"cannot type {ch!r} — add it to vm-qmp.py")


MODIFIERS = {"shift", "ctrl", "alt", "meta_l", "meta_r", "shift_r", "ctrl_r", "altgr"}


def send_keys(q, codes, hold=None):
    """Press a chord with explicit down/up events.

    NOT `send-key`. That command takes a key list and a hold-time and is meant
    to do exactly this, but successive calls leak modifier state into the
    guest: typing "/authorized_keys" came back as "?authorie_KEYS" — a stuck
    shift from the previous chord turned "/" into "?" and lowercased letters
    into capitals. Nothing in the protocol reports it, and it looks like
    dropped characters rather than a stuck key, which is what made it slow to
    find.

    input-send-event gives us the press and the release separately, so every
    modifier is explicitly lifted in reverse order and no state survives the
    call.
    """
    mods = [c for c in codes if c in MODIFIERS]
    keys = [c for c in codes if c not in MODIFIERS]

    def ev(code, down):
        return {"type": "key",
                "data": {"down": down, "key": {"type": "qcode", "data": code}}}

    events = [ev(m, True) for m in mods]
    events += [ev(k, True) for k in keys]
    events += [ev(k, False) for k in reversed(keys)]
    events += [ev(m, False) for m in reversed(mods)]
    q.cmd("input-send-event", events=events)


def release_all(q):
    """Lift every modifier, unconditionally. Cheap insurance before typing."""
    for m in ("shift", "ctrl", "alt", "meta_l"):
        try:
            q.cmd("input-send-event", events=[
                {"type": "key",
                 "data": {"down": False, "key": {"type": "qcode", "data": m}}}])
        except SystemExit:
            pass


# --- commands ----------------------------------------------------------------

def do_shot(q, argv):
    out = os.path.abspath(argv[0]) if argv else os.path.abspath("vm.png")
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    # QEMU writes the file itself, as root-of-its-own-process; PNG since 7.1.
    q.cmd("screendump", filename=out, format="png")
    for _ in range(50):                    # the write is not synchronous
        if os.path.exists(out) and os.path.getsize(out) > 0:
            break
        time.sleep(0.1)
    else:
        sys.exit(f"screendump produced nothing at {out}")
    size = subprocess.run(["magick", "identify", "-format", "%wx%h", out],
                          capture_output=True, text=True).stdout.strip()
    print(f"{out}  {size}  {os.path.getsize(out)} bytes")


def do_key(q, argv):
    if not argv:
        sys.exit("usage: key <chord> [chord...]   e.g. ctrl-alt-f2, ret, super-s")
    for chord in argv:
        codes = [ALIASES.get(p.lower(), p.lower()) for p in chord.split("-")]
        send_keys(q, codes)
        time.sleep(0.05)
    print(f"sent {len(argv)} chord(s)")


# Typing through QMP is not reliable at speed. At ~12ms/char a 254-character
# command arrived with characters dropped and transposed — "authorized_keys"
# became "atorzdE C" — and nothing in the protocol reports it. The guest's
# input stack, not QEMU, is the bottleneck, so the only fix is to slow down.
#
# The real fix is not to type long commands: write a script to the 9p share
# from the host and type the one short line that runs it.
TYPE_DELAY = 0.030
TYPE_HOLD = 30
LONG_LINE = 120


def do_type(q, argv):
    if not argv:
        sys.exit("usage: type <string>")
    text = " ".join(argv)
    if len(text) > LONG_LINE:
        print(f"warning: {len(text)} chars is long enough to arrive corrupted.\n"
              f"         Prefer writing a script to the 9p share and typing\n"
              f"         'bash /repo/<script>' instead.", file=sys.stderr)
    release_all(q)
    for ch in text:
        send_keys(q, qcodes_for_char(ch))
        time.sleep(TYPE_DELAY)
    release_all(q)
    print(f"typed {len(text)} chars at {int(TYPE_DELAY*1000)}ms")


def _point(q, x, y, w, h):
    return [
        {"type": "abs", "data": {"axis": "x", "value": int(x / w * ABS_MAX)}},
        {"type": "abs", "data": {"axis": "y", "value": int(y / h * ABS_MAX)}},
    ]


def _display_size(q):
    # QMP exposes no geometry query (query-display-devices does not exist on
    # every build and carries no size anyway), so measure the screenshot.
    tmp = os.path.join(tempfile.gettempdir(), "vm-size.png")
    q.cmd("screendump", filename=tmp, format="png")
    for _ in range(50):
        if os.path.exists(tmp) and os.path.getsize(tmp):
            break
        time.sleep(0.1)
    out = subprocess.run(["magick", "identify", "-format", "%w %h", tmp],
                         capture_output=True, text=True).stdout.split()
    os.unlink(tmp)
    return int(out[0]), int(out[1])


def do_move(q, argv, click=False):
    if len(argv) < 2:
        sys.exit("usage: move|click <x> <y>")
    x, y = int(argv[0]), int(argv[1])
    w, h = _display_size(q)
    events = _point(q, x, y, w, h)
    q.cmd("input-send-event", events=events)
    if click:
        time.sleep(0.05)
        q.cmd("input-send-event", events=[
            {"type": "btn", "data": {"down": True, "button": "left"}}])
        time.sleep(0.05)
        q.cmd("input-send-event", events=[
            {"type": "btn", "data": {"down": False, "button": "left"}}])
    print(f"{'clicked' if click else 'moved to'} {x},{y}  (display {w}x{h})")


def do_drag(q, argv):
    """drag <x1> <y1> <x2> <y2>   press at one point, move in steps, release at the other"""
    if len(argv) < 4:
        sys.exit("usage: drag <x1> <y1> <x2> <y2>")
    x1, y1, x2, y2 = (int(a) for a in argv[:4])
    w, h = _display_size(q)
    q.cmd("input-send-event", events=_point(q, x1, y1, w, h))
    time.sleep(0.1)
    q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
    for i in range(1, 13):
        time.sleep(0.04)
        q.cmd("input-send-event", events=_point(q, x1 + (x2 - x1) * i // 12, y1 + (y2 - y1) * i // 12, w, h))
    time.sleep(0.1)
    q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
    print(f"dragged {x1},{y1} -> {x2},{y2}")


def do_raw(q, argv):
    """raw <command> [json-arguments]   send any QMP command, print the reply"""
    if not argv:
        sys.exit("usage: raw <command> [json-arguments]")
    args = json.loads(argv[1]) if len(argv) > 1 else {}
    print(json.dumps(q.cmd(argv[0], **args), indent=1))

def do_info(q, argv):
    status = q.cmd("query-status")
    w, h = _display_size(q)
    print(f"status:  {status.get('status')}  running={status.get('running')}")
    print(f"display: {w}x{h}")


def main():
    if len(sys.argv) < 2:
        print(__doc__.strip())
        return
    verb, argv = sys.argv[1], sys.argv[2:]
    q = Qmp()
    {"shot": do_shot, "key": do_key, "type": do_type, "info": do_info, "raw": do_raw, "drag": do_drag,
     "move": lambda q, a: do_move(q, a, False),
     "click": lambda q, a: do_move(q, a, True),
     }.get(verb, lambda *_: sys.exit(f"unknown command: {verb}"))(q, argv)


if __name__ == "__main__":
    main()
