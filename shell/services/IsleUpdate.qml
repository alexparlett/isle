pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The shell's own updates: the checkout behind ~/.config against its origin. Checked hourly and on demand; pulling
// runs in a terminal, then install.sh relinks and re-renders, and Quickshell reloads itself from the changed files.
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

    Process { id: updater; onExited: { info.running = true; root.check(); } }
    function update() {
        updater.command = ["kitty", "--class", "isle-windows", "--title", "Isle update", "-e", "sh", "-c",
            "cd -P \"" + repo + "\" && git pull --ff-only " + (viaHttps ? publicUrl : "origin") + " " + branch + " && tools/install.sh; echo; echo Done. The shell reloads on its own. Press Enter.; read x"];
        updater.running = true;
    }
}
