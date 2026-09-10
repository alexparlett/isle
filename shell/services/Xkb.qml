pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// XKB's registry: layouts with their variants, options by group, with the names people read.
Singleton {
    id: root
    property var layouts: []
    property var groups: []
    // [{ name, description, vendor }], Generic first.
    property var models: []
    Process {
        command: ["python3", Quickshell.shellDir + "/scripts/xkb.py"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { const d = JSON.parse(text); root.layouts = d.layouts || []; root.groups = d.groups || []; root.models = d.models || []; } catch (e) {} } }
    }
    function modelName(code) { const m = models.find(x => x.name === code); return m ? m.description : code; }
    function layoutName(code) { const l = layouts.find(x => x.name === code); return l ? l.description : code; }
    function variantName(code, v) { const l = layouts.find(x => x.name === code); const x = l && l.variants.find(y => y.name === v); return x ? x.description : (v || ""); }
    function variantsOf(code) { const l = layouts.find(x => x.name === code); return l ? l.variants : []; }
    function optionsOf(group) { const g = groups.find(x => x.name === group); return g ? g.options : []; }
}
