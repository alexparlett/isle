import QtQuick
import qs.theme

// A ring of parts, each [value, colour], drawn clockwise from the top with a hairline gap between.
Canvas {
    id: root
    property var parts: []
    property real thickness: 22
    property color track: Theme.hairline

    onPartsChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        const r = Math.min(width, height) / 2 - thickness / 2;
        const cx = width / 2, cy = height / 2;
        ctx.lineWidth = thickness;
        ctx.strokeStyle = String(track);
        ctx.beginPath();
        ctx.arc(cx, cy, r, 0, Math.PI * 2);
        ctx.stroke();

        const total = parts.reduce((s, p) => s + p[0], 0);
        if (!total) return;
        // The gap in radians that makes a two-pixel break on the ring.
        const gap = parts.filter(p => p[0] > 0).length > 1 ? 2 / r : 0;
        let a = -Math.PI / 2;
        for (const p of parts) {
            const sweep = p[0] / total * Math.PI * 2;
            if (sweep > gap) {
                ctx.strokeStyle = String(p[1]);
                ctx.beginPath();
                ctx.arc(cx, cy, r, a + gap / 2, a + sweep - gap / 2);
                ctx.stroke();
            }
            a += sweep;
        }
    }
}
