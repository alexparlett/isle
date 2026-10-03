// P1 board harness: a scaled 3440×1440 screen, tokens read from tokens.kdl, a state picker.
(function () {
  "use strict";

  // ---- KDL: the subset tokens.kdl uses (nodes, string, number and bool
  // arguments, properties, children, comments). Not a full KDL v2 parser.
  function parseKdl(src) {
    let i = 0;
    const n = src.length;
    const isWs = (c) => c === " " || c === "\t" || c === "\r";
    const isIdent = (c) => c && !/[\s{}()\[\]";=\\\/]/.test(c);

    function skip(newlines) {
      for (;;) {
        const c = src[i];
        if (isWs(c) || (newlines && (c === "\n" || c === ";"))) i++;
        else if (c === "\\") { i++; while (i < n && src[i] !== "\n") i++; i++; }
        else if (c === "/" && src[i + 1] === "/") { while (i < n && src[i] !== "\n") i++; }
        else if (c === "/" && src[i + 1] === "*") {
          let depth = 0;
          do {
            if (src.startsWith("/*", i)) { depth++; i += 2; }
            else if (src.startsWith("*/", i)) { depth--; i += 2; }
            else i++;
          } while (depth > 0 && i < n);
        } else return;
      }
    }

    function str() {
      i++;
      let out = "";
      while (i < n && src[i] !== '"') {
        if (src[i] === "\\") {
          const e = src[++i];
          out += e === "n" ? "\n" : e === "t" ? "\t" : e;
        } else out += src[i];
        i++;
      }
      i++;
      return out;
    }

    function word() {
      const s = i;
      while (isIdent(src[i])) i++;
      return src.slice(s, i);
    }

    function value() {
      if (src[i] === '"') return str();
      const w = word();
      if (w === "#true") return true;
      if (w === "#false") return false;
      if (w === "#null") return null;
      const num = Number(w.replace(/_/g, ""));
      return Number.isNaN(num) ? w : num;
    }

    function nodes(close) {
      const out = [];
      for (;;) {
        skip(true);
        if (i >= n || src[i] === close) { i++; return out; }
        if (src.startsWith("/-", i)) { i += 2; skip(false); nodeOne(); continue; }
        out.push(nodeOne());
      }
    }

    function nodeOne() {
      const name = src[i] === '"' ? str() : word();
      if (!name) throw new Error("tokens.kdl: unexpected '" + src[i] + "' at " + i);
      const node = { name, args: [], props: {}, children: [] };
      for (;;) {
        skip(false);
        const c = src[i];
        if (c === undefined || c === "\n" || c === ";" || c === "}") return node;
        if (c === "{") { i++; node.children = nodes("}"); return node; }
        const s = i;
        const v = value();
        if (src[i] === "=" && typeof v === "string") { i++; node.props[v] = value(); }
        else node.args.push(v);
        if (i === s) throw new Error("tokens.kdl: stuck at " + i);
      }
    }

    return nodes(undefined);
  }

  // Flattens a token tree to CSS custom properties. A `light` or `dark`
  // node anywhere on a path splits its tokens by appearance, so
  // `colour { text { dark "#fff"; light "#111" } }` and
  // `colour { dark { text "#fff" } }` both give --colour-text per theme.
  function tokensToVars(tree) {
    const vars = { any: {}, dark: {}, light: {} };
    (function walk(list, path, theme) {
      for (const node of list) {
        let t = theme;
        let p = path;
        if (node.name === "dark" || node.name === "light") t = node.name;
        else p = path.concat(node.name);
        if (node.args.length) {
          const v = node.args.map((a) => (typeof a === "number" && !/(opacity|weight|scale|ratio|stiffness|damping|mass|factor|saturation|brightness|height-ratio|mix|-ms)$/.test(p.join("-")) ? a + "px" : a)).join(" ");
          vars[t || "any"]["--" + p.join("-")] = v;
        }
        walk(node.children, p, t);
      }
    })(tree, [], null);
    return vars;
  }

  // ---- Spring. Same parameters the compositor takes: stiffness, damping,
  // mass, from a value and velocity to a target.
  function spring(opts, onFrame) {
    const { from = 0, to = 1, stiffness = 300, damping = 30, mass = 1, velocity = 0 } = opts;
    if (Isle.flags.reduceMotion) { onFrame(to, true); return () => {}; }
    let x = from, v = velocity, last = performance.now(), raf;
    function step(now) {
      let dt = Math.min(0.064, (now - last) / 1000);
      last = now;
      while (dt > 0) {
        const h = Math.min(dt, 1 / 240);
        const a = (-stiffness * (x - to) - damping * v) / mass;
        v += a * h;
        x += v * h;
        dt -= h;
      }
      const done = Math.abs(v) < 0.001 && Math.abs(x - to) < 0.0005;
      onFrame(done ? to : x, done);
      if (!done) raf = requestAnimationFrame(step);
    }
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }

  // ---- Storage, per viewer, never required.
  function load(key, fallback) {
    try { const v = localStorage.getItem("isle-p1:" + key); return v === null ? fallback : JSON.parse(v); }
    catch (e) { return fallback; }
  }
  function save(key, value) {
    try { localStorage.setItem("isle-p1:" + key, JSON.stringify(value)); } catch (e) { /* not kept */ }
  }

  function h(tag, attrs, ...children) {
    const el = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (k === "class") el.className = v;
      else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
      else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
      else if (v !== false && v !== null && v !== undefined) el.setAttribute(k, v === true ? "" : v);
    }
    for (const c of children.flat()) if (c !== null && c !== undefined && c !== false) el.append(c.nodeType ? c : String(c));
    return el;
  }

  const SCREENS = {
    ultrawide: { w: 3440, h: 1440, name: "3440 × 1440" },
    hd: { w: 1920, h: 1080, name: "1920 × 1080" },
  };

  const WALLPAPERS = {
    wire: { name: "Flat", wire: true },
    dusk: { name: "Dusk", css: "radial-gradient(120% 90% at 70% 110%, #3b2f5c 0%, #1d1a33 40%, #0d0d16 75%)" },
    slate: { name: "Slate", css: "linear-gradient(160deg, #23262b 0%, #15171a 60%, #0f1012 100%)" },
    pine: { name: "Pine", css: "radial-gradient(90% 120% at 15% 0%, #1f3a35 0%, #10201d 45%, #0a1110 100%)" },
    haze: { name: "Haze", css: "radial-gradient(40% 55% at 18% 30%, #3a2f63 0%, transparent 70%), radial-gradient(45% 60% at 82% 70%, #1f4a57 0%, transparent 70%), radial-gradient(30% 40% at 55% 15%, #5a3348 0%, transparent 70%), #101116" },
    mist: { name: "Mist", css: "radial-gradient(100% 120% at 30% 0%, #e9eef4 0%, #cfd8e3 55%, #b7c3d1 100%)" },
  };

  const Isle = {
    h, spring, parseKdl, tokensToVars,
    flags: { reduceMotion: false, reduceTransparency: false, increaseContrast: false },
  };

  // config: { title, subtitle, tokens: "path/to/tokens.kdl" | null,
  //   directions: [{ id, name, tokens, note }] (optional; replaces tokens),
  //   selects: [{ id, label, options: { value: name }, value, apply(screen, value) }] (optional),
  //   surfaces: [{ id, name, sheet, states: [{ id, name, note, screen, render(screen, ctx) }] }] }
  Isle.board = function (config) {
    const directions = config.directions || null;
    const hasTokens = !!(config.tokens || directions);
    const state = {
      surface: config.surfaces[0].id,
      st: config.surfaces[0].states[0].id,
      theme: load("theme", "dark"),
      mode: hasTokens ? load("mode", "tokens") : "wire",
      wallpaper: load("wallpaper:" + config.title, config.wallpaper || (hasTokens ? "dusk" : "wire")),
      screen: "ultrawide",
      actual: false,
      whole: load("whole", false),
      direction: directions ? load("direction:" + config.title, directions[0].id) : null,
    };
    if (directions && !directions.find((d) => d.id === state.direction)) state.direction = directions[0].id;
    const extra = (config.selects || []).map((x) => ({ ...x, value: load("sel:" + config.title + ":" + x.id, x.value) }));
    const tokensPath = () => directions ? directions.find((d) => d.id === state.direction).tokens : config.tokens;
    Object.assign(Isle.flags, load("flags", {}));

    function readHash() {
      const [sid, stid] = location.hash.slice(1).split(".");
      const s = config.surfaces.find((x) => x.id === sid);
      if (!s) return false;
      state.surface = s.id;
      state.st = (s.states.find((x) => x.id === stid) || s.states[0]).id;
      return true;
    }
    readHash();
    window.addEventListener("hashchange", () => { if (readHash()) render(); });

    const root = h("div", { class: "hb" });
    document.body.append(root);

    const head = h("header", { class: "hb-head" },
      h("div", { class: "hb-titles" },
        h("h1", {}, config.title),
        config.subtitle ? h("p", {}, config.subtitle) : null));
    const tabs = h("nav", { class: "hb-tabs", "aria-label": "Surfaces" });
    const states = h("div", { class: "hb-states", role: "tablist", "aria-label": "States" });
    const controls = h("div", { class: "hb-controls" });
    const viewport = h("div", { class: "hb-viewport" });
    const scaler = h("div", { class: "hb-scaler" });
    const screen = h("div", { class: "screen", "data-theme-screen": state.theme });
    const wall = h("div", { class: "screen-wall" });
    const layer = h("div", { class: "screen-layer" });
    screen.append(wall, layer);
    scaler.append(screen);
    viewport.append(scaler);
    const note = h("section", { class: "hb-note" });
    const error = h("p", { class: "hb-error", hidden: true });
    root.append(head, tabs, states, controls, error, viewport, note);

    function select(name, label, options, value, onChange) {
      const id = "hb-" + name;
      const sel = h("select", { id, onchange: (e) => onChange(e.target.value) },
        Object.entries(options).map(([k, v]) => h("option", { value: k, selected: k === value }, v)));
      return h("label", { class: "hb-field", for: id }, h("span", {}, label), sel);
    }
    function toggle(name, label, checked, onChange) {
      const id = "hb-" + name;
      return h("label", { class: "hb-check", for: id },
        h("input", { type: "checkbox", id, checked, onchange: (e) => onChange(e.target.checked) }), label);
    }

    function buildControls() {
      controls.replaceChildren(...[
        directions ? select("direction", "Direction", Object.fromEntries(directions.map((d) => [d.id, d.name])), state.direction, (v) => { state.direction = v; save("direction:" + config.title, v); tokenText = null; readTokens(); }) : null,
        ...extra.map((x) => select("x-" + x.id, x.label, x.options, x.value, (v) => { x.value = v; save("sel:" + config.title + ":" + x.id, v); render(); })),
        select("theme", "Appearance", { dark: "Dark", light: "Light" }, state.theme, (v) => { state.theme = v; save("theme", v); render(); }),
        hasTokens ? select("mode", "Draw", { tokens: "Tokens", wire: "Wireframe" }, state.mode, (v) => { state.mode = v; save("mode", v); render(); }) : null,
        select("wallpaper", "Wallpaper", Object.fromEntries(Object.entries(WALLPAPERS).map(([k, v]) => [k, v.name])), state.wallpaper, (v) => { state.wallpaper = v; save("wallpaper:" + config.title, v); render(); }),
        select("screen", "Screen", Object.fromEntries(Object.entries(SCREENS).map(([k, v]) => [k, v.name])), state.screen, (v) => { state.screen = v; render(); }),
        toggle("whole", "Whole screen", state.whole, (v) => { state.whole = v; save("whole", v); fit(); }),
        toggle("actual", "Actual size", state.actual, (v) => { state.actual = v; fit(); }),
        toggle("rm", "Reduce motion", Isle.flags.reduceMotion, (v) => { Isle.flags.reduceMotion = v; save("flags", Isle.flags); render(); }),
        toggle("rt", "Reduce transparency", Isle.flags.reduceTransparency, (v) => { Isle.flags.reduceTransparency = v; save("flags", Isle.flags); render(); }),
        toggle("ic", "Increase contrast", Isle.flags.increaseContrast, (v) => { Isle.flags.increaseContrast = v; save("flags", Isle.flags); render(); }),
      ].filter(Boolean));
    }

    function buildTabs() {
      tabs.replaceChildren(...config.surfaces.map((s) =>
        h("button", { class: "hb-tab", "aria-current": s.id === state.surface ? "page" : false,
          onclick: () => { state.surface = s.id; state.st = s.states[0].id; render(); } }, s.name)));
      const surface = config.surfaces.find((s) => s.id === state.surface);
      states.replaceChildren(...surface.states.map((st) =>
        h("button", { class: "hb-state", role: "tab", "aria-selected": st.id === state.st ? "true" : "false",
          onclick: () => { state.st = st.id; render(); } }, st.name)));
    }

    let tokenVars = null;
    let tokenText = null;
    async function readTokens() {
      if (!hasTokens) return;
      try {
        const res = await fetch(tokensPath(), { cache: "no-store" });
        if (!res.ok) throw new Error("tokens.kdl: " + res.status);
        const text = await res.text();
        if (text === tokenText) return;
        tokenText = text;
        tokenVars = tokensToVars(parseKdl(text));
        error.hidden = true;
        render();
      } catch (e) {
        error.textContent = e.message;
        error.hidden = false;
      }
    }

    function applyVars() {
      screen.removeAttribute("style");
      const size = SCREENS[state.screen];
      screen.style.width = size.w + "px";
      screen.style.height = size.h + "px";
      if (state.mode === "tokens" && tokenVars) {
        for (const set of [tokenVars.any, tokenVars[state.theme]])
          for (const [k, v] of Object.entries(set)) screen.style.setProperty(k, v);
        // Increase Contrast: `colour { contrast { … } }` replaces the roles it names.
        if (Isle.flags.increaseContrast)
          for (const [k, v] of Object.entries(tokenVars[state.theme]))
            if (k.startsWith("--colour-contrast-")) screen.style.setProperty("--colour-" + k.slice(18), v);
      }
    }

    let stopAnim = [];
    let current = null;
    function render() {
      stopAnim.forEach((f) => f());
      stopAnim = [];
      buildTabs();
      buildControls();
      const surface = config.surfaces.find((s) => s.id === state.surface);
      const st = surface.states.find((x) => x.id === state.st) || surface.states[0];
      current = st;
      history.replaceState(null, "", "#" + surface.id + "." + st.id);

      screen.dataset.mode = state.mode;
      if (directions) screen.dataset.direction = state.direction;
      screen.dataset.themeScreen = state.theme;
      screen.dataset.rt = Isle.flags.reduceTransparency ? "1" : "0";
      screen.dataset.ic = Isle.flags.increaseContrast ? "1" : "0";
      applyVars();
      for (const x of extra) x.apply(screen, x.value);
      const wp = WALLPAPERS[state.wallpaper];
      wall.style.background = wp.wire || state.mode === "wire" ? "var(--wf-wall)" : wp.css;

      layer.replaceChildren();
      const ctx = {
        theme: state.theme, mode: state.mode, flags: { ...Isle.flags }, screen: SCREENS[state.screen],
        h, spring: (o, f) => { const stop = spring(o, f); stopAnim.push(stop); return stop; },
        replay: () => render(),
      };
      st.render(layer, ctx);
      if (config.afterRender) config.afterRender(layer, ctx);

      const dir = directions ? directions.find((d) => d.id === state.direction) : null;
      note.replaceChildren(...[
        h("h2", {}, (dir ? dir.name + " · " : "") + surface.name + " · " + st.name),
        dir && dir.note ? h("p", { class: "hb-dir" }, dir.note) : null,
        st.note ? h("p", {}, st.note) : null,
        surface.sheet ? h("p", { class: "hb-sheet" }, "Sheet: ", h("code", {}, surface.sheet)) : null].filter(Boolean));
      fit();
    }

    // A state may name a crop [x, y, w, h] in screen points: the board
    // shows that region filling the width unless "Whole screen" is on.
    function fit() {
      const size = SCREENS[state.screen];
      viewport.classList.toggle("is-actual", state.actual);
      const crop = !state.whole && current && current.crop && state.screen === "ultrawide" ? current.crop : [0, 0, size.w, size.h];
      const scale = state.actual ? 1 : viewport.clientWidth / crop[2];
      screen.style.transform = "translate(" + -crop[0] * scale + "px," + -crop[1] * scale + "px) scale(" + scale + ")";
      scaler.style.width = crop[2] * scale + "px";
      scaler.style.height = crop[3] * scale + "px";
      scaler.style.overflow = "hidden";
    }

    new ResizeObserver(fit).observe(viewport);
    render();
    if (hasTokens) {
      readTokens();
      if (/^(localhost|127\.0\.0\.1)$/.test(location.hostname)) setInterval(readTokens, 1000);
    }
    return { render };
  };

  window.Isle = Isle;
})();
