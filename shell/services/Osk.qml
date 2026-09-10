pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The on-screen keyboard's state and its typing: every key becomes a wtype run, queued so fast taps land in
// order; wtype speaks the virtual keyboard protocol, so the text reaches whichever window has focus.
Singleton {
    id: root
    property bool open: false
    // "letters", "symbols" or "more".
    property string page: "letters"
    // 0 off, 1 for the next letter, 2 locked.
    property int shift: 0
    property bool available: false
    property bool hasDictionary: false
    Process {
        command: ["sh", "-c", "command -v wtype >/dev/null && echo wtype; test -r /usr/share/dict/words && echo dict"]
        running: true
        stdout: StdioCollector { onStreamFinished: { root.available = text.indexOf("wtype") >= 0; root.hasDictionary = text.indexOf("dict") >= 0; } }
    }

    // Over a game the pad is read all along, so Select held can bring the keyboard up.
    readonly property bool padWatch: Modes.game
    onPadWatchChanged: Gamepad.listeners += padWatch ? 1 : -1
    Connections { target: Gamepad; function onPressed(b) { if (b === "select-hold") root.toggle(); } }

    function show() { open = true; }
    function hide() { open = false; }
    function toggle() { open = !open; }

    // Named keys, as wtype's -k takes them.
    readonly property var named: ({ backspace: "BackSpace", enter: "Return", tab: "Tab", esc: "Escape", left: "Left", right: "Right", up: "Up", down: "Down", space: "space" })
    property var queue: []
    function run(args) {
        queue = queue.concat([args]);
        if (!typer.running) next();
    }
    function next() {
        if (queue.length === 0) return;
        typer.command = ["wtype"].concat(queue[0]);
        queue = queue.slice(1);
        typer.running = true;
    }
    Process { id: typer; onExited: root.next() }

    function type(text) {
        run(["--", root.shift ? text.toUpperCase() : text]);
        if (root.shift === 1) root.shift = 0;
        root.word = text.length === 1 && (text.toLowerCase() !== text.toUpperCase() || text === "'") ? root.word + text : "";
    }
    function key(name) {
        if (name === "shift") { root.shift = (root.shift + 1) % 3; return; }
        if (name === "hide") { root.open = false; return; }
        if (name.indexOf("page:") === 0) { root.page = name.slice(5); return; }
        if (name.indexOf("suggest:") === 0) { root.complete(name.slice(8)); return; }
        if (named[name]) run(["-k", named[name]]);
        root.word = name === "backspace" ? root.word.slice(0, -1) : "";
    }
    // Word suggestions, as the consoles offer: the word typed so far, completed from the system dictionary
    // when there is one. Picking one types the rest and a space.
    property string word: ""
    property var suggestions: []
    readonly property string dictionary: "/usr/share/dict/words"
    onWordChanged: { if (word.length >= 2 && root.hasDictionary) lookup.restart(); else suggestions = []; }
    onOpenChanged: { word = ""; suggestions = []; }
    // The dictionary is alphabetical, so a wider net is taken and the shortest words offered first; a name
    // (capitalised) is offered only for a capitalised start.
    Timer { id: lookup; interval: 60; onTriggered: { finder.command = ["grep", "-m", "400", "-i", "^" + root.word.replace(/[\\.^$*+?()[\]{}|]/g, ""), root.dictionary]; finder.running = true; } }
    Process {
        id: finder
        stdout: StdioCollector {
            onStreamFinished: {
                const w = root.word, upper = w.charAt(0) !== w.charAt(0).toLowerCase();
                const seen = {}, out = [];
                for (const c of text.trim().split("\n")) {
                    if (!c || c.indexOf("'") >= 0 || c.toLowerCase() === w.toLowerCase()) continue;
                    if (!upper && c.charAt(0) !== c.charAt(0).toLowerCase()) continue;
                    const word = w + c.slice(w.length);
                    if (!seen[word]) { seen[word] = true; out.push(word); }
                }
                root.suggestions = out.sort((a, b) => a.length - b.length).slice(0, 5);
            }
        }
    }
    function complete(w) {
        const rest = w.slice(root.word.length);
        if (rest) run(["--", (root.shift === 2 ? rest.toUpperCase() : rest) + " "]); else run(["--", " "]);
        root.word = ""; root.suggestions = [];
    }

    IpcHandler {
        target: "osk"
        function show(): void { root.show(); }
        function hide(): void { root.hide(); }
        function toggle(): void { root.toggle(); }
    }
}
