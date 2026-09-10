pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// Keyboards and their profiles. Renders keymap.json into the Hyprland bind fragment and the xremap config.
Singleton {
    id: root

    property var keymap: ({ actions: [], media: [], macTerminalIds: [] })
    FileView {
        path: Qt.resolvedUrl("../keymap.json")
        blockLoading: true
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { root.keymap = JSON.parse(text()); root.render(); }
    }

    // Keyboards the compositor sees: [{ name, profile }]. A profile is "win" or "mac"; "auto" guesses from the name.
    property var devices: []
    Process {
        id: devs
        command: ["hyprctl", "-j", "devices"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const seen = {};
                    root.devices = JSON.parse(text).keyboards
                        .filter(k => k.name && !/virtual|hypr|xremap|power-button|sleep-button|lid-switch|video-bus|hdmi|hda-|pc-speaker/i.test(k.name) && !seen[k.name] && (seen[k.name] = true))
                        .map(k => ({ name: k.name, profile: root.profileFor(k.name) }));
                } catch (e) {}
            }
        }
    }
    function refreshDevices() { devs.running = true; }

    function profileFor(name) {
        const set = Prefs.p.keyboardProfiles[name];
        if (set === "win" || set === "mac") return set;
        return /keychron|apple|magic keyboard/i.test(name) ? "mac" : "win";
    }
    function setProfile(name, profile) {
        const p = Object.assign({}, Prefs.p.keyboardProfiles);
        if (profile === "auto") delete p[name]; else p[name] = profile;
        Prefs.p.keyboardProfiles = p;
        refreshDevices();
        render();
    }
    readonly property var macDevices: devices.filter(d => d.profile === "mac").map(d => d.name)

    // The compositor lists every interface a keyboard exposes, a mouse's media keys among them; one row per
    // physical device, named from the slug, and a profile set on the row lands on each of its interfaces.
    readonly property var groups: {
        const out = [], byBase = {};
        for (const d of devices) {
            const base = d.name.replace(/(-keyboard)?(-\d+)?$/, "");
            if (!byBase[base]) {
                const words = base.split("-").filter(Boolean).map(w => w.charAt(0).toUpperCase() + w.slice(1));
                const label = words.filter((w, i) => i === 0 || w.toLowerCase() !== words[i - 1].toLowerCase()).join(" ");
                byBase[base] = { name: base, label: label, members: [], profile: d.profile };
                out.push(byBase[base]);
            }
            byBase[base].members.push(d.name);
        }
        return out;
    }
    function setGroupProfile(g, profile) { for (const m of g.members) setProfile(m, profile); }

    // What the kernel says about each keyboard: bus, vendor, size from its key bitmap, a guess at the XKB model.
    // [{ name, slug, bus, vendor, vendorId, productId, size, model }]
    property var hardware: []
    Process {
        id: hw
        command: ["python3", Quickshell.shellDir + "/scripts/keyboards.py", Input.layouts.length ? Input.layouts[0].layout : "us"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.hardware = JSON.parse(text).filter(h => !/xremap|virtual|hypr/i.test(h.name)); } catch (e) {} } }
    }
    onDevicesChanged: hw.running = true
    // The device lines depend on this list, so the config follows it.
    onHardwareChanged: if (groups.length) Input.render()
    // The compositor's slug is the kernel name lowercased with every other character a dash; a group is that
    // slug without its trailing "-keyboard".
    function hardwareFor(g) { return hardware.find(h => h.slug === g.name || h.slug.replace(/-keyboard$/, "") === g.name) || null; }
    // The XKB model in force for a group: the one set, else the guess.
    function modelFor(g) {
        const set = (Prefs.p.keyboardModels || {})[g.name];
        if (set) return set;
        const h = hardwareFor(g);
        return h ? h.model : "pc105";
    }
    function setGroupModel(g, model) {
        const m = Object.assign({}, Prefs.p.keyboardModels || {});
        if (model) m[g.name] = model; else delete m[g.name];
        Prefs.p.keyboardModels = m;
        Input.render();
    }
    // hl.device lines for the compositor, one per interface, so each keyboard gets its model.
    // Layouts with a Macintosh variant in xkeyboard-config: the symbols where a Mac keyboard prints them.
    readonly property var macVariantLayouts: ["ara", "dk", "nl", "gb", "us", "fi", "fr", "de", "at", "is", "it", "jp", "no", "pt", "ru", "se"]
    // A Mac-profile keyboard takes each layout's Macintosh variant unless the layout already names one.
    function macVariantLua() {
        const ls = Input.layouts;
        if (!ls.some(l => !l.variant && macVariantLayouts.indexOf(l.layout) >= 0)) return "";
        return ", kb_layout = " + JSON.stringify(ls.map(l => l.layout).join(","))
             + ", kb_variant = " + JSON.stringify(ls.map(l => l.variant || (macVariantLayouts.indexOf(l.layout) >= 0 ? "mac" : "")).join(","));
    }
    function deviceLua() {
        let out = "";
        for (const g of groups) {
            const model = modelFor(g);
            for (const m of g.members) out += "hl.device({ name = " + JSON.stringify(m) + ", kb_model = " + JSON.stringify(model) + (profileFor(m) === "mac" ? macVariantLua() : "") + " })\n";
        }
        return out;
    }

    // --- overrides ------------------------------------------------------------------
    // prefs.keymapOverrides: action id -> chord. An overridden action binds that chord on both profiles.
    readonly property var overrides: Prefs.p.keymapOverrides
    readonly property var actions: keymap.actions.map(a => {
        const o = overrides[a.id];
        if (!o) return a;
        const c = Object.assign({}, a, { hypr: o, win: human(o, "win"), mac: human(o, "mac") });
        delete c.macHypr;
        return c;
    })
    function human(chord, profile) {
        const names = { space: "Space", Tab: "Tab", Return: "Enter", grave: "`", comma: ",", period: ".", slash: "/", minus: "-", equal: "=", Print: "PrtSc",
                        bracketleft: "[", bracketright: "]", semicolon: ";", apostrophe: "'", backslash: "\\", Prior: "PgUp", Next: "PgDn" };
        return chord.split("+").map(s => s.trim()).map(p => {
            if (p === "SUPER") return profile === "mac" ? "Cmd" : "Win";
            if (p === "SHIFT") return "Shift";
            if (p === "CTRL") return "Ctrl";
            if (p === "ALT") return profile === "mac" ? "Opt" : "Alt";
            return names[p] || (p.length === 1 ? p.toUpperCase() : p.charAt(0).toUpperCase() + p.slice(1));
        }).join(" ");
    }
    // The action already on a chord, if any, so Settings can refuse a clash.
    function usedBy(chord, exceptId) {
        const norm = c => c.toUpperCase().replace(/\s+/g, "");
        return actions.find(a => a.id !== exceptId && !a.range && (norm(a.hypr) === norm(chord) || (a.macHypr && norm(a.macHypr) === norm(chord)))) || null;
    }
    function setOverride(id, chord) {
        const o = Object.assign({}, overrides);
        if (chord) o[id] = chord; else delete o[id];
        Prefs.p.keymapOverrides = o;
        render(true);
    }
    // A Qt key event as a Hyprland chord, or null for a bare modifier or a key with no name.
    function chordFromEvent(event) {
        const mods = [];
        if (event.modifiers & Qt.MetaModifier) mods.push("SUPER");
        if (event.modifiers & Qt.ControlModifier) mods.push("CTRL");
        if (event.modifiers & Qt.AltModifier) mods.push("ALT");
        if (event.modifiers & Qt.ShiftModifier) mods.push("SHIFT");
        const k = event.key;
        if ([Qt.Key_Shift, Qt.Key_Control, Qt.Key_Alt, Qt.Key_Meta, Qt.Key_Super_L, Qt.Key_Super_R, Qt.Key_AltGr, Qt.Key_CapsLock].indexOf(k) >= 0) return null;
        const names = {};
        names[Qt.Key_Space] = "space"; names[Qt.Key_Tab] = "Tab"; names[Qt.Key_Backtab] = "Tab"; names[Qt.Key_Return] = "Return"; names[Qt.Key_Enter] = "Return";
        names[Qt.Key_Up] = "up"; names[Qt.Key_Down] = "down"; names[Qt.Key_Left] = "left"; names[Qt.Key_Right] = "right";
        names[Qt.Key_QuoteLeft] = "grave"; names[Qt.Key_AsciiTilde] = "grave"; names[Qt.Key_Comma] = "comma"; names[Qt.Key_Period] = "period"; names[Qt.Key_Slash] = "slash";
        names[Qt.Key_Minus] = "minus"; names[Qt.Key_Equal] = "equal"; names[Qt.Key_BracketLeft] = "bracketleft"; names[Qt.Key_BracketRight] = "bracketright";
        names[Qt.Key_Semicolon] = "semicolon"; names[Qt.Key_Apostrophe] = "apostrophe"; names[Qt.Key_Backslash] = "backslash";
        names[Qt.Key_Print] = "Print"; names[Qt.Key_Delete] = "Delete"; names[Qt.Key_Home] = "Home"; names[Qt.Key_End] = "End";
        names[Qt.Key_PageUp] = "Prior"; names[Qt.Key_PageDown] = "Next"; names[Qt.Key_Insert] = "Insert"; names[Qt.Key_Escape] = "Escape"; names[Qt.Key_Backspace] = "BackSpace";
        let name = names[k];
        if (!name && k >= Qt.Key_F1 && k <= Qt.Key_F12) name = "F" + (k - Qt.Key_F1 + 1);
        if (!name && k >= Qt.Key_A && k <= Qt.Key_Z) name = String.fromCharCode(k).toLowerCase();
        if (!name && k >= Qt.Key_0 && k <= Qt.Key_9) name = String.fromCharCode(k);
        if (!name) return null;
        return mods.concat([name]).join(" + ");
    }
    // While Settings records a chord the compositor's binds step aside, so Super chords reach the window.
    function setRecording(on) { Hyprland.dispatch(on ? "hl.dsp.submap(\"isle-record\")" : "hl.dsp.submap(\"reset\")"); }

    // --- rendering ------------------------------------------------------------------

    readonly property string hyprOut: Quickshell.shellDir + "/../hypr/generated/binds.lua"
    readonly property string xremapOut: Prefs.dir + "/xremap.yml"

    function luaFor(a, n) {
        if (a.lua) return a.lua.replace("{n}", n);
        if (a.cmd) return "hl.dsp.exec_cmd(" + JSON.stringify(a.cmd) + ")";
        return "ipc(" + JSON.stringify(a.ipc[0]) + ", " + JSON.stringify(a.ipc[1]) + (a.ipc[2] !== undefined ? ", " + JSON.stringify(a.ipc[2]) : "") + ")";
    }

    function renderHypr() {
        let out = "-- Rendered from shell/keymap.json by the Keyboard service. Edit the source, not this.\n";
        out += "local ipc, terminal, fileManager = ...\n";
        if (Prefs.p.terminal) out += "terminal = " + JSON.stringify(Prefs.p.terminal) + "\n";
        out += "\n";
        for (const a of actions) {
            const chords = [a.hypr];
            if (a.macHypr) chords.push(a.macHypr);
            for (const chord of chords) {
                if (a.range) {
                    out += "for n = 1, " + a.range + " do hl.bind(" + JSON.stringify(chord).replace("{n}", "\" .. n .. \"") + ", " + luaFor(a, "n") + ") end\n";
                } else {
                    out += "hl.bind(" + JSON.stringify(chord) + ", " + luaFor(a, "") + (a.opts ? ", " + a.opts : "") + ")\n";
                }
            }
        }
        out += "\n-- The switcher commits when the modifier is released.\n";
        // After a Super+Tab the compositor suppresses a plain release bind on the modifier; a non-consuming,
        // transparent one with the modifier in its mask still fires.
        for (const [mod, keys] of [["SUPER", ["Super_L", "Super_R"]], ["ALT", ["Alt_L", "Alt_R"]]])
            for (const k of keys) for (const chord of [mod + " + " + k, mod + " + SHIFT + " + k])
                out += "hl.bind(\"" + chord + "\", ipc(\"switcher\", \"commit\"), { release = true, non_consuming = true, transparent = true })\n";
        out += "hl.bind(\"Escape\", ipc(\"surfaces\", \"closeAll\"), { transparent = true, non_consuming = true })\n\n";
        for (const m of keymap.media) {
            out += "hl.bind(" + JSON.stringify(m.key) + ", hl.dsp.exec_cmd(" + JSON.stringify(m.cmd) + "), { locked = true" + (m.repeat ? ", repeating = true" : "") + " })\n";
        }
        return out.replace(/\"\" \.\. n \.\. \"\"/g, "n");
    }

    // The Mac profile: Cmd+key reaches apps as Ctrl+key, except the chords the shell owns, which stay Super.
    function xremapChord(hypr) {
        // "SUPER + SHIFT + 4" -> "Super-Shift-4"
        return hypr.split("+").map(s => s.trim()).map(p => ({ SUPER: "Super", SHIFT: "Shift", CTRL: "C", ALT: "Alt" })[p] || p.toLowerCase()).join("-");
    }
    function renderXremap() {
        const owned = {};
        for (const a of actions) {
            if (a.profile === "win") continue;
            for (const chord of [a.macHypr || a.hypr]) {
                if (a.range) for (let n = 1; n <= a.range; n++) owned[xremapChord(chord.replace("{n}", n))] = true;
                else owned[xremapChord(chord)] = true;
            }
        }
        const keys = "abcdefghijklmnopqrstuvwxyz0123456789".split("").concat(["minus", "equal", "leftbrace", "rightbrace", "semicolon", "apostrophe", "comma", "dot", "slash", "backslash"]);
        const remap = {};
        for (const k of keys) {
            for (const mods of ["Super-", "Super-Shift-"]) {
                const from = mods + k;
                if (owned[from] || owned[from.replace("dot", ".")]) continue;
                remap[from] = from.replace("Super-", "C-");
            }
        }
        // Workspaces and spaces, as macOS has them.
        for (let n = 1; n <= 9; n++) { remap["C-" + n] = "Super-" + n; remap["C-Shift-" + n] = "Super-Shift-" + n; }
        Object.assign(remap, { "C-Left": "Super-Left", "C-Right": "Super-Right", "C-Up": "Super-Up",
                               "Alt-Left": "C-Left", "Alt-Right": "C-Right", "Alt-Backspace": "C-Backspace",
                               "Super-Left": "Home", "Super-Right": "End", "Super-Up": "C-Home", "Super-Down": "C-End", "Super-Backspace": "C-Shift-Backspace" });
        const term = { "Super-c": "C-Shift-c", "Super-v": "C-Shift-v", "Super-t": "C-Shift-t", "Super-w": "C-Shift-w", "Super-n": "C-Shift-n", "Super-f": "C-Shift-f" };

        const devices = macDevices.length ? macDevices : ["Keychron Q6 Max"];
        const dev = "    device:\n      only:\n" + devices.map(d => "        - " + JSON.stringify(d)).join("\n") + "\n";
        let out = "# Rendered from shell/keymap.json by the Keyboard service. Edit the source, not this.\n";
        out += "# The Mac profile, for the keyboards listed under device. Cmd reaches apps as Ctrl; the shell's chords stay Super.\n";
        out += "keymap:\n";
        out += "  - name: mac terminals\n" + dev + "    application:\n      only:\n" + keymap.macTerminalIds.map(i => "        - " + JSON.stringify(i)).join("\n") + "\n    remap:\n";
        for (const k in term) out += "      " + k + ": " + term[k] + "\n";
        out += "  - name: mac\n" + dev + "    remap:\n";
        for (const k in remap) out += "      " + k + ": " + remap[k] + "\n";
        return out;
    }

    Process { id: writer; onExited: if (root.reloadAfter) { root.reloadAfter = false; reloader.running = true; } }
    // After a change from Settings: the compositor rereads the fragment, xremap its config.
    property bool reloadAfter: false
    Process { id: reloader; command: ["sh", "-c", "hyprctl reload >/dev/null; systemctl --user try-restart xremap.service 2>/dev/null"] }
    function render(reload) {
        if (!keymap.actions.length) return;
        const hypr = renderHypr(), xremap = renderXremap();
        if (reload) reloadAfter = true;
        writer.command = ["sh", "-c", "printf '%s' \"$1\" > \"$2\" && printf '%s' \"$3\" > \"$4\"", "_", hypr, hyprOut, xremap, xremapOut];
        writer.running = true;
    }

    IpcHandler {
        target: "keyboard"
        function render(): void { root.render(); }
    }
}
