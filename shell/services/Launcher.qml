pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The launcher's sources. A query with no prefix searches apps, windows and shell actions, and offers the web;
// ">" runs a command, "=" calculates, "/" finds files, ":" searches the clipboard, "@" windows only, "*" the
// password manager's vault only.
// Every result has `run`; some have `alt`, the secondary action on Shift+Enter, and `extra`, the third on Ctrl+Enter.
Singleton {
    id: root

    property string query: ""
    // [{ kind, title, subtitle, icon, glyph, run, alt, altLabel, extra, extraLabel }]
    property var results: []

    // Set by refresh() from the query, not bound: a binding would still be stale inside onQueryChanged.
    property string mode: ""
    property string term: ""

    onQueryChanged: refresh()

    function refresh() {
        mode = query.length && ">=/:@*".indexOf(query[0]) >= 0 ? query[0] : "";
        term = (mode ? query.slice(1) : query).trim();
        switch (mode) {
        case ">": results = runResults(); break;
        case "=": calc(); break;
        case "/": findFiles(); break;
        case ":": clipboard(); break;
        case "@": results = windowResults(term); break;
        case "*": results = vaultResults(term, 12); break;
        // The shell's own windows, modes and power actions first: "set" is Settings before any app.
        default: results = shellResults(term).concat(appResults(term), settingsResults(term), windowResults(term, 3), vaultResults(term, 3), webResult(term));
        }
    }

    // --- scoring --------------------------------------------------------------

    // Subsequence match; prefix and word-start hits score higher. 0 means no match.
    function score(text, q) {
        if (!q) return 1;
        const t = text.toLowerCase(), s = q.toLowerCase();
        if (t.indexOf(s) === 0) return 100 - t.length * 0.1;
        const w = t.indexOf(" " + s);
        if (w >= 0) return 80 - w * 0.1;
        const i = t.indexOf(s);
        if (i >= 0) return 60 - i * 0.5;
        let ti = 0, hits = 0;
        for (const ch of s) { const j = t.indexOf(ch, ti); if (j < 0) return 0; hits += j === ti ? 2 : 1; ti = j + 1; }
        return 10 + hits;
    }

    // --- apps -------------------------------------------------------------------

    // Launch counts, so what you use rises.
    function used(id) { return (Prefs.p.launchCounts || {})[id] || 0; }
    function bump(id) { const c = Object.assign({}, Prefs.p.launchCounts || {}); c[id] = (c[id] || 0) + 1; Prefs.p.launchCounts = c; }

    function appResults(q) {
        const out = [];
        for (const e of DesktopEntries.applications.values) {
            if (e.noDisplay) continue;
            let s = Math.max(score(e.name, q), score(e.genericName || "", q) * 0.8, (e.keywords || []).reduce((m, k) => Math.max(m, score(k, q) * 0.7), 0));
            if (s <= 0) continue;
            s += Math.min(20, used(e.id) * 2);
            out.push({ kind: "app", title: e.name, subtitle: e.genericName || e.comment || "Application", icon: Quickshell.iconPath(e.icon, "application-x-executable"), score: s,
                       run: () => { bump(e.id); e.execute(); },
                       alt: e.runInTerminal ? null : () => { bump(e.id); Compositor.exec("kitty -e " + JSON.stringify(e.command.join(" "))); }, altLabel: "in a terminal" });
        }
        out.sort((a, b) => b.score - a.score || a.title.localeCompare(b.title));
        return out.slice(0, q ? 8 : 12);
    }

    // --- the vault -----------------------------------------------------------------
    // The password manager's logins by title: Enter copies the password, Shift+Enter the username, Ctrl+Enter the code.
    function vaultResults(q, limit) {
        if (!Vault.any) return mode === "*" ? [{ kind: "hint", title: "No password manager's tool is installed", subtitle: "Settings › Passwords", glyph: "key-round", run: () => Surfaces.showSettings("passwords") }] : [];
        if (!Vault.ready.length) {
            if (mode !== "*") return [];
            // The first waiting provider's own primary action, from the launcher: sign in, unlock, whatever it names.
            const w = Vault.waiting[0], a = w ? (w.impl.actions.find(x => x.primary && !x.input) || null) : null;
            return [{ kind: "hint", title: w ? w.name + "  ·  " + w.impl.state : Vault.installed.map(p => p.name + " is switched off").join(", "), subtitle: a ? a.label : "Keychain", glyph: "key-round", run: () => { if (a) w.impl.act(a.id); else Surfaces.show("keychain"); } }];
        }
        if (!q) return mode === "*" ? [{ kind: "hint", title: "Search " + Vault.ready.map(p => p.name).join(" and "), subtitle: "* github   ·   Enter copies the password, Shift+Enter the username, Ctrl+Enter the code", glyph: "key-round", run: () => {} }] : [];
        const out = [];
        for (const it of Vault.items) {
            const s = Math.max(score(it.title, q), score(it.vault, q) * 0.5);
            if (s < 60) continue;
            const login = it.type === "login";
            out.push({ kind: "vault", title: it.title, subtitle: it.providerName + "  ·  " + it.vault + (login ? "  ·  Enter copies the password" : "  ·  " + it.type.replace("_", " ")), glyph: "key-round", score: s,
                       run: () => Vault.copy(it, login ? "password" : "note"),
                       alt: login ? () => Vault.copy(it, "username") : null, altLabel: "copy username",
                       extra: login ? () => Vault.copy(it, "totp") : null, extraLabel: "copy code" });
        }
        out.sort((a, b) => b.score - a.score || a.title.localeCompare(b.title));
        return out.slice(0, limit);
    }

    // --- windows ------------------------------------------------------------------

    function windowResults(q, limit) {
        const out = [];
        for (const g of Windows.groups) for (const a of g.apps) {
            const s = Math.max(score(a.title || "", q), score(a.appId || "", q));
            if (s <= 0 || (!q && limit)) continue;
            out.push({ kind: "window", title: a.title || a.appId, subtitle: "Window · " + a.appId + " · workspace " + g.id, icon: a.icon, score: s * 0.9, run: () => Windows.focus(a),
                       alt: () => Windows.closeWindow(a), altLabel: "close" });
        }
        out.sort((a, b) => b.score - a.score);
        return limit ? out.slice(0, limit) : out;
    }

    // --- the shell and power ----------------------------------------------------------

    readonly property var shellActions: [
        { title: "Settings", glyph: "sliders-horizontal", kind: "Shell", key: "settings", run: () => Surfaces.show("settings") },
        { title: "Keychain", glyph: "key-round", kind: "Shell", run: () => Surfaces.show("keychain") },
        { title: "Monitor", glyph: "cpu", kind: "Shell", key: "monitor", run: () => Surfaces.show("monitor") },
        { title: "Dashboard", glyph: "layout-grid", kind: "Shell", key: "dashboard", run: () => { Surfaces.dashboard = true; } },
        { title: "Mission Control", glyph: "app-window", kind: "Shell", key: "overview", words: "desktops workspaces windows", run: () => { Surfaces.overview = true; } },
        { title: "Files", glyph: "folder", kind: "Shell", key: "filesGui", run: () => Compositor.exec("thunar") },
        { title: "Files in the terminal", glyph: "terminal", kind: "Shell", key: "files", run: () => Compositor.exec("kitty -e yazi") },
        { title: "Do not disturb", glyph: "bell-off", kind: "Mode", key: "dnd", run: () => Notifications.setDnd(!Notifications.dnd) },
        { title: "Game mode", glyph: "gamepad-2", kind: "Mode", key: "gameMode", run: () => Modes.toggle("game") },
        { title: "Big Picture", glyph: "gamepad-2", kind: "Mode", key: "bigPicture", run: () => Modes.set("bigpicture") },
        { title: "Connect VPN", glyph: "shield", kind: "Network", when: () => Vpn.ready && !Vpn.active, run: () => Vpn.toggle() },
        { title: "Disconnect VPN", glyph: "shield-off", kind: "Network", when: () => Vpn.active !== null, run: () => Vpn.toggle() },
        { title: "Lock", glyph: "lock", kind: "Power", key: "lock", words: "lock screen", run: () => Session.lock() },
        { title: "Sleep", glyph: "moon", kind: "Power", words: "suspend", run: () => Session.sleep() },
        { title: "Hibernate", glyph: "moon", kind: "Power", words: "suspend to disk", run: () => Session.hibernate() },
        { title: "Restart", glyph: "rotate-cw", kind: "Power", words: "reboot", run: () => Session.restart() },
        { title: "Shut down", glyph: "power", kind: "Power", words: "power off poweroff halt", run: () => Session.shutdown() },
        { title: "Log out", glyph: "log-out", kind: "Power", key: "logout", words: "logout exit session", run: () => Session.logout() },
        { title: "Power menu", glyph: "power", kind: "Power", key: "power", run: () => { Surfaces.power = true; } },
    ]
    // The chord the keyboard in use presses for a keymap action, or "".
    function chord(id) {
        const a = id ? Keyboard.actions.find(a => a.id === id) : null;
        if (!a) return "";
        return (Keyboard.macDevices.length ? a.mac : a.win) || a.win || a.mac || "";
    }
    // The title, the group ("power" lists every power action) or a synonym.
    function shellResults(q) {
        if (!q) return [];
        return shellActions.filter(a => !a.when || a.when())
            .filter(a => score(a.title, q) >= 60 || score(a.kind, q) >= 80 || (a.words || "").split(" ").some(w => score(w, q) >= 80))
            .map(a => ({ kind: "power", title: a.title, subtitle: a.kind, glyph: a.glyph, chord: chord(a.key), run: a.run }));
    }
    // Settings pages by name, each opening straight to that page.
    function settingsResults(q) {
        if (!q || q.length < 2) return [];
        return SettingsIndex.pages.filter(p => p.id && score(p.label, q) >= 60)
            .map(p => ({ kind: "settings", title: p.label, subtitle: "Settings", glyph: p.glyph, run: () => Surfaces.showSettings(p.id) }));
    }

    // The web, always last, so a query that matches nothing still goes somewhere.
    function webResult(q) {
        if (!q || q.length < 2) return [];
        return [{ kind: "web", title: "Search the web for “" + q + "”", subtitle: "DuckDuckGo", glyph: "globe",
                  run: () => Compositor.exec("xdg-open " + JSON.stringify("https://duckduckgo.com/?q=" + encodeURIComponent(q))) }];
    }

    // --- run ------------------------------------------------------------------------

    function runResults() {
        if (!term) return [{ kind: "hint", title: "Run a command", subtitle: "> ls -la   ·   Shift+Enter runs it in a terminal", glyph: "terminal", run: () => {} }];
        return [{ kind: "run", title: term, subtitle: "Run  ·  Shift+Enter in a terminal", glyph: "terminal", run: () => Compositor.exec(term),
                  alt: () => Compositor.exec("kitty --hold -e sh -c " + JSON.stringify(term)), altLabel: "in a terminal" }];
    }

    // --- calculator -----------------------------------------------------------------

    Process {
        id: qalc
        stdout: StdioCollector {
            onStreamFinished: {
                const v = text.trim();
                if (root.mode !== "=") return;
                root.results = v ? [{ kind: "calc", title: v, subtitle: root.term + "  ·  Enter copies", glyph: "calculator", run: () => root.copy(v) }] : [];
            }
        }
    }
    function calc() {
        if (!term) { results = [{ kind: "hint", title: "Calculate", subtitle: "= 2 * (3 + 4)   ·   = 5 km to miles", glyph: "calculator", run: () => {} }]; return; }
        qalc.running = false;
        qalc.command = ["qalc", "-t", term];
        qalc.running = true;
    }

    // --- files -----------------------------------------------------------------------

    Process {
        id: fd
        stdout: StdioCollector {
            onStreamFinished: {
                if (root.mode !== "/") return;
                root.results = text.trim().split("\n").filter(l => l).map(p => ({
                    kind: "file", title: p.split("/").pop(), subtitle: p.replace(root.home, "~") + "  ·  Shift+Enter opens the folder", glyph: "folder",
                    run: () => Compositor.exec("xdg-open " + JSON.stringify(p)),
                    alt: () => Compositor.exec("xdg-open " + JSON.stringify(p.slice(0, p.lastIndexOf("/")) || "/")), altLabel: "open folder" }));
            }
        }
    }
    readonly property string home: Quickshell.env("HOME")
    function findFiles() {
        if (!term) { results = [{ kind: "hint", title: "Find files", subtitle: "/ invoice 2026", glyph: "folder", run: () => {} }]; return; }
        results = [];
        fd.running = false;
        fd.command = ["fd", "--max-results", "20", "-i", "-H", "-E", ".cache", "-E", ".git", "-E", "node_modules", "-p", term.split(" ").join(".*"), home];
        fd.running = true;
    }

    // --- clipboard --------------------------------------------------------------------

    readonly property string thumbDir: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle-clip"
    Process { command: ["mkdir", "-p", root.thumbDir]; running: true }
    // Thumbnails arrive after the rows; re-publishing the results makes the images load them.
    Process { id: thumber; onExited: { if (root.mode === ":") root.results = root.results.map(r => Object.assign({}, r, { thumb: r.thumb ? r.thumb.split("?")[0] + "?" + Date.now() : "" })); } }

    Process {
        id: cliphist
        stdout: StdioCollector {
            onStreamFinished: {
                if (root.mode !== ":") return;
                const out = root.pinnedResults(), wanted = [];
                for (const line of text.split("\n")) {
                    const tab = line.indexOf("\t");
                    if (tab < 0) continue;
                    const id = line.slice(0, tab), body = line.slice(tab + 1);
                    if (root.term && score(body, root.term) <= 0) continue;
                    const image = body.indexOf("[[ binary data") === 0;
                    const m = image ? body.match(/(\d+)x(\d+)\s+(\w+)/) : null;
                    if (image) wanted.push(id);
                    out.push({ kind: "clip", id: id, title: image ? "Image" + (m ? "  ·  " + m[1] + "×" + m[2] + " " + m[3] : "") : body.replace(/\s+/g, " ").slice(0, 120),
                               subtitle: image ? "Image" : "Text", glyph: "clipboard", thumb: image ? "file://" + root.thumbDir + "/" + id + ".png" : "",
                               // Copied, then pasted into the window that had focus before the launcher took it.
                               run: () => { const target = Windows.recent[0] || ""; Compositor.exec("sh -c " + JSON.stringify("cliphist decode " + id + " | wl-copy; sleep 0.25; " + Windows.pasteCommand(target))); },
                               alt: () => root.pin(id, image, body), altLabel: "pin",
                               extra: () => root.deleteClip(line), extraLabel: "delete" });
                }
                root.results = out.slice(0, 40);
                // Thumbnails for the images shown, decoded once each.
                if (wanted.length) {
                    thumber.command = ["sh", "-c", "mkdir -p \"$1\" && cd \"$1\" || exit 1; shift; for id in \"$@\"; do [ -s \"$id.png\" ] || cliphist decode \"$id\" > \"$id.png\"; done", "_", root.thumbDir].concat(wanted.slice(0, 30));
                    thumber.running = true;
                }
            }
        }
    }
    // Pins live in prefs: { text } or { image: path } under ~/.config/isle/pins. They lead the list.
    readonly property string pinDir: Prefs.dir + "/pins"
    function pinnedResults() {
        return Prefs.p.clipPins.filter(p => !root.term || score(p.text || "image", root.term) > 0).map(p => ({
            kind: "clip", pinned: true, title: p.image ? "Image" : (p.text || "").replace(/\s+/g, " ").slice(0, 120), subtitle: "Pinned",
            glyph: "pin", thumb: p.image ? "file://" + p.image : "",
            run: () => { const target = Windows.recent[0] || ""; Compositor.exec("sh -c " + JSON.stringify((p.image ? "wl-copy -t image/png < " + JSON.stringify(p.image) : "wl-copy " + JSON.stringify(p.text)) + "; sleep 0.25; " + Windows.pasteCommand(target))); },
            alt: () => root.unpin(p), altLabel: "unpin" }));
    }
    Process { id: pinner; onExited: root.clipboard() }
    function pin(id, image, body) {
        if (image) {
            const dest = pinDir + "/" + Date.now() + ".png";
            pinner.command = ["sh", "-c", "mkdir -p \"$1\" && cliphist decode \"$2\" > \"$3\"", "_", pinDir, id, dest];
            Prefs.p.clipPins = Prefs.p.clipPins.concat([{ image: dest }]);
            pinner.running = true;
        } else {
            if (Prefs.p.clipPins.some(p => p.text === body)) return;
            Prefs.p.clipPins = Prefs.p.clipPins.concat([{ text: body }]);
            clipboard();
        }
    }
    function unpin(p) {
        Prefs.p.clipPins = Prefs.p.clipPins.filter(q => q.text !== p.text || q.image !== p.image);
        if (p.image) { pinner.command = ["rm", "-f", p.image]; pinner.running = true; } else clipboard();
    }
    function clipboard() {
        results = pinnedResults();
        cliphist.running = false;
        cliphist.command = ["cliphist", "list"];
        cliphist.running = true;
    }
    Process { id: clipDeleter; onExited: root.clipboard() }
    function deleteClip(line) { clipDeleter.command = ["sh", "-c", "printf '%s\\n' \"$1\" | cliphist delete", "_", line]; clipDeleter.running = true; }

    Process { id: copier }
    function copy(text) { copier.command = ["wl-copy", text]; copier.running = true; }
}
