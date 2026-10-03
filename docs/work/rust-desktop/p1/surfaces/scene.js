// Drawing kit for P1's surface boards: Isle's surfaces from system/components.css, sized by tokens.kdl (p1/surfaces/*.md).
(function () {
  "use strict";
  const { h } = Isle;
  const W = 3440, H = 1440, CX = W / 2;

  const el = (cls, style, ...kids) => h("div", { class: cls, style: style || {} }, ...kids);
  const sp = (cls, s, style) => h("span", { class: cls, style: style || {} }, s);
  const at = (x, y, w, hh) => ({ left: x + "px", top: y + "px", width: w != null ? w + "px" : undefined, height: hh != null ? hh + "px" : undefined });
  const box = (cls, x, y, w, hh, style, ...kids) => el("s " + cls, Object.assign(at(x, y, w, hh), style || {}), ...kids);
  const row = (style, ...kids) => el("s-row", style, ...kids);
  const col = (style, ...kids) => el("s-col", style, ...kids);
  const grow = () => el("s-grow");
  const tok = (node, name, fallback) => parseFloat(getComputedStyle(node).getPropertyValue(name)) || fallback;

  // Phosphor glyph, inline so it takes currentColor. `fill` picks the fill weight.
  function ic(name, size, fill, style) {
    const n = name + (fill ? "-fill" : "");
    const e = h("i", { class: "s-ic", style: Object.assign(size ? { width: size + "px", height: size + "px" } : {}, style || {}) });
    e.innerHTML = '<svg viewBox="0 0 256 256" fill="currentColor" width="100%" height="100%">' + ((window.PHOSPHOR || {})[n] || "") + "</svg>";
    return e;
  }
  // A level glyph: the full glyph dimmed with the level over it (icons.md).
  function level(full, part, size) {
    return h("span", { style: { position: "relative", width: size + "px", height: size + "px", display: "inline-block", flex: "none" } },
      ic(full, size, false, { position: "absolute", inset: 0, color: "var(--colour-text-disabled)" }),
      ic(part, size, false, { position: "absolute", inset: 0 }));
  }
  // Isle-drawn glyphs Phosphor lacks.
  function isle(name, size) {
    const paths = {
      mark: '<path d="M70 150c-10-40 18-86 66-92 30-4 50 10 60 30 6 12-6 20-16 16-14-6-28 0-26 16 2 14 20 18 30 28 14 14 4 40-24 48-40 12-80-6-90-46z" fill="currentColor"/>',
      equaliser: '<rect x="44" y="96" width="36" height="112" rx="18" fill="currentColor"/><rect x="110" y="48" width="36" height="160" rx="18" fill="currentColor"/><rect x="176" y="120" width="36" height="88" rx="18" fill="currentColor"/>',
      "keyboard-slash": '<rect x="28" y="64" width="200" height="128" rx="16" fill="none" stroke="currentColor" stroke-width="16"/><path d="M40 40l176 176" stroke="currentColor" stroke-width="16" stroke-linecap="round"/>',
      vpn: '<path d="M128 24l80 32v56c0 56-36 92-80 112-44-20-80-56-80-112V56z" fill="currentColor"/>',
    };
    const e = h("i", { class: "s-ic", style: { width: size + "px", height: size + "px" } });
    e.innerHTML = '<svg viewBox="0 0 256 256" width="100%" height="100%">' + (paths[name] || "") + "</svg>";
    return e;
  }
  const cover = (size, round) => h("span", { style: { width: size + "px", height: size + "px", borderRadius: round ? "50%" : Math.round(size / 5) + "px", background: "linear-gradient(135deg,#c05a8a,#4a3a9a)", flex: "none", display: "block" } });
  const appIcon = (size, a, b, glyph) => h("span", { style: { width: size + "px", height: size + "px", borderRadius: Math.round(size * 0.22) + "px", background: "linear-gradient(180deg," + a + "," + b + ")", display: "grid", placeItems: "center", color: "#fff", flex: "none" } }, glyph ? ic(glyph, Math.round(size * 0.56)) : null);

  // ---- Island ---------------------------------------------------------------
  const clock = (t) => sp("s-clock", t || "14:32", { minWidth: "38px", textAlign: "center" });
  const well = (...kids) => h("span", { style: { height: "var(--size-island-well)", minWidth: "var(--size-island-well)", padding: "0 6px", boxSizing: "border-box", borderRadius: "11px", background: "var(--material-solid)", display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "var(--space-xs)", margin: "0 -8px 0 0" } }, ...kids);
  const leadWell = (...kids) => h("span", { style: { height: "var(--size-island-well)", minWidth: "var(--size-island-well)", padding: "0 8px 0 6px", boxSizing: "border-box", borderRadius: "11px", background: "var(--material-solid)", display: "inline-flex", alignItems: "center", gap: "var(--space-xs)", margin: "0 0 0 -8px" } }, ...kids);
  const PRIV = { microphone: ["microphone", "--colour-privacy-microphone"], camera: ["video-camera", "--colour-privacy-camera"], screen: ["screencast", "--colour-privacy-screen"] };
  const privacy = (kinds) => well(...kinds.map((k) => ic(PRIV[k][0], 12, true, { color: "var(" + PRIV[k][1] + ")" })));
  const mark = (name) => ic(name, 12, true);
  const count = (n) => sp("s-sub", "+" + n, { fontWeight: 600, padding: "0 var(--space-xs)", height: "16px", lineHeight: "16px", borderRadius: "8px", background: "var(--colour-fill-strong)" });
  const faces = {
    media: (paused) => [h("span", { style: { margin: "0 0 0 -8px", display: "flex" } }, cover(22, true)), paused ? ic("pause", 12) : isle("equaliser", 12)],
    recording: (t) => [leadWell(h("span", { style: { width: "8px", height: "8px", borderRadius: "50%", background: "var(--colour-live)" } }), sp("s-clock s-num", t || "0:42", { color: "var(--colour-text)" }))],
    sharing: (app) => [leadWell(ic("screencast", 12, true, { color: "var(--colour-privacy-screen)" }), appIcon(16, "#ff9a3c", "#e6572c", "globe")), sp("", app || "Firefox", { fontSize: "var(--type-style-body-size)" })],
    timer: (t, n) => [ic("timer", 16), sp("s-clock s-num", t || "4:58"), n ? sp("s-sub s-2", "+" + n) : null],
  };
  // The pill: one shape centred at the top of the screen.
  function pill(s, kids, o) {
    o = o || {};
    const e = el("s s-island", Object.assign({ left: CX + "px", top: "var(--size-island-top)", transform: "translateX(-50%)", gap: "var(--space-s)", padding: "0 var(--space-m)", boxShadow: "var(--shadow-menu)", justifyContent: "center" }, o.style || {}), ...kids.filter(Boolean));
    s.append(e);
    return e;
  }
  const glyphs = (list) => h("span", { style: { display: "inline-flex", gap: "var(--space-s)", marginLeft: "4px", alignItems: "center" } }, ...list);
  function lineEvent(s, icon, text, extra) {
    return pill(s, [icon, sp("", text, { fontWeight: "var(--type-weight-medium)" }), extra]);
  }
  function levelEvent(s, icon, name, pct, privacyKinds) {
    const e = el("s s-island", { left: CX + "px", top: "var(--size-island-top)", transform: "translateX(-50%)", height: "var(--size-island-event)", width: "var(--size-island-osd)", borderRadius: "var(--radius-panel)", padding: "0 var(--space-l)", gap: "var(--space-m)", boxShadow: "var(--shadow-menu)" },
      icon, col({ gap: "var(--space-xs)", flex: 1 }, sp("s-sub s-2", name), h("div", { style: { height: "6px", borderRadius: "3px", background: "var(--colour-fill-strong)", position: "relative" } }, h("b", { style: { position: "absolute", left: 0, top: 0, bottom: 0, width: pct + "%", borderRadius: "3px", background: "var(--colour-island-text)" } }))),
      privacyKinds ? privacy(privacyKinds) : null);
    s.append(e);
    return e;
  }

  // Control Centre and Notification Centre: glass panels unfolding from the pill.
  function panel(s, rowKids, body, o) {
    o = o || {};
    const e = el("s s-glass", Object.assign({ left: CX + "px", top: "var(--size-island-top)", transform: "translateX(-50%)", width: "var(--size-panel)", borderRadius: "var(--radius-panel)", padding: "0 var(--space-s) var(--space-s)", boxSizing: "border-box", color: "var(--colour-text)" }, o.style || {}),
      h("div", { style: { height: "var(--size-island-height)", display: "flex", alignItems: "center", justifyContent: "center", gap: "var(--space-s)", marginBottom: "var(--space-s)" } }, ...rowKids.filter(Boolean)),
      el("s-col", { gap: "var(--space-s)" }, ...[].concat(body).filter(Boolean)));
    s.append(e);
    return e;
  }
  const card = (style, ...kids) => el("", Object.assign({ background: "var(--colour-fill)", borderRadius: "var(--radius-card)" }, style || {}), ...kids);
  const btnS = (t, primary) => sp("s-btn small" + (primary ? " primary" : ""), t);
  function nowLeader(o) {
    return el("s-row", { height: "56px", padding: "0 var(--space-m)", gap: "var(--space-m)" }, o.icon,
      col({ gap: "0", flex: 1, minWidth: 0 }, sp("s-headline", o.title), sp("s-sub s-2 s-num", o.sub)), ...(o.actions || []).map((a, i) => btnS(a, i === 0 && o.primary)));
  }
  function nowRow(icon, text, action) {
    return el("s-row", { height: "var(--size-row)", padding: "0 var(--space-m)", gap: "var(--space-s)" }, icon, sp("", text, { flex: 1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }), action ? btnS(action) : null);
  }
  // A module tile in one of its four frames.
  function module(size, o) {
    const cols = { S: 1, M: 2, L: 2, W: 4 }[size], rows = size === "L" ? 2 : 1;
    const dot = el("", { width: "var(--size-module-glyph)", height: "var(--size-module-glyph)", borderRadius: "50%", display: "grid", placeItems: "center", flex: "none", background: o.on ? "var(--colour-accent)" : "var(--colour-fill-strong)", color: o.on ? "var(--colour-on-accent)" : "var(--colour-text)" }, ic(o.glyph, size === "L" || size === "W" ? 22 : 18, o.on));
    const text = col({ gap: "0", minWidth: 0 }, sp("s-headline", o.name, { whiteSpace: "nowrap" }), o.state ? sp("s-sub s-2", o.state, { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }) : null);
    const kids = size === "S" ? [el("", { margin: "auto" }, dot)] : size === "W" ? [row({ gap: "var(--space-m)", height: "100%" }, dot, col({ gap: "6px", flex: 1 }, sp("s-headline", o.name), o.content))] : size === "L" ? [row({ gap: "var(--space-m)" }, dot, text), o.content] : [row({ gap: "var(--space-m)", height: "100%" }, dot, text)];
    return el("", { gridColumn: "span " + cols, gridRow: "span " + rows, background: "var(--colour-fill)", borderRadius: "var(--radius-card)", padding: size === "L" ? "var(--space-m)" : size === "S" ? "0" : "0 var(--space-l)", display: "flex", flexDirection: "column", gap: "var(--space-s)", boxSizing: "border-box", minWidth: 0 }, ...kids);
  }
  const grid = (...mods) => el("", { display: "grid", gridTemplateColumns: "repeat(4, var(--size-module-cell-width))", gridAutoRows: "var(--size-module-cell-height)", gap: "var(--space-s)" }, ...mods);
  const ccSlider = (pct, glyph) => h("div", { style: { height: "24px", borderRadius: "12px", background: "var(--colour-fill-strong)", position: "relative", overflow: "hidden" } }, h("b", { style: { position: "absolute", left: 0, top: 0, bottom: 0, width: pct + "%", background: "var(--colour-text)", opacity: .92 } }), ic(glyph, 14, false, { position: "absolute", left: "6px", top: "5px", color: "var(--colour-window)" }));
  function defaultGrid() {
    return grid(
      module("M", { glyph: "wifi-high", on: true, name: "Wi-Fi", state: "Home" }),
      module("L", { glyph: "music-note", on: false, name: "Now Playing", state: "M83", content: row({ gap: "var(--space-m)", marginTop: "auto" }, cover(40), col({ gap: "0" }, sp("s-callout", "Midnight City", { fontWeight: 600 }), row({ gap: "var(--space-l)", marginTop: "4px" }, ic("skip-back", 16, true), ic("pause", 18, true), ic("skip-forward", 16, true)))) }),
      module("M", { glyph: "bluetooth", on: true, name: "Bluetooth", state: "2 devices" }),
      module("M", { glyph: "moon", on: false, name: "Focus", state: "Off" }),
      module("M", { glyph: "shield", on: false, name: "VPN", state: "Not connected" }),
      module("W", { glyph: "speaker-high", on: false, name: "Sound", content: ccSlider(64, "speaker-low") }),
      module("W", { glyph: "sun", on: false, name: "Display", content: ccSlider(78, "sun-dim") }),
      module("S", { glyph: "sun-horizon", on: false }), module("S", { glyph: "gauge", on: true }),
      module("M", { glyph: "arrow-circle-up", on: false, name: "Updates", state: "3 available" }),
      module("M", { glyph: "game-controller", on: false, name: "Batteries", state: "Controller 70%" }));
  }
  const tray = () => card({ padding: "var(--space-s)", display: "flex", gap: "var(--space-xs)" }, ...["cloud", "chat-circle", "key"].map((g) => el("", { width: "var(--size-row)", height: "var(--size-row)", display: "grid", placeItems: "center", borderRadius: "var(--radius-row)" }, ic(g, 16))));
  const footer = () => el("s-row", { height: "var(--size-row)", padding: "0 var(--space-xs)" }, sp("s-callout s-2", "Edit controls"), grow(), sp("s-callout s-2", "Control Centre Settings…"));

  // ---- Windows ----------------------------------------------------------------
  function lights(o) {
    o = o || {};
    const L = (c, glyph) => h("i", { style: { width: "var(--size-traffic-light)", height: "var(--size-traffic-light)", borderRadius: "50%", display: "grid", placeItems: "center", background: o.inactive ? "var(--colour-fill-strong)" : "var(--traffic-" + c + ")", boxShadow: o.inactive ? "none" : "inset 0 0 0 0.5px var(--traffic-edge)", color: "var(--traffic-" + c + "-glyph)" } },
      o.hover ? h("span", { style: { width: "8px", height: "8px" } }) : null);
    const e = el("s-lights", { gap: "var(--space-s)" }, L("close"), L("minimise"), L("zoom"));
    if (o.hover) {
      const g = ['<path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>', '<path d="M2.5 6h7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>', '<path d="M3 4v5h5zM9 8V3H4z" fill="currentColor"/>'];
      [...e.children].forEach((c, i) => { c.firstChild.innerHTML = '<svg viewBox="0 0 12 12" width="8" height="8">' + g[i] + "</svg>"; });
    }
    return e;
  }
  function titlebarWindow(s, x, y, w, hh, o) {
    o = o || {};
    const win = box("s-window" + (o.inactive ? " is-inactive" : ""), x, y, w, hh, o.style);
    win.append(el("", { height: "var(--size-titlebar)", display: "flex", alignItems: "center", padding: "0 var(--space-l)", position: "relative", background: o.inactive ? "var(--colour-titlebar-inactive)" : "var(--colour-titlebar)", borderBottom: "1px solid var(--colour-hairline)" },
      lights({ inactive: o.inactive, hover: o.hover }), sp("s-headline", o.title || "Window", { position: "absolute", left: 0, right: 0, textAlign: "center", pointerEvents: "none", color: o.inactive ? "var(--colour-text-3)" : "var(--colour-text)" })));
    if (o.body) win.append(o.body);
    s.append(win);
    return win;
  }
  function terminal(s, x, y, w, hh, o) {
    const body = h("div", { class: "s-mono", style: { padding: "14px 18px", whiteSpace: "pre", lineHeight: "1.6", color: "var(--term-white)", background: "var(--colour-view)", height: "100%" } },
      h("span", { style: { color: "var(--term-blue)" } }, "~/D/isle"), " ", h("span", { style: { color: "var(--term-bright-black)" } }, "feature/rust-desktop"), " cargo build -p isle-compositor\n",
      "   Compiling isle-compositor v0.1.0\n", h("span", { style: { color: "var(--term-green)" } }, "    Finished"), " `dev` profile in 41.2s\n",
      h("span", { style: { color: "var(--term-blue)" } }, "~/D/isle"), " ", h("span", { style: { color: "var(--term-bright-black)" } }, "feature/rust-desktop"), " ▌");
    return titlebarWindow(s, x, y, w, hh, Object.assign({ title: "~/Development/isle — fish", body }, o || {}));
  }
  const PANES = [["wifi-high", "blue", "Wi-Fi"], ["bluetooth", "blue", "Bluetooth"], ["globe", "blue", "Network"], null, ["gear", "grey", "General"], ["person-arms-spread", "blue", "Accessibility"], ["circle-half", "graphite", "Appearance"], ["squares-four", "purple", "Control Centre"], ["app-window", "graphite", "Desktop"], ["monitor", "blue", "Displays"]];
  function settings(s, x, y, w, hh, o) {
    o = o || {};
    const win = box("s-window" + (o.inactive ? " is-inactive" : ""), x, y, w, hh);
    const side = el("s-sidebar", { position: "absolute", left: 0, top: 0, bottom: 0, width: "var(--size-sidebar)", background: "var(--colour-sidebar)", paddingTop: "0" },
      el("", { height: "var(--size-toolbar)", display: "flex", alignItems: "center", padding: "0 var(--space-s)" }, lights({ inactive: o.inactive })),
      el("s-field search", { marginBottom: "var(--space-s)" }, ic("magnifying-glass", 14), sp("ph", "Search")),
      ...PANES.map((p) => p ? el("s-side-item" + (p[2] === "Appearance" ? " sel" : ""), {}, h("span", { class: "s-pane-ic", style: { background: "var(--pane-" + p[1] + ")" } }, ic(p[0], 13)), sp("", p[2])) : el("", { height: "var(--space-s)" })));
    const main = el("", { position: "absolute", left: "var(--size-sidebar)", right: 0, top: 0, bottom: 0 },
      el("s-toolbar", {}, ic("caret-left", 20), ic("caret-right", 20, false, { color: "var(--colour-text-3)" }), sp("s-headline", "Appearance")),
      col({ width: "560px", margin: "var(--space-xxl) auto", gap: "var(--space-l)" },
        el("s-group", {}, el("r", {}, sp("", "Appearance"), grow(), el("s-seg", {}, sp("", "Light"), sp("on", "Dark"), sp("", "Auto"))), el("r", {}, sp("", "Accent colour"), grow(), row({ gap: "8px" }, ...["blue", "purple", "pink", "red", "orange", "yellow", "green", "graphite"].map((a) => h("span", { style: { width: "16px", height: "16px", borderRadius: "50%", background: "var(--accent-" + a + "-fill)" } }))))),
        el("s-group", {}, el("r", {}, sp("", "Glass"), grow(), sp("s-callout s-2", "Clear"), el("", { width: "180px" }, el("s-slider", { minWidth: "0" }, h("b", { style: { width: "50%" } }), h("i", { style: { left: "50%" } }))), sp("s-callout s-2", "Tinted")), el("r", {}, sp("", "Reduce transparency"), grow(), h("span", { class: "s-switch" })))));
    win.append(side, main);
    s.append(win);
    return win;
  }
  function desktop(s, o) {
    o = o || {};
    terminal(s, 1520, 180, 1180, 720, { inactive: !o.termActive });
    settings(s, 320, 110, 1100, 760, { inactive: !!o.termActive });
  }

  // ---- Menus ------------------------------------------------------------------
  // items: strings, "—", or { label, key, check, sel, dim, sub, head }
  function menu(x, y, items, o) {
    o = o || {};
    const mat = o.material === "window" ? { background: "var(--colour-window-raised)", boxShadow: "var(--shadow-menu)" } : o.material === "solid" ? { background: "var(--material-solid)", boxShadow: "var(--shadow-menu)" } : {};
    const m = box((o.material ? "" : "s-glass ") + "s-menu", x, y, null, null, Object.assign({ minWidth: "var(--size-menu-min)", padding: "5px" }, mat));
    for (const it of items) {
      if (it === "—") { m.append(h("hr")); continue; }
      const i = typeof it === "string" ? { label: it } : it;
      if (i.head) { m.append(el("head", {}, i.head)); continue; }
      m.append(el("s-mi" + (i.sel ? " sel" : "") + (i.dim ? " dim" : ""), { height: "var(--size-menu-row)", background: i.open ? "var(--colour-fill-strong)" : undefined },
        sp("mark", i.check || "", { width: "16px" }), sp("", i.label), i.key ? sp("key", i.key) : null, i.sub ? ic("caret-right", 12, false, { marginLeft: "auto" }) : null));
    }
    return m;
  }
  function bar(s, o) {
    o = o || {};
    const b = box("s-glass", 0, 0, W, null, { height: "var(--size-bar-height)", borderRadius: 0, boxShadow: "var(--shadow-bar)", borderBottom: "1px solid var(--colour-hairline)", display: "flex", alignItems: "center", gap: "2px", padding: "0 var(--space-s)", background: o.solid ? "var(--material-solid)" : undefined });
    const title = (t, cls, open) => sp(cls || "", t, { padding: "3px var(--space-s)", borderRadius: "var(--radius-control)", background: open ? "var(--colour-fill-strong)" : "transparent" });
    b.append(h("span", { style: { padding: "0 var(--space-s)", display: "flex" } }, isle("mark", 16)), title(o.app || "System Settings", "s-headline"), ...(o.menus || ["File", "Edit", "View", "Window", "Help"]).map((m) => title(m, "", m === o.open)));
    s.append(b);
    return b;
  }

  window.Scene = { W, H, CX, el, sp, at, box, row, col, grow, tok, ic, level, isle, cover, appIcon, clock, well, leadWell, privacy, mark, count, faces, pill, glyphs, lineEvent, levelEvent, panel, card, btnS, nowLeader, nowRow, module, grid, ccSlider, defaultGrid, tray, footer, lights, titlebarWindow, terminal, settings, desktop, menu, bar };
})();
