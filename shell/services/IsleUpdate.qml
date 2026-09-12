pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The shell's own updates: the checkout behind ~/.config against its origin. Checked hourly and on demand; pulling
// pulls and runs install.sh, which relinks and re-renders, and Quickshell reloads itself from the changed files.
Singleton {
    id: root

    // shellDir is a link into the checkout; the parent must be taken after resolving it.
    readonly property string repo: Quickshell.shellDir + "/.."
    property string head: ""
    property string headDate: ""
    property string remote: ""
    property string branch: "main"
    property var incoming: []
    readonly property int behind: incoming.length
    property bool checking: false
    property string checkedAt: ""
    property string error: ""
    // origin's https form, for a fetch when origin is an SSH remote this session cannot use.
    readonly property string publicUrl: {
        const m = remote.match(/^(?:ssh:\/\/)?(?:[\w.-]+@)?([\w.-]+)[:\/]+(.+?)(?:\.git)?\/?$/);
        return remote.indexOf("http") === 0 ? remote : m ? "https://" + m[1] + "/" + m[2] + ".git" : remote;
    }
    // origin refused the fetch and the public URL served it: reads work, and a push from here will not.
    property bool viaHttps: false

    Process {
        id: info
        command: ["sh", "-c", "cd -P \"$1\" && git rev-parse --short HEAD && git log -1 --format=%cs && git remote get-url origin && git rev-parse --abbrev-ref HEAD", "_", root.repo]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n");
                if (l.length >= 4) { root.head = l[0]; root.headDate = l[1]; root.remote = l[2]; root.branch = l[3]; }
            }
        }
    }
    Timer { interval: 3600000; running: true; repeat: true; triggeredOnStart: true; onTriggered: root.check() }

    // origin first; when that is an SSH remote GitHub does not know, the public https URL. BatchMode: from a session,
    // ssh would otherwise ask on the controlling tty and wait forever.
    Process {
        id: fetcher
        command: ["sh", "-c",
            "cd -P \"$1\" && export GIT_TERMINAL_PROMPT=0 GIT_SSH_COMMAND='ssh -o BatchMode=yes -o ConnectTimeout=15'; "
            + "if timeout 120 git fetch -q origin \"$2\" 2>/dev/null; then echo @origin; elif timeout 120 git fetch -q \"$3\" \"$2\" 2>&1; then echo @https; else exit 1; fi; "
            + "git log --format=%s HEAD..FETCH_HEAD", "_", root.repo, root.branch, root.publicUrl]
        onStarted: root.checking = true
        property int code: 0
        onExited: c => code = c
        stdout: StdioCollector {
            onStreamFinished: {
                const lines = text.trim().split("\n");
                root.checking = false;
                root.checkedAt = Qt.formatTime(new Date(), "HH:mm");
                if (fetcher.code !== 0) { root.error = lines.find(l => /Permission denied|fatal:|error:/.test(l)) || lines[0] || "fetch failed"; return; }
                root.error = "";
                root.viaHttps = lines[0] === "@https";
                root.incoming = lines.slice(1).filter(l => l);
            }
        }
    }
    function check() { if (!fetcher.running) { info.running = true; fetcher.running = true; } }

    // The pull and the install, their lines kept; the shell reloads itself from the changed files after.
    property bool running: false
    property string phase: ""
    property var log: []
    property string failed: ""
    Process {
        id: updater
        stdout: SplitParser { onRead: line => root.took(line) }
        stderr: SplitParser { onRead: line => root.took(line) }
        onStarted: { root.running = true; root.failed = ""; root.log = []; root.phase = "Pulling"; }
        onExited: (code) => {
            root.running = false;
            root.phase = code === 0 ? "Done" : "Stopped";
            if (code !== 0 && !root.failed) root.failed = "Stopped with code " + code;
            info.running = true; root.check();
            IslandEvents.show({ kind: "text", duration: 5000, glyph: code === 0 ? "check" : "x", text: code === 0 ? "Isle updated" : "Isle update stopped", detail: code === 0 ? "" : root.failed });
        }
    }
    function took(line) {
        const l = line.replace(/\x1b\[[0-9;]*m/g, "").replace(/\s+$/, "");
        if (!l) return;
        log = log.concat([l]).slice(-200);
        if (/^(Updating|Fast-forward)/.test(l)) phase = "Pulling";
        else if (/✓/.test(l)) phase = l.replace(/^\s*✓\s*/, "");
        else if (/^(fatal|error):/i.test(l)) failed = l.replace(/^(fatal|error):\s*/i, "");
    }
    function update() {
        if (updater.running) return;
        updater.command = ["sh", "-c", "cd -P \"$1\" && git pull --ff-only \"$2\" \"$3\" && tools/install.sh", "_", repo, viaHttps ? publicUrl : "origin", branch];
        updater.running = true;
    }
}
