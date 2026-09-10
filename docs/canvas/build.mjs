// Builds the design canvas artboards (*.dc.html) from the tokens in DESIGN.md.
// node docs/canvas/build.mjs
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const out = dirname(fileURLToPath(import.meta.url));

const T = {
  ink: "#0A0B0D", glass: "rgba(18,19,22,.78)", raised: "#1B1C21", pressed: "#26272D",
  hair: "rgba(255,255,255,.08)", hairS: "rgba(255,255,255,.14)",
  text: "#F2F2F3", text2: "rgba(242,242,243,.62)", text3: "rgba(242,242,243,.38)",
  accent: "#7FA6FF", ok: "#6FCF97", warn: "#F2C94C", danger: "#FF6B6B", live: "#FF453A",
};

const paths = {
  wifi: '<path d="M12 20h.01"/><path d="M8.5 16.4a5 5 0 0 1 7 0"/><path d="M5 12.9a10 10 0 0 1 14 0"/><path d="M2 8.8a15 15 0 0 1 20 0"/>',
  bluetooth: '<path d="m7 7 10 10-5 5V2l5 5L7 17"/>',
  volume: '<path d="M11 5 6 9H2v6h4l5 4V5z"/><path d="M15.5 8.5a5 5 0 0 1 0 7"/><path d="M19 5a10 10 0 0 1 0 14"/>',
  volumex: '<path d="M11 5 6 9H2v6h4l5 4V5z"/><path d="m23 9-6 6"/><path d="m17 9 6 6"/>',
  bell: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
  belloff: '<path d="M8.7 3A6 6 0 0 1 18 8c0 4.5 1.3 7 2.4 8.4"/><path d="M6 8c0 7-3 9-3 9h14"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/><path d="m2 2 20 20"/>',
  power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/>',
  search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
  gamepad: '<line x1="6" y1="12" x2="10" y2="12"/><line x1="8" y1="10" x2="8" y2="14"/><line x1="15" y1="13" x2="15.01" y2="13"/><line x1="18" y1="11" x2="18.01" y2="11"/><rect x="2" y="6" width="20" height="12" rx="2"/>',
  target: '<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>',
  play: '<polygon points="6 3 20 12 6 21 6 3"/>',
  pause: '<rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/>',
  next: '<polygon points="5 4 15 12 5 20 5 4"/><line x1="19" y1="5" x2="19" y2="19"/>',
  prev: '<polygon points="19 20 9 12 19 4 19 20"/><line x1="5" y1="19" x2="5" y2="5"/>',
  stop: '<rect x="5" y="5" width="14" height="14" rx="2"/>',
  drive: '<line x1="22" y1="12" x2="2" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" y1="16" x2="6.01" y2="16"/><line x1="10" y1="16" x2="10.01" y2="16"/>',
  lock: '<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  key: '<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>',
  terminal: '<polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>',
  globe: '<circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>',
  message: '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>',
  code: '<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>',
  sliders: '<line x1="21" y1="4" x2="14" y2="4"/><line x1="10" y1="4" x2="3" y2="4"/><line x1="21" y1="12" x2="12" y2="12"/><line x1="8" y1="12" x2="3" y2="12"/><line x1="21" y1="20" x2="16" y2="20"/><line x1="12" y1="20" x2="3" y2="20"/><line x1="14" y1="2" x2="14" y2="6"/><line x1="8" y1="10" x2="8" y2="14"/><line x1="16" y1="18" x2="16" y2="22"/>',
  monitor: '<rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>',
  keyboard: '<rect x="2" y="4" width="20" height="16" rx="2"/><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10"/>',
  palette: '<circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.9 0 1.6-.7 1.6-1.7 0-.4-.2-.8-.4-1.1-.3-.3-.4-.7-.4-1.1a1.6 1.6 0 0 1 1.7-1.7h2c3 0 5.5-2.5 5.5-5.6C22 6 17.5 2 12 2z"/>',
  chevron: '<path d="m9 18 6-6-6-6"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  x: '<path d="M18 6 6 18M6 6l12 12"/>',
  camera: '<path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/><circle cx="12" cy="13" r="3"/>',
  video: '<path d="m22 8-6 4 6 4V8Z"/><rect x="2" y="6" width="14" height="12" rx="2"/>',
  pipette: '<path d="m2 22 1-1h3l9-9"/><path d="M3 21v-3l9-9"/><path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>',
  battery: '<rect x="2" y="7" width="16" height="10" rx="2"/><line x1="22" y1="11" x2="22" y2="13"/><rect x="4" y="9" width="9" height="6" fill="currentColor" stroke="none"/>',
  headphones: '<path d="M3 18v-6a9 9 0 0 1 18 0v6"/><path d="M21 19a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3zM3 19a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-2-2H3z"/>',
  folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
  calc: '<rect width="16" height="20" x="4" y="2" rx="2"/><line x1="8" x2="16" y1="6" y2="6"/><line x1="16" x2="16" y1="14" y2="18"/><path d="M16 10h.01M12 10h.01M8 10h.01M12 14h.01M8 14h.01M12 18h.01M8 18h.01"/>',
  clipboard: '<rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>',
  cpu: '<rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M15 2v2M15 20v2M2 15h2M2 9h2M20 15h2M20 9h2M9 2v2M9 20v2"/>',
  logout: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>',
  restart: '<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>',
  square: '<rect x="3" y="3" width="18" height="18" rx="2"/>',
  layout: '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 21V9"/>',
  info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>',
  eject: '<path d="m5 15 7-9 7 9z"/><path d="M5 19h14"/>',
  music: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
  steam: '<circle cx="12" cy="12" r="10"/><circle cx="15" cy="9" r="3"/><circle cx="8.5" cy="15.5" r="2.5"/><path d="m11 14 2-2"/>',
  rocket: '<path d="M4.5 16.5c-1.5 1.3-2 5-2 5s3.7-.5 5-2c.7-.8.7-2 0-2.8-.8-.7-2-.7-3 0z"/><path d="m12 15-3-3a22 22 0 0 1 2-3.9A12.7 12.7 0 0 1 22 2c0 2.7-.8 7.6-6 11a22.4 22.4 0 0 1-4 2z"/>',
  swords: '<path d="m14.5 17.5 3 3L21 17l-3-3"/><path d="M3 3l11 11"/><path d="m3 21 6-6"/><path d="M21 3l-8 8"/>',
  arrowLeft: '<path d="m12 19-7-7 7-7M19 12H5"/>',
};

const icon = (n, s = 16, c = "currentColor", w = 1.75) =>
  `<svg width="${s}" height="${s}" viewBox="0 0 24 24" fill="none" stroke="${c}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0">${paths[n]}</svg>`;

const css = `
  body { margin:0; font-family: Inter, system-ui, sans-serif; color:${T.text}; background:${T.ink}; font-size:13px; font-weight:500; -webkit-font-smoothing:antialiased; }
  a { color:${T.accent}; } a:hover { color:${T.text}; }
  .wall { position:relative; overflow:hidden; background:
    radial-gradient(90% 70% at 78% 115%, #1a2237 0%, rgba(26,34,55,0) 60%),
    radial-gradient(70% 55% at 8% -10%, #1a1626 0%, rgba(26,22,38,0) 60%),
    radial-gradient(40% 30% at 55% 40%, #10121a 0%, rgba(16,18,26,0) 70%), ${T.ink}; }
  .glass { background:${T.glass}; backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px); border:1px solid ${T.hair}; box-shadow: 0 12px 40px rgba(0,0,0,.45); }
  .tnum { font-variant-numeric: tabular-nums; }
  .mono { font-family: "JetBrains Mono", ui-monospace, monospace; }
  .t2 { color:${T.text2}; } .t3 { color:${T.text3}; }
  .row { display:flex; align-items:center; }
  .col { display:flex; flex-direction:column; }
`;

const head = (extra = "") => `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap">
  <style>${css}${extra}</style>
</helmet>`;
const tail = `</x-dc>
</body>
</html>
`;

// --- building blocks --------------------------------------------------------

const appIcon = (name, bg, size = 20) =>
  `<div style="width:${size}px;height:${size}px;border-radius:${Math.round(size * 0.28)}px;background:${bg};display:flex;align-items:center;justify-content:center;flex-shrink:0">${icon(name, Math.round(size * 0.6), "#fff", 2)}</div>`;

const apps = {
  kitty: () => appIcon("terminal", "#2B2D33"),
  firefox: () => appIcon("globe", "#E5642E"),
  code: () => appIcon("code", "#2F7BD6"),
  discord: () => appIcon("message", "#5865F2"),
  steam: () => appIcon("steam", "#1B2838"),
};

const dot = (c, s = 6) => `<div style="width:${s}px;height:${s}px;border-radius:999px;background:${c};flex-shrink:0"></div>`;

const pill = (inner, { h = 30, px = 12, gap = 10, extra = "" } = {}) =>
  `<div class="glass row" style="height:${h}px;padding:0 ${px}px;gap:${gap}px;border-radius:999px;box-sizing:border-box;${extra}">${inner}</div>`;

const islandRest = () => pill(
  `<div class="row" style="gap:5px">${dot(T.accent)}${dot(T.text3)}${dot(T.text3)}</div>
   <div class="tnum" style="font-size:13px;font-weight:500">14:32</div>
   <div class="row" style="gap:8px;color:${T.text2}">${icon("bluetooth", 14)}</div>`);

const slider = (pct, w = 120, glyph = null, val = null) =>
  `<div class="row" style="gap:8px;width:${w}px">${glyph ? `<span style="color:${T.text2}">${icon(glyph, 14)}</span>` : ""}
    <div style="flex:1;height:4px;border-radius:2px;background:${T.hairS};position:relative"><div style="position:absolute;left:0;top:0;bottom:0;width:${pct}%;border-radius:2px;background:${T.accent}"></div></div>
    ${val !== null ? `<span class="mono tnum t2" style="font-size:11px;width:22px;text-align:right">${val}</span>` : ""}</div>`;

const wsGroup = (icons, active) =>
  `<div class="row" style="gap:6px;height:32px;padding:0 8px;border-radius:10px;background:${active ? T.raised : "transparent"};border:1px solid ${active ? T.hairS : "transparent"}">${icons.join("")}</div>`;



// Hover, three options. "Air" is the one used in the sheets.
const wsIcons = (groups, activeIdx, size = 18) => `<div class="row" style="gap:12px">${groups.map((g, i) =>
  `<div class="row" style="gap:5px;opacity:${i === activeIdx ? 1 : 0.42}">${g.map(n => apps[n]()).join("")}</div>`).join("")}</div>`;

const islandHoverAir = () => `<div class="glass row" style="height:44px;padding:0 16px 0 12px;gap:24px;border-radius:22px;box-sizing:border-box">
  ${wsIcons([["kitty", "code"], ["firefox"], ["discord", "steam"]], 0)}
  <div class="row" style="gap:8px"><span class="tnum" style="font-size:15px;font-weight:600">14:32</span><span class="t3">·</span><span class="t2" style="font-size:13px">Tue 9 Sep</span></div>
  <div class="row" style="gap:14px;color:${T.text2}">${icon("bluetooth", 15)}${icon("volume", 15)}<span class="row" style="gap:4px">${icon("bell", 15)}<span class="tnum" style="font-size:12px;font-weight:600;color:${T.text}">3</span></span></div>
</div>`;



const islandHover = islandHoverAir;

function iconBtn(n, on) {
  return `<div style="width:28px;height:28px;border-radius:14px;display:flex;align-items:center;justify-content:center;background:${on ? T.accent : T.raised};color:${on ? T.ink : T.text2};border:1px solid ${on ? "transparent" : T.hair}">${icon(n, 15, "currentColor", 2)}</div>`;
}

const islandNotif = () => `<div class="glass row" style="height:52px;padding:0 16px 0 10px;gap:12px;border-radius:26px;box-sizing:border-box;width:380px">
  ${appIcon("message", "#5865F2", 30)}
  <div class="col" style="flex:1;gap:1px;min-width:0">
    <div class="row" style="justify-content:space-between"><span style="font-size:13px;font-weight:600">Discord</span><span class="t3" style="font-size:11px">now</span></div>
    <div class="t2" style="font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">sam: ready when you are, lobby's up</div>
  </div>
</div>`;

const islandOsd = () => pill(`<span style="color:${T.text2}">${icon("volume", 14)}</span>${slider(64, 120)}<span class="mono tnum t2" style="font-size:11px">64</span>`, { gap: 10 });

const islandMedia = () => pill(
  `<div style="width:20px;height:20px;border-radius:5px;background:linear-gradient(135deg,#7A4FBF,#E2A15A);flex-shrink:0"></div>
   <div class="row" style="gap:6px;font-size:13px"><span>Nightcall</span><span class="t3">·</span><span class="t2">Kavinsky</span></div>
   <span style="color:${T.text2}">${icon("pause", 13, "currentColor", 2)}</span>`);

const islandRec = () => pill(
  `${dot(T.live, 8)}<span class="mono tnum" style="font-size:12px">00:42</span><span class="t3">·</span><span class="t2" style="font-size:12px">Recording</span><span style="color:${T.text}">${icon("stop", 13, "currentColor", 2)}</span>`);

const islandDrive = () => pill(
  `<span style="color:${T.text2}">${icon("drive", 14)}</span><span>SanDisk 64 GB</span>
   <div class="row" style="gap:6px;margin-left:4px">
     <div style="height:20px;padding:0 9px;border-radius:10px;background:${T.accent};color:${T.ink};font-size:11px;font-weight:600;display:flex;align-items:center">Mount</div>
     <div style="height:20px;padding:0 9px;border-radius:10px;background:${T.raised};border:1px solid ${T.hair};font-size:11px;font-weight:600;display:flex;align-items:center">Eject</div>
   </div>`);

const islandFocus = () => pill(
  `<span style="color:${T.accent}">${icon("target", 14)}</span><span class="tnum" style="font-size:13px">14:32</span><span class="t3">·</span><span class="t2 tnum" style="font-size:12px">Focus 24:10</span>`);

const islandAuth = () => pill(
  `<span style="color:${T.warn}">${icon("key", 14)}</span><span>Authentication required</span><span class="t2" style="font-size:12px">LACT</span>`);

// A desktop frame: wallpaper, two tiled windows, the island, plus whatever surface.
const desktop = (surface = "", { island = islandRest(), windows = true, dim = false, blur = false } = {}) => `
<div class="wall" style="width:1280px;height:560px;font-size:13px">
  ${windows ? `
  <div style="position:absolute;inset:0;padding:52px 8px 8px;display:flex;gap:8px;box-sizing:border-box;${blur ? "filter:blur(14px)" : ""}">
    <div style="flex:1.4;border-radius:10px;background:#111216;border:1px solid ${T.hairS};overflow:hidden;position:relative">
      <div class="mono" style="padding:14px 16px;font-size:12px;line-height:19px;color:${T.text2}">
        <div><span style="color:${T.accent}">~/dev/isle</span> <span class="t3">›</span> qs -c isle</div>
        <div class="t3">[isle] island mounted on DP-1</div>
        <div class="t3">[isle] pipewire: 2 sinks, default WH-1000XM5</div>
        <div class="t3">[isle] networking: ethernet, connected</div>
        <div><span style="color:${T.accent}">~/dev/isle</span> <span class="t3">›</span> <span style="display:inline-block;width:7px;height:14px;background:${T.text};vertical-align:-2px"></span></div>
      </div>
    </div>
    <div style="flex:1;border-radius:10px;background:#131418;border:1px solid ${T.hair};overflow:hidden">
      <div class="row" style="height:36px;padding:0 12px;gap:8px;border-bottom:1px solid ${T.hair}">
        ${apps.firefox()}<div style="flex:1;height:22px;border-radius:6px;background:${T.raised};border:1px solid ${T.hair}"></div>
      </div>
      <div style="padding:20px 24px;display:flex;flex-direction:column;gap:10px">
        <div style="width:60%;height:14px;border-radius:4px;background:${T.raised}"></div>
        <div style="width:90%;height:8px;border-radius:4px;background:${T.raised};opacity:.7"></div>
        <div style="width:85%;height:8px;border-radius:4px;background:${T.raised};opacity:.7"></div>
        <div style="width:70%;height:8px;border-radius:4px;background:${T.raised};opacity:.7"></div>
      </div>
    </div>
  </div>` : ""}
  ${dim ? `<div style="position:absolute;inset:0;background:rgba(10,11,13,.55)"></div>` : ""}
  ${island ? `<div style="position:absolute;top:12px;left:0;right:0;display:flex;justify-content:center;pointer-events:none">${island}</div>` : ""}
  ${surface}
</div>`;

const files = {};

// --- Main: the desktop at rest ---------------------------------------------
files["Main.dc.html"] = head() + desktop() + tail;

// --- Island states, 1:1 -----------------------------------------------------
const stateRow = (label, sub, el) => `
<div style="display:grid;grid-template-columns:150px minmax(0,1fr);gap:24px;align-items:center;padding:14px 0;border-bottom:1px solid ${T.hair}">
  <div class="col" style="gap:2px"><div style="font-size:13px;font-weight:600">${label}</div><div class="t3" style="font-size:11px;font-weight:500">${sub}</div></div>
  <div style="display:flex;justify-content:center">${el}</div>
</div>`;
files["IslandStates.dc.html"] = head() + `
<div class="wall" style="width:960px;height:820px;padding:20px 28px;box-sizing:border-box">
  <div style="font-size:15px;font-weight:600;margin-bottom:4px">The island</div>
  <div class="t2" style="font-size:12px;margin-bottom:8px">Top centre of the focused monitor. Every other surface grows out of it.</div>
  ${stateRow("Rest", "30px · dots, clock, glyphs only when they say something", islandRest())}
  ${stateRow("Hover", "the pill unfolds into the control panel; this is its top band (see Control panel)", islandHover())}
  ${stateRow("Notification", "5s, click opens, swipe dismisses", islandNotif())}
  ${stateRow("Volume", "the OSD; brightness the same", islandOsd())}
  ${stateRow("Media", "art, title, transport", islandMedia())}
  ${stateRow("Recording", "live dot, timer, stop", islandRec())}
  ${stateRow("Drive inserted", "mount or eject in place", islandDrive())}
  ${stateRow("Focus mode", "clock and timer, nothing else", islandFocus())}
  ${stateRow("Auth pending", "click opens the dialog", islandAuth())}
  ${stateRow("Game mode", "island unmapped; 4px hot zone at the top edge peeks it", `<div style="width:380px;height:30px;position:relative"><div style="position:absolute;left:0;right:0;top:0;height:4px;border-radius:2px;background:${T.hairS}"></div><div class="t3" style="position:absolute;left:0;right:0;top:12px;text-align:center;font-size:11px">nothing on screen</div></div>`)}
</div>` + tail;



// --- Shared cards (control panel, dashboard) ---------------------------------------------------------
const toggle = (n, label, sub, on) => `
<div class="row" style="gap:10px;height:56px;padding:0 12px;border-radius:14px;background:${on ? T.accent : T.raised};color:${on ? T.ink : T.text};border:1px solid ${on ? "transparent" : T.hair};box-sizing:border-box">
  ${icon(n, 18, "currentColor", 2)}
  <div class="col" style="gap:1px;min-width:0"><div style="font-size:13px;font-weight:600">${label}</div><div style="font-size:11px;opacity:.7;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${sub}</div></div>
</div>`;

// --- Notifications ----------------------------------------------------
const notifRow = (ic, app, body, time, last = false) => `
<div class="row" style="gap:10px;padding:10px 4px;${last ? "" : `border-bottom:1px solid ${T.hair}`}">
  ${ic}
  <div class="col" style="flex:1;gap:1px;min-width:0"><div class="row" style="justify-content:space-between"><span style="font-size:13px;font-weight:600">${app}</span><span class="t3" style="font-size:11px">${time}</span></div><div class="t2" style="font-size:12px">${body}</div></div>
</div>`;


// --- Dashboard --------------------------------------------------------------
const meter = (label, val, pct) => `<div class="col" style="gap:6px;flex:1;min-width:0"><div class="row" style="justify-content:space-between"><span class="t3" style="font-size:11px;font-weight:600">${label}</span><span class="mono tnum" style="font-size:11px;color:${T.text2}">${val}</span></div><div style="height:3px;border-radius:2px;background:${T.hairS}"><div style="width:${pct}%;height:100%;border-radius:2px;background:${T.text2}"></div></div></div>`;
const meters = () => `${meter("CPU", "12%", 12)}${meter("GPU", "38% · 54°", 38)}${meter("Memory", "9.2 / 64 GB", 14)}${meter("Network", "↓ 1.2 MB/s", 30)}`;
const trayBtn = (n, c = T.text2) => `<div style="width:30px;height:30px;border-radius:9px;display:flex;align-items:center;justify-content:center;color:${c}">${icon(n, 15)}</div>`;
const tray = () => `<div class="row" style="gap:2px">
  <div style="opacity:.6">${trayBtn("steam")}</div><div style="opacity:.6">${trayBtn("message")}</div>
  <div style="width:1px;height:18px;background:${T.hairS};margin:0 6px"></div>
  ${trayBtn("lock")}${trayBtn("sliders")}${trayBtn("power")}</div>`;
const week = () => `<div style="display:grid;grid-template-columns:repeat(7, minmax(0, 1fr));gap:4px">${[["M", 7], ["T", 8], ["W", 9], ["T", 10], ["F", 11], ["S", 12], ["S", 13]].map(([d, n]) =>
  `<div class="col" style="align-items:center;gap:4px;padding:6px 0;border-radius:10px;background:${n === 9 ? T.raised : "transparent"};border:1px solid ${n === 9 ? T.hairS : "transparent"}"><span class="t3" style="font-size:10px;font-weight:600">${d}</span><span class="tnum" style="font-size:13px;font-weight:${n === 9 ? 600 : 500};color:${n < 9 ? T.text3 : T.text}">${n}</span></div>`).join("")}</div>`;
const togglesGrid = () => `<div style="display:grid;grid-template-columns:repeat(2, minmax(0, 1fr));gap:8px">
  ${toggle("wifi", "Wi-Fi", "Off · on ethernet", false)}${toggle("bluetooth", "Bluetooth", "WH-1000XM5", true)}
  ${toggle("belloff", "Do not disturb", "Off", false)}${toggle("moon", "Night light", "Until 07:00", true)}
  ${toggle("target", "Focus", "Off", false)}${toggle("gamepad", "Game mode", "Off", false)}</div>`;
const outputCard = () => `<div class="col" style="gap:12px;padding:12px;border-radius:14px;background:${T.raised};border:1px solid ${T.hair}">
  <div class="row" style="justify-content:space-between"><span class="t2" style="font-size:11px">Output</span><span class="row" style="gap:4px;font-size:11px">${icon("headphones", 12)} WH-1000XM5 ${icon("chevron", 12)}</span></div>
  ${slider(64, 9999, "volume", "64")}${slider(80, 9999, "sun", "80")}</div>`;
const mediaCard = () => `<div class="row" style="gap:12px;padding:10px 12px;border-radius:14px;background:${T.raised};border:1px solid ${T.hair}">
  <div style="width:40px;height:40px;border-radius:10px;background:linear-gradient(135deg,#7A4FBF,#E2A15A);flex-shrink:0"></div>
  <div class="col" style="flex:1;gap:2px;min-width:0"><div style="font-size:13px;font-weight:600">Nightcall</div><div class="t2" style="font-size:12px">Kavinsky · OutRun</div></div>
  <div class="row" style="gap:10px;color:${T.text2}">${icon("prev", 16)}<span style="color:${T.text}">${icon("pause", 18, "currentColor", 2)}</span>${icon("next", 16)}</div></div>`;
const notifList = () => `<div class="col" style="flex:1">
  <div class="row" style="justify-content:space-between;padding:2px 4px 6px"><span class="t2" style="font-size:11px;font-weight:600">3 notifications</span><span style="font-size:11px;color:${T.accent}">Clear all</span></div>
  ${notifRow(appIcon("message", "#5865F2", 28), "Discord", "sam: ready when you are, lobby's up", "now")}
  ${notifRow(appIcon("message", "#5865F2", 28), "Discord", "jo: pushed the fix, want to review?", "12m")}
  ${notifRow(appIcon("steam", "#1B2838", 28), "Steam", "Update finished · Hades II", "1h", true)}</div>`;
const dashHeader = () => `<div class="row" style="height:44px;padding:0 18px 0 14px;justify-content:space-between;border-bottom:1px solid ${T.hair}">
  ${wsIcons([["kitty", "code"], ["firefox"], ["discord", "steam"]], 0)}
  <div class="row" style="gap:8px"><span class="tnum" style="font-size:15px;font-weight:600">14:32</span><span class="t3">·</span><span class="t2" style="font-size:13px">Tue 9 Sep</span></div>
  <div class="row" style="gap:14px;color:${T.text2}">${icon("bluetooth", 15)}${icon("volume", 15)}<span class="row" style="gap:4px">${icon("bell", 15)}<span class="tnum" style="font-size:12px;font-weight:600;color:${T.text}">3</span></span></div></div>`;


// Hover: the pill unfolds into the control panel.
const controlPanel = () => `
<div class="glass col" style="position:absolute;top:12px;left:50%;transform:translateX(-50%);width:420px;border-radius:22px;box-sizing:border-box;overflow:hidden">
  ${dashHeader()}
  <div class="col" style="gap:10px;padding:10px">
    ${togglesGrid()}${outputCard()}${mediaCard()}
    <div class="row" style="justify-content:space-between;padding:2px 4px 0;font-size:11px;color:${T.text2}">
      <span class="row" style="gap:6px">${icon("gamepad", 13)} Steam Controller <span class="tnum" style="color:${T.ok}">82%</span></span>
      <span class="row" style="gap:6px">${icon("power", 13)} Balanced</span>
    </div>
  </div>
</div>`;
files["ControlPanel.dc.html"] = head() + desktop(controlPanel(), { island: null }) + tail;

// Keybind: the full-screen dashboard.
const card = (inner, extra = "") => `<div class="glass col" style="border-radius:20px;padding:14px;gap:12px;box-sizing:border-box;${extra}">${inner}</div>`;
const cardTitle = (t, right = "") => `<div class="row" style="justify-content:space-between"><span class="t2" style="font-size:11px;font-weight:600">${t}</span><span class="t3" style="font-size:11px">${right}</span></div>`;
const spark = (vals, c = T.text2) => `<div class="row" style="gap:2px;align-items:flex-end;height:28px">${vals.map(v => `<div style="width:5px;height:${Math.max(2, v * 28 / 100)}px;border-radius:1px;background:${c};opacity:.8"></div>`).join("")}</div>`;
const stat = (label, val, vals, c) => `<div class="col" style="gap:6px;padding:10px 12px;border-radius:14px;background:${T.raised};border:1px solid ${T.hair}">
  <div class="row" style="justify-content:space-between"><span class="t2" style="font-size:11px;font-weight:600">${label}</span><span class="mono tnum" style="font-size:12px">${val}</span></div>${spark(vals, c)}</div>`;
const devRowS = (n, name, sub) => `<div class="row" style="gap:10px;height:36px">${icon(n, 15, T.text2)}<span style="flex:1;font-size:12px">${name}</span><span class="t2 tnum" style="font-size:11px">${sub}</span></div>`;

const widget = (title, meta, inner, w, h, extra = "") => `<div class="glass col" style="grid-column:span ${w};grid-row:span ${h};border-radius:18px;padding:12px;gap:10px;box-sizing:border-box;min-height:0;overflow:hidden;${extra}">
  <div class="row" style="justify-content:space-between;flex-shrink:0"><span class="t2" style="font-size:11px;font-weight:600">${title}</span><span class="t3" style="font-size:11px">${meta}</span></div>${inner}</div>`;
const statS = (label, val, vals, c) => `<div class="col" style="gap:4px;padding:8px 10px;border-radius:12px;background:${T.raised};border:1px solid ${T.hair};min-width:0">
  <div class="row" style="justify-content:space-between;white-space:nowrap"><span class="t2" style="font-size:11px;font-weight:600">${label}</span><span class="mono tnum" style="font-size:11px">${val}</span></div>${spark(vals, c)}</div>`;
const usage = (label, used, pct) => `<div class="col" style="gap:5px"><div class="row" style="justify-content:space-between"><span style="font-size:12px">${label}</span><span class="mono tnum t2" style="font-size:11px">${used}</span></div><div style="height:3px;border-radius:2px;background:${T.hairS}"><div style="width:${pct}%;height:100%;border-radius:2px;background:${pct > 85 ? T.warn : T.text2}"></div></div></div>`;
const agentRow = (name, state, live) => `<div class="row" style="gap:8px;height:30px"><div style="width:6px;height:6px;border-radius:3px;background:${live ? T.ok : T.text3}"></div><span class="mono" style="font-size:12px;flex:1">${name}</span><span class="t2" style="font-size:11px">${state}</span></div>`;
const wsCard = (n, icons, active) => `<div class="col" style="flex:1;gap:6px;padding:8px;border-radius:12px;background:${active ? T.raised : "transparent"};border:1px solid ${active ? T.hairS : T.hair}"><span class="t3 tnum" style="font-size:10px;font-weight:600">${n}</span><div class="row" style="gap:4px">${icons.map(i => apps[i]()).join("")}</div></div>`;
const gameTile = (bg) => `<div style="flex:1;height:44px;border-radius:10px;background:${bg}"></div>`;

const dashboardFull = () => `
<div style="position:absolute;inset:0;padding:20px 24px;display:grid;grid-template-columns:repeat(12, minmax(0, 1fr));grid-template-rows:repeat(4, minmax(0, 1fr));gap:12px;box-sizing:border-box">
  ${widget("", "", `<div class="row" style="justify-content:space-between;align-items:flex-end;margin-top:-16px"><div class="col"><div class="tnum" style="font-size:34px;font-weight:600;line-height:36px;letter-spacing:-1px">14:32</div><div class="t2" style="font-size:12px">Tuesday 9 September</div></div>${icon("bell", 15, T.text2)}</div>`, 3, 1)}
  ${widget("Media", "Spotify", `<div class="row" style="gap:10px"><div style="width:36px;height:36px;border-radius:9px;background:linear-gradient(135deg,#7A4FBF,#E2A15A);flex-shrink:0"></div><div class="col" style="flex:1;gap:1px;min-width:0"><div style="font-size:12px;font-weight:600">Nightcall</div><div class="t2" style="font-size:11px">Kavinsky</div></div><div class="row" style="gap:8px;color:${T.text2}">${icon("prev", 14)}<span style="color:${T.text}">${icon("pause", 16, "currentColor", 2)}</span>${icon("next", 14)}</div></div>`, 3, 1)}
  ${widget("Workspaces", "3", `<div class="row" style="gap:6px;flex:1">${wsCard(1, ["kitty", "code"], true)}${wsCard(2, ["firefox"], false)}${wsCard(3, ["discord", "steam"], false)}</div>`, 3, 1)}
  ${widget("Notifications", "Clear all", `<div class="col" style="flex:1;min-height:0">${notifRow(appIcon("message", "#5865F2", 26), "Discord", "sam: ready when you are, lobby's up", "now")}${notifRow(appIcon("message", "#5865F2", 26), "Discord", "jo: pushed the fix, want to review?", "12m")}${notifRow(appIcon("steam", "#1B2838", 26), "Steam", "Update finished · Hades II", "1h")}${notifRow(appIcon("cpu", "#2B2D33", 26), "LACT", "Profile applied · Quiet", "3h", true)}</div>`, 3, 4)}
  ${widget("Controls", "", `<div style="display:grid;grid-template-columns:repeat(2, minmax(0, 1fr));gap:6px;flex:1">${[["wifi", "Wi-Fi", false], ["bluetooth", "Bluetooth", true], ["belloff", "Do not disturb", false], ["moon", "Night light", true], ["target", "Focus", false], ["gamepad", "Game mode", false]].map(([n, l, on]) =>
    `<div class="row" style="gap:8px;padding:0 10px;border-radius:12px;background:${on ? T.accent : T.raised};color:${on ? T.ink : T.text};border:1px solid ${on ? "transparent" : T.hair};font-size:12px;font-weight:600;min-height:36px">${icon(n, 15, "currentColor", 2)}${l}</div>`).join("")}</div>
    <div class="col" style="gap:8px;flex-shrink:0">${slider(64, 9999, "volume", "64")}${slider(80, 9999, "sun", "80")}</div>`, 3, 2)}
  ${widget("System", "up 6d 4h · 7.2.3-cachyos", `<div style="display:grid;grid-template-columns:repeat(2, minmax(0, 1fr));gap:6px;flex:1">
    ${statS("CPU", "12%", [8, 14, 9, 22, 12, 10, 30, 12, 9, 15, 11, 12], T.text2)}${statS("GPU", "38% · 54°", [30, 35, 40, 38, 42, 36, 39, 41, 37, 38, 40, 38], T.accent)}
    ${statS("Memory", "9.2G", [14, 14, 14, 15, 15, 14, 14, 14, 15, 14, 14, 14], T.text2)}${statS("Network", "↓1.2M", [5, 40, 20, 60, 30, 10, 45, 25, 80, 30, 20, 30], T.text2)}</div>`, 3, 2)}
  ${widget("Agents", "2 running", `<div class="col">${agentRow("isle · slice 0", "editing", true)}${agentRow("api · tests", "waiting on you", true)}${agentRow("infra", "done 2h", false)}</div>`, 3, 2)}
  ${widget("Session", "Balanced", `<div class="row" style="justify-content:space-between;flex:1;align-items:center"><div class="row" style="gap:2px"><div style="opacity:.6">${trayBtn("steam")}</div><div style="opacity:.6">${trayBtn("message")}</div></div><div class="row" style="gap:2px">${trayBtn("lock")}${trayBtn("moon")}${trayBtn("restart")}${trayBtn("power")}</div></div>`, 3, 1)}
  ${widget("Devices", "", `<div class="col" style="gap:4px">${devRowS("headphones", "WH-1000XM5", "68%").replace("height:36px", "height:24px")}${devRowS("gamepad", "Steam Controller", "82%").replace("height:36px", "height:24px")}</div>`, 3, 1)}
  ${widget("Storage", "", `<div class="col" style="gap:8px">${usage("nvme0 · /", "412 / 931 GB", 44)}${usage("Games", "1.6 / 1.8 TB", 89)}</div>`, 3, 1)}
</div>`;
files["Dashboard.dc.html"] = head() + desktop(dashboardFull(), { island: null, dim: true, blur: true }) + tail;

// --- Launcher ---------------------------------------------------------------
const resultRow = (ic, title, sub, sel = false, trailing = "") => `
<div class="row" style="gap:12px;height:52px;padding:0 12px;border-radius:12px;background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hairS : "transparent"}">
  ${ic}<div class="col" style="flex:1;gap:1px"><div style="font-size:13px;font-weight:600">${title}</div><div class="t2" style="font-size:11px">${sub}</div></div>
  <span class="t3 mono" style="font-size:11px">${trailing}</span></div>`;
const launcher = () => `
<div class="glass col" style="position:absolute;top:150px;left:50%;transform:translateX(-50%);width:640px;border-radius:20px;padding:10px;gap:6px;box-sizing:border-box">
  <div class="row" style="gap:12px;height:48px;padding:0 14px;border-radius:12px;background:${T.raised};border:1px solid ${T.hairS}">
    <span style="color:${T.text2}">${icon("search", 18)}</span>
    <span style="font-size:15px;font-weight:500">ste<span style="display:inline-block;width:1.5px;height:17px;background:${T.text};vertical-align:-3px;margin-left:1px"></span></span>
    <span class="t3" style="font-size:15px;font-weight:400;margin-left:-4px">am</span>
    <div style="flex:1"></div>
    <span class="t3 mono" style="font-size:11px">&gt; run · = maths · / files · : clipboard · @ windows</span>
  </div>
  <div class="col" style="gap:2px">
    ${resultRow(appIcon("steam", "#1B2838", 36), "Steam", "Application", true, "↵")}
    ${resultRow(appIcon("gamepad", "#2B2D33", 36), "Big Picture", "Mode · Steam, Heroic, Lutris", false)}
    ${resultRow(appIcon("folder", "#3B7DD8", 36), "steam", "~/.local/share/Steam", false)}
    ${resultRow(appIcon("power", "#2B2D33", 36), "Sleep", "Power", false)}
  </div>
</div>`;
files["Launcher.dc.html"] = head() + desktop(launcher()) + tail;

// --- Switcher ---------------------------------------------------------------
const switcher = () => `
<div style="position:absolute;top:210px;left:0;right:0;display:flex;flex-direction:column;align-items:center;gap:12px">
  <div class="glass row" style="gap:10px;padding:10px;border-radius:24px">
    ${[["terminal", "#2B2D33", false], ["code", "#2F7BD6", false], ["globe", "#E5642E", true], ["message", "#5865F2", false], ["steam", "#1B2838", false]].map(([n, bg, sel]) =>
      `<div style="padding:8px;border-radius:16px;background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hairS : "transparent"}">${appIcon(n, bg, 48)}</div>`).join("")}
    <div style="padding:8px;border-radius:16px;opacity:.4">${appIcon("music", "#8B5CF6", 48)}</div>
  </div>
  <div class="col" style="align-items:center;gap:2px">
    <div style="font-size:13px;font-weight:600">Firefox</div>
    <div class="t2" style="font-size:12px">Hyprland wiki · 2 windows</div>
  </div>
</div>`;
files["Switcher.dc.html"] = head() + desktop(switcher(), { island: null }) + tail;

// --- Lock -------------------------------------------------------------------
const lock = () => `
<div style="position:absolute;top:0;left:0;right:0;display:flex;flex-direction:column;align-items:center;gap:20px;padding-top:150px">
  <div class="glass col" style="align-items:center;padding:20px 48px 22px;border-radius:32px;gap:2px">
    <div class="tnum" style="font-size:56px;font-weight:600;line-height:60px;letter-spacing:-1px">14:32</div>
    <div class="t2" style="font-size:13px">Tuesday 9 September</div>
  </div>
  <div class="glass row" style="width:300px;height:44px;padding:0 16px;gap:10px;border-radius:22px;box-sizing:border-box">
    <span style="color:${T.text2}">${icon("lock", 15)}</span>
    <div class="row" style="gap:5px">${[0,1,2,3,4,5].map(() => dot(T.text, 7)).join("")}</div>
    <div style="flex:1"></div>
    <span class="t3" style="font-size:11px">alex</span>
  </div>
</div>`;
files["Lock.dc.html"] = head() + desktop(lock(), { island: null, dim: true, blur: true }) + tail;

// --- Power menu + Auth -----------------------------------------------------
const powerMenu = () => `
<div class="glass row" style="position:absolute;top:220px;left:50%;transform:translateX(-50%);gap:8px;padding:10px;border-radius:24px">
  ${[["lock", "Lock", false], ["moon", "Sleep", true], ["restart", "Restart", false], ["power", "Shut down", false], ["logout", "Log out", false]].map(([n, l, sel]) =>
    `<div class="col" style="align-items:center;gap:8px;width:88px;height:76px;justify-content:center;border-radius:16px;background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hairS : "transparent"}">${icon(n, 22, sel ? T.text : T.text2)}<span style="font-size:12px;font-weight:${sel ? 600 : 500};color:${sel ? T.text : T.text2}">${l}</span></div>`).join("")}
</div>`;
const auth = () => `
<div class="glass col" style="position:absolute;top:170px;left:50%;transform:translateX(-50%);width:380px;border-radius:20px;padding:20px;gap:16px;box-sizing:border-box">
  <div class="row" style="gap:12px">${appIcon("cpu", "#2B2D33", 40)}<div class="col" style="gap:2px"><div style="font-size:15px;font-weight:600">LACT wants to change GPU settings</div><div class="t2" style="font-size:12px">Authentication is required to apply a power limit.</div></div></div>
  <div class="row" style="height:40px;padding:0 14px;gap:10px;border-radius:10px;background:${T.raised};border:1px solid ${T.accent}"><span style="color:${T.text2}">${icon("key", 14)}</span><div class="row" style="gap:5px">${[0,1,2,3].map(() => dot(T.text, 7)).join("")}</div><span style="display:inline-block;width:1.5px;height:16px;background:${T.text};margin-left:-2px"></span></div>
  <div class="row" style="justify-content:flex-end;gap:8px">
    <div style="height:34px;padding:0 16px;border-radius:10px;display:flex;align-items:center;font-size:13px;font-weight:600;color:${T.text2}">Cancel</div>
    <div style="height:34px;padding:0 16px;border-radius:10px;display:flex;align-items:center;font-size:13px;font-weight:600;background:${T.accent};color:${T.ink}">Authenticate</div>
  </div>
</div>`;
files["PowerAndAuth.dc.html"] = head() + `<div class="col" style="gap:0">${desktop(powerMenu(), { island: islandRest() })}${desktop(auth(), { island: islandAuth() })}</div>` + tail;

// --- Capture ----------------------------------------------------------------
const capture = () => `
<div style="position:absolute;inset:0;background:rgba(10,11,13,.35)"></div>
<div style="position:absolute;left:330px;top:120px;width:520px;height:280px;border:1px solid ${T.accent};box-shadow:0 0 0 9999px rgba(10,11,13,.35);border-radius:2px">
  <div class="glass mono tnum" style="position:absolute;right:0;top:-30px;height:22px;padding:0 8px;border-radius:6px;font-size:11px;display:flex;align-items:center">520 × 280</div>
</div>
<div class="glass row" style="position:absolute;bottom:20px;left:50%;transform:translateX(-50%);height:44px;padding:0 6px;gap:2px;border-radius:22px;box-sizing:border-box">
  ${[["square", "Region", true], ["layout", "Window", false], ["monitor", "Screen", false]].map(([n, l, sel]) =>
    `<div class="row" style="gap:6px;height:32px;padding:0 12px;border-radius:16px;background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hairS : "transparent"};font-size:12px;font-weight:600;color:${sel ? T.text : T.text2}">${icon(n, 14)}${l}</div>`).join("")}
  <div style="width:1px;height:20px;background:${T.hairS};margin:0 6px"></div>
  ${[["camera", true], ["video", false], ["pipette", false]].map(([n, sel]) =>
    `<div style="width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:${sel ? T.text : T.text2};background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hairS : "transparent"}">${icon(n, 15)}</div>`).join("")}
</div>`;
files["Capture.dc.html"] = head() + desktop(capture(), { island: islandRest() }) + tail;

// --- Big Picture ------------------------------------------------------------
const tile = (n, label, bg, sel = false) => `
<div class="col" style="gap:14px;align-items:center">
  <div style="width:200px;height:200px;border-radius:24px;background:${bg};display:flex;align-items:center;justify-content:center;box-shadow:${sel ? `0 0 0 3px ${T.accent}, 0 0 0 5px ${T.ink}` : "none"};transform:${sel ? "scale(1.04)" : "none"}">${icon(n, 64, "#fff", 1.5)}</div>
  <div style="font-size:24px;font-weight:600;color:${sel ? T.text : T.text2}">${label}</div>
</div>`;
files["BigPicture.dc.html"] = head() + `
<div class="wall" style="width:1280px;height:560px;padding:44px 64px;box-sizing:border-box;position:relative">
  <div class="row" style="justify-content:space-between;align-items:flex-start">
    <div class="col" style="gap:4px"><div class="tnum" style="font-size:28px;font-weight:600">14:32</div><div class="t2" style="font-size:15px">Tuesday 9 September</div></div>
    <div class="row" style="gap:20px;color:${T.text2};font-size:15px">
      <span class="row" style="gap:8px">${icon("gamepad", 20)}<span class="tnum" style="color:${T.ok}">82%</span></span>
      <span class="row" style="gap:8px">${icon("headphones", 20)} WH-1000XM5</span>
      <span class="row" style="gap:8px">${icon("volume", 20)}</span>
    </div>
  </div>
  <div class="row" style="gap:48px;justify-content:center;margin-top:44px">
    ${tile("steam", "Steam", "linear-gradient(135deg,#1B2838,#2A475E)", true)}
    ${tile("rocket", "Heroic", "linear-gradient(135deg,#1F1B3A,#3B2F7A)")}
    ${tile("swords", "Lutris", "linear-gradient(135deg,#3A2A16,#8A5A24)")}
    ${tile("monitor", "Desktop", T.raised)}
  </div>
  <div class="row" style="position:absolute;bottom:36px;left:64px;right:64px;justify-content:space-between;font-size:15px;color:${T.text3}">
    <span class="row" style="gap:12px"><span class="row" style="gap:6px"><span style="width:26px;height:26px;border-radius:13px;border:1.5px solid ${T.text3};display:inline-flex;align-items:center;justify-content:center;font-size:12px;font-weight:600">A</span> Launch</span><span class="row" style="gap:6px"><span style="width:26px;height:26px;border-radius:13px;border:1.5px solid ${T.text3};display:inline-flex;align-items:center;justify-content:center;font-size:12px;font-weight:600">B</span> Desktop</span></span>
    <span>Steam Controller · lizard mode</span>
  </div>
</div>` + tail;

// --- Settings window --------------------------------------------------------
const sideItem = (n, l, sel = false) => `<div class="row" style="gap:10px;height:34px;padding:0 10px;border-radius:10px;background:${sel ? T.raised : "transparent"};border:1px solid ${sel ? T.hair : "transparent"};font-size:13px;font-weight:${sel ? 600 : 500};color:${sel ? T.text : T.text2}">${icon(n, 15)}${l}</div>`;
const keyRow = (a, w, m, last = false) => `<div style="display:grid;grid-template-columns:minmax(0,1.4fr) minmax(0,1fr) minmax(0,1fr);gap:12px;height:36px;align-items:center;padding:0 12px;${last ? "" : `border-bottom:1px solid ${T.hair}`}"><span style="font-size:13px">${a}</span><span class="mono t2" style="font-size:12px">${w}</span><span class="mono t2" style="font-size:12px">${m}</span></div>`;
const devRow = (name, id, profile, auto) => `
<div class="row" style="gap:12px;height:56px;padding:0 14px;border-radius:14px;background:${T.raised};border:1px solid ${T.hair}">
  ${icon("keyboard", 18, T.text2)}
  <div class="col" style="flex:1;gap:1px"><div style="font-size:13px;font-weight:600">${name}</div><div class="t3 mono" style="font-size:11px">${id}</div></div>
  <div class="row" style="height:28px;border-radius:8px;background:${T.pressed};padding:2px;gap:2px">
    ${["Windows", "Mac"].map(p => `<div style="height:24px;padding:0 10px;border-radius:6px;display:flex;align-items:center;font-size:12px;font-weight:600;background:${p === profile ? T.raised : "transparent"};color:${p === profile ? T.text : T.text3};box-shadow:${p === profile ? "0 1px 2px rgba(0,0,0,.4)" : "none"}">${p}</div>`).join("")}
  </div>
  <span class="t3" style="font-size:11px;width:34px">${auto ? "auto" : ""}</span>
</div>`;
files["Settings.dc.html"] = head() + `
<div style="width:960px;height:600px;background:#131417;border:1px solid ${T.hairS};border-radius:14px;display:flex;overflow:hidden;box-sizing:border-box">
  <div class="col" style="width:200px;padding:14px 10px;gap:2px;border-right:1px solid ${T.hair};box-sizing:border-box">
    <div style="font-size:15px;font-weight:600;padding:6px 10px 14px">Settings</div>
    ${sideItem("palette", "Appearance")}${sideItem("keyboard", "Keyboard", true)}${sideItem("monitor", "Displays")}${sideItem("volume", "Audio")}${sideItem("wifi", "Network")}${sideItem("bluetooth", "Bluetooth")}${sideItem("bell", "Notifications")}${sideItem("power", "Power")}${sideItem("gamepad", "Modes")}${sideItem("info", "About")}
  </div>
  <div class="col" style="flex:1;padding:24px 28px;gap:20px;overflow:hidden">
    <div class="col" style="gap:4px"><div style="font-size:20px;font-weight:600">Keyboard</div><div class="t2" style="font-size:12px">Each keyboard gets a profile. Hyprland sees one set of bindings; the profile decides what reaches it.</div></div>
    <div class="col" style="gap:8px">
      ${devRow("Corsair K95 RGB Platinum", "1b1c:1b2d", "Windows", true)}
      ${devRow("Keychron Q6 Max", "3434:0b30 · not connected", "Mac", true)}
    </div>
    <div class="col" style="border-radius:14px;border:1px solid ${T.hair};overflow:hidden">
      <div style="display:grid;grid-template-columns:minmax(0,1.4fr) minmax(0,1fr) minmax(0,1fr);gap:12px;height:32px;align-items:center;padding:0 12px;background:${T.raised};font-size:11px;font-weight:600;color:${T.text2}"><span>Action</span><span>Windows</span><span>Mac</span></div>
      ${keyRow("Launcher", "Win Space", "Cmd Space")}
      ${keyRow("App switcher", "Alt Tab", "Cmd Tab")}
      ${keyRow("Workspace 1–9", "Win 1–9", "Ctrl 1–9")}
      ${keyRow("Clipboard", "Win V", "Cmd Shift V")}
      ${keyRow("Capture region", "Win Shift S", "Cmd Shift 4")}
      ${keyRow("Control centre", "Win A", "Cmd Shift C")}
      ${keyRow("Lock", "Win L", "Cmd Ctrl Q", true)}
    </div>
  </div>
</div>` + tail;

// --- Components -------------------------------------------------------------
const sw = (on) => `<div style="width:36px;height:20px;border-radius:10px;background:${on ? T.accent : T.pressed};position:relative;border:1px solid ${on ? "transparent" : T.hair};box-sizing:border-box"><div style="position:absolute;top:2px;${on ? "right:2px" : "left:2px"};width:14px;height:14px;border-radius:7px;background:${on ? T.ink : T.text}"></div></div>`;
const btn = (l, v) => {
  const s = { text: `color:${T.text2}`, raised: `background:${T.raised};border:1px solid ${T.hair}`, accent: `background:${T.accent};color:${T.ink}`, danger: `background:${T.danger};color:${T.ink}` }[v];
  return `<div style="height:34px;padding:0 16px;border-radius:10px;display:flex;align-items:center;font-size:13px;font-weight:600;${s}">${l}</div>`;
};
const spec = (label, el) => `<div class="col" style="gap:10px"><div class="t3" style="font-size:11px;font-weight:600">${label}</div>${el}</div>`;
files["Components.dc.html"] = head() + `
<div class="wall" style="width:720px;height:640px;padding:24px 28px;box-sizing:border-box;display:flex;flex-direction:column;gap:24px">
  <div class="col" style="gap:2px"><div style="font-size:15px;font-weight:600">Component kit</div><div class="t2" style="font-size:12px">Inter 13/500 body · 4pt grid · 1px hairlines · one shadow · one accent</div></div>
  <div style="display:grid;grid-template-columns:repeat(2, minmax(0, 1fr));gap:24px 40px">
    ${spec("Colour", `<div class="row" style="gap:8px">${[T.ink, "#121316", T.raised, T.pressed, T.text, T.accent, T.ok, T.warn, T.danger, T.live].map(c => `<div style="width:36px;height:36px;border-radius:10px;background:${c};border:1px solid ${T.hair}"></div>`).join("")}</div>`)}
    ${spec("Type", `<div class="col" style="gap:4px"><span style="font-size:28px;font-weight:600;line-height:32px">Display 28</span><span style="font-size:20px;font-weight:600">Title 20</span><span style="font-size:15px;font-weight:600">Heading 15</span><span style="font-size:13px">Body 13</span><span class="t2" style="font-size:12px">Secondary 12</span><span class="t3" style="font-size:11px">Caption 11</span><span class="mono t2" style="font-size:12px">mono 12 · 14:32:07</span></div>`)}
    ${spec("Toggle", `<div class="row" style="gap:12px">${sw(true)}${sw(false)}</div>`)}
    ${spec("Slider", slider(64, 220, "volume", "64"))}
    ${spec("Segmented", `<div class="row" style="height:28px;border-radius:8px;background:${T.pressed};padding:2px;gap:2px;width:max-content">${["Region", "Window", "Screen"].map((p, i) => `<div style="height:24px;padding:0 12px;border-radius:6px;display:flex;align-items:center;font-size:12px;font-weight:600;background:${i === 0 ? T.raised : "transparent"};color:${i === 0 ? T.text : T.text3};box-shadow:${i === 0 ? "0 1px 2px rgba(0,0,0,.4)" : "none"}">${p}</div>`).join("")}</div>`)}
    ${spec("Field", `<div class="row" style="height:36px;width:260px;padding:0 12px;gap:8px;border-radius:10px;background:${T.raised};border:1px solid ${T.accent};box-sizing:border-box"><span style="color:${T.text2}">${icon("search", 14)}</span><span class="t3" style="font-weight:400">Search</span></div>`)}
    ${spec("Buttons", `<div class="row" style="gap:8px">${btn("Cancel", "text")}${btn("Eject", "raised")}${btn("Connect", "accent")}${btn("Forget", "danger")}</div>`)}
    ${spec("Focus ring · keyboard and controller only", `<div class="row" style="gap:12px">${iconBtn("wifi", false).replace('style="', `style="box-shadow:0 0 0 2px ${T.ink},0 0 0 4px ${T.accent};`)}<div style="height:34px;padding:0 16px;border-radius:10px;display:flex;align-items:center;font-size:13px;font-weight:600;background:${T.raised};border:1px solid ${T.hair};box-shadow:0 0 0 2px ${T.ink},0 0 0 4px ${T.accent}">Mount</div></div>`)}
    ${spec("Row", `<div class="col" style="width:300px;border-radius:14px;border:1px solid ${T.hair};overflow:hidden">
      <div class="row" style="gap:10px;height:44px;padding:0 12px;border-bottom:1px solid ${T.hair}">${icon("wifi", 16, T.text2)}<div class="col" style="flex:1"><span style="font-size:13px">Home 5G</span><span class="t3" style="font-size:11px">WPA3 · strong</span></div>${icon("check", 14, T.accent)}</div>
      <div class="row" style="gap:10px;height:44px;padding:0 12px;background:${T.raised}">${icon("wifi", 16, T.text2)}<div class="col" style="flex:1"><span style="font-size:13px">Neighbour_2.4</span><span class="t3" style="font-size:11px">WPA2</span></div>${icon("chevron", 14, T.text3)}</div>
    </div>`)}
    ${spec("Glass", `<div class="wall" style="width:300px;height:80px;border-radius:14px;display:flex;align-items:center;justify-content:center"><div class="glass row" style="height:44px;padding:0 16px;gap:10px;border-radius:14px">${icon("bluetooth", 14, T.text2)}<span>78% glass · blur 24 · hairline 8%</span></div></div>`)}
  </div>
</div>` + tail;

// --- Directions: two alternates on the island's material --------------------
const direction = (title, sub, t) => {
  const g = `background:${t.glass};${t.blur ? "backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);" : ""}border:1px solid ${t.hair};box-shadow:${t.shadow}`;
  const dotc = (c, s = 6) => `<div style="width:${s}px;height:${s}px;border-radius:999px;background:${c}"></div>`;
  return `
<div style="width:560px;height:300px;padding:20px 24px;box-sizing:border-box;position:relative;overflow:hidden;background:${t.wall};color:${t.text};font-family:${t.font}">
  <div style="font-size:15px;font-weight:600">${title}</div>
  <div style="font-size:12px;color:${t.text2};margin-bottom:22px">${sub}</div>
  <div class="col" style="gap:18px;align-items:center">
    <div class="row" style="height:${t.h}px;padding:0 12px;gap:10px;border-radius:${t.r};${g}">
      <div class="row" style="gap:5px">${dotc(t.accent)}${dotc(t.text3)}${dotc(t.text3)}</div>
      <div class="tnum" style="font-size:13px;font-weight:500">14:32</div>
      <span style="color:${t.text2}">${icon("bluetooth", 14)}</span>
    </div>
    <div class="row" style="height:${t.h + 22}px;padding:0 16px 0 10px;gap:12px;border-radius:${t.r};width:380px;box-sizing:border-box;${g}">
      ${appIcon("message", "#5865F2", 30)}
      <div class="col" style="flex:1;gap:1px"><div class="row" style="justify-content:space-between"><span style="font-size:13px;font-weight:600">Discord</span><span style="font-size:11px;color:${t.text3}">now</span></div><div style="font-size:12px;color:${t.text2}">sam: ready when you are, lobby's up</div></div>
    </div>
    <div class="row" style="height:${t.h}px;padding:0 12px;gap:10px;border-radius:${t.r};${g}">
      <span style="color:${t.text2}">${icon("volume", 14)}</span>
      <div style="width:120px;height:4px;border-radius:2px;background:${t.hair}"><div style="width:64%;height:100%;border-radius:2px;background:${t.accent}"></div></div>
      <span class="mono tnum" style="font-size:11px;color:${t.text2}">64</span>
    </div>
  </div>
</div>`;
};
files["DirectionWarm.dc.html"] = head() + direction("Alternate · Warm", "Warm greys, amber accent, same geometry. Cosier, less 'gaming'.", {
  wall: "radial-gradient(90% 70% at 78% 115%, #2a2119 0%, rgba(42,33,25,0) 60%), radial-gradient(70% 55% at 8% -10%, #201a17 0%, rgba(32,26,23,0) 60%), #0E0D0C",
  glass: "rgba(24,22,20,.8)", blur: true, hair: "rgba(255,240,220,.1)", shadow: "0 12px 40px rgba(0,0,0,.45)",
  text: "#F4EFE8", text2: "rgba(244,239,232,.62)", text3: "rgba(244,239,232,.38)", accent: "#E8B86D", h: 30, r: "999px", font: "Inter, system-ui, sans-serif",
}) + tail;
files["DirectionInk.dc.html"] = head() + direction("Alternate · Ink", "Opaque, no blur, 8px corners, white accent. Sharper and cheaper to draw; survives game mode unchanged.", {
  wall: "#0A0B0D",
  glass: "#141517", blur: false, hair: "rgba(255,255,255,.12)", shadow: "none",
  text: "#F2F2F3", text2: "rgba(242,242,243,.62)", text3: "rgba(242,242,243,.38)", accent: "#F2F2F3", h: 28, r: "8px", font: "Inter, system-ui, sans-serif",
}) + tail;

// --- canvas.json ------------------------------------------------------------
const boards = [
  ["Main.dc.html", 0, 0, 1280, 560, "Desktop at rest"],
  ["IslandStates.dc.html", 1360, 0, 960, 820, "Island states, 1:1"],
  ["ControlPanel.dc.html", 0, 700, 1280, 560, "Control panel · hover unfolds the island"],
  ["Dashboard.dc.html", 1360, 940, 1280, 560, "Dashboard · the control centre keybind, full screen"],
  ["Launcher.dc.html", 0, 1400, 1280, 560, "Launcher"],
  ["Switcher.dc.html", 1360, 1620, 1280, 560, "App switcher"],
  ["Capture.dc.html", 0, 2100, 1280, 560, "Capture"],
  ["Lock.dc.html", 1360, 2300, 1280, 560, "Lock"],
  ["PowerAndAuth.dc.html", 0, 2800, 1280, 1120, "Power menu · Auth dialog"],
  ["BigPicture.dc.html", 1360, 3000, 1280, 560, "Big Picture"],
  ["Settings.dc.html", 1440, 3700, 960, 600, "Settings window · Keyboard"],
  ["Components.dc.html", 0, 4060, 720, 640, "Component kit"],
  ["DirectionWarm.dc.html", 800, 4060, 560, 300, "Alternate · Warm"],
  ["DirectionInk.dc.html", 800, 4480, 560, 300, "Alternate · Ink"],
];
const canvas = {
  artboards: boards.map(([file, x, y, w, h, title]) => ({ file, x, y, w, h, title })),
  annotations: [
    { id: "brief", x: 0, y: -150, w: 520, text: "Isle — a desktop shell for Hyprland.\nDark glass, one accent, nothing on screen at rest but the island. Every surface here is the same material and grows out of the pill.\nTokens: docs/DESIGN.md. Alternates at the bottom vary only the island's material." },
  ],
  launch: { view: "canvas" },
};

for (const [name, src] of Object.entries(files)) writeFileSync(join(out, name), src);
writeFileSync(join(out, "canvas.json"), JSON.stringify(canvas, null, 2) + "\n");
console.log(`wrote ${Object.keys(files).length} artboards + canvas.json to ${out}`);
