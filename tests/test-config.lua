-- Load the whole Hyprland config against the mock API and assert on what it
-- produced. Run with:  lua tests/test-config.lua
--
-- This catches the class of mistake that is otherwise only discovered by
-- logging in: a dispatcher that does not exist, a bind with no action, a rule
-- table with a misspelled field, a module that forgets to require another.

package.path = "./config/hypr/?.lua;./tests/?.lua;" .. package.path

local mock = require("hl-mock")
_G.hl = mock.hl

local failures, checks = {}, 0

local function check(name, condition, detail)
    checks = checks + 1
    if not condition then
        table.insert(failures, detail and (name .. " — " .. detail) or name)
    end
end

-- --- load every module, in the order hyprland.lua does ---------------------

local MODULES = { "env", "monitors", "input", "look", "binds", "snap", "rules", "gaming", "dnd", "autostart" }

for _, name in ipairs(MODULES) do
    local ok, err = pcall(require, name)
    check("module " .. name .. " loads", ok, tostring(err))
end

local c = mock.calls

-- --- binds -----------------------------------------------------------------

check("binds were registered", #c.bind > 40, ("only %d"):format(#c.bind))

local seen, duplicates = {}, {}
for _, b in ipairs(c.bind) do
    -- A dispatcher marker, or a Lua function for the multi-step ones.
    local kind = type(b.action)
    check("bind " .. b.keys .. " has a usable action",
          kind == "function" or (kind == "table" and b.action.__dispatcher ~= nil),
          "got " .. kind)

    -- `click`, `drag` and `release` are the documented way to hang different
    -- gestures off one button, so those are not collisions.
    local gesture = type(b.flags) == "table"
        and (b.flags.drag or b.flags.click or b.flags.release or b.flags.long_press)

    -- Modifier order matters for duplicate detection, not for Hyprland.
    local mods, key = {}, nil
    for raw in b.keys:gmatch("[^+]+") do
        local part = raw:match("^%s*(.-)%s*$")
        if part:match("^[A-Z]+$") and #part > 1 then table.insert(mods, part) else key = part end
    end
    table.sort(mods)
    local norm = table.concat(mods, "+") .. "|" .. tostring(key)
    if not gesture then
        if seen[norm] then table.insert(duplicates, b.keys .. "  collides with  " .. seen[norm]) end
        seen[norm] = b.keys
    end
end
check("no duplicate keybinds", #duplicates == 0, table.concat(duplicates, "; "))

-- Descriptions drive the ⌘? cheatsheet.
local described = 0
for _, b in ipairs(c.bind) do
    if type(b.flags) == "table" and b.flags.description then described = described + 1 end
end
check("most binds carry a description", described > #c.bind * 0.6,
      ("%d of %d"):format(described, #c.bind))

-- --- rules -----------------------------------------------------------------

check("window rules registered", #c.window_rule > 10, ("%d"):format(#c.window_rule))
check("layer rules registered", #c.layer_rule >= 5, ("%d"):format(#c.layer_rule))

local VALID_MATCH = {
    class = true, title = true, initial_class = true, initial_title = true,
    xwayland = true, float = true, fullscreen = true, pin = true, focus = true,
    workspace = true, tag = true, modal = true, group = true, content = true,
    xdg_tag = true, fullscreen_state_client = true, fullscreen_state_internal = true,
}
for _, r in ipairs(c.window_rule) do
    check("window rule has a match table", type(r.match) == "table", r.name or "unnamed")
    if type(r.match) == "table" then
        for k in pairs(r.match) do
            check("window rule match field '" .. k .. "' is valid", VALID_MATCH[k],
                  (r.name or "unnamed") .. " uses unknown match prop " .. k)
        end
    end
end

for _, r in ipairs(c.layer_rule) do
    check("layer rule matches on namespace",
          type(r.match) == "table" and r.match.namespace ~= nil, r.name or "unnamed")
end

-- --- monitors and workspaces ----------------------------------------------

check("a monitor rule exists", #c.monitor >= 1)
check("a fallback monitor rule exists", (function()
    for _, m in ipairs(c.monitor) do if m.output == "" then return true end end
    return false
end)(), "no `output = \"\"` catch-all, so a new display gets no rule")

check("workspaces are pinned", #c.workspace_rule >= 10, ("%d"):format(#c.workspace_rule))

-- --- events ----------------------------------------------------------------

local events = {}
for _, e in ipairs(c.on) do events[e.event] = true end
check("autostart hooks hyprland.start", events["hyprland.start"] == true)
check("DND reacts to screen sharing", events["screenshare.state"] == true)
check("DND reacts to fullscreen", events["window.fullscreen"] == true)

local KNOWN_EVENTS = {
    ["hyprland.start"] = true, ["hyprland.shutdown"] = true, ["window.open"] = true,
    ["window.close"] = true, ["window.active"] = true, ["window.fullscreen"] = true,
    ["window.title"] = true, ["window.urgent"] = true, ["window.pin"] = true,
    ["screenshare.state"] = true, ["workspace.active"] = true, ["config.reloaded"] = true,
    ["monitor.added"] = true, ["monitor.removed"] = true, ["window.destroy"] = true,
}
for _, e in ipairs(c.on) do
    check("event '" .. e.event .. "' is a real event", KNOWN_EVENTS[e.event] == true)
end

-- --- environment -----------------------------------------------------------

local env = {}
for _, e in ipairs(c.env) do env[e[1]] = e[2] end
check("NVIDIA VA-API driver set", env.LIBVA_DRIVER_NAME == "nvidia")
check("GBM_BACKEND is NOT set", env.GBM_BACKEND == nil,
      "it is counterproductive on driver 555+")
check("WLR_NO_HARDWARE_CURSORS is NOT set", env.WLR_NO_HARDWARE_CURSORS == nil)
check("primary GPU pinned by stable path",
      type(env.AQ_DRM_DEVICES) == "string" and env.AQ_DRM_DEVICES:match("by%-path"),
      "card numbers are not stable across boots")

-- --- config ----------------------------------------------------------------

local cfg = {}
for _, t in ipairs(c.config) do
    for section, values in pairs(t) do
        cfg[section] = cfg[section] or {}
        if type(values) == "table" then
            for k, v in pairs(values) do cfg[section][k] = v end
        end
    end
end
check("tearing is enabled for games to opt into", cfg.general and cfg.general.allow_tearing == true)
check("VRR is fullscreen-only", cfg.misc and cfg.misc.vrr == 2,
      "full-desktop VRR flickers on this panel")
check("keyboard layout is gb", cfg.input and cfg.input.kb_layout == "gb")

-- --- snap zones ------------------------------------------------------------
--
-- Pure geometry, so it can be checked properly rather than eyeballed.

local snap = require("snap")
local g = { x = 0, y = 0, width = 3440, height = 1440, top = 50, bottom = 10 }

local zones = {
    { 5,    700,  "left",         "left edge" },
    { 3435, 700,  "right",        "right edge" },
    { 1700, 5,    "maximise",     "top edge, away from corners" },
    { 5,    5,    "top-left",     "top-left corner" },
    { 3435, 5,    "top-right",    "top-right corner" },
    { 5,    1435, "bottom-left",  "bottom-left corner" },
    { 3435, 1435, "bottom-right", "bottom-right corner" },
    { 1700, 1435, "centre",       "bottom edge, away from corners" },
    { 1700, 700,  nil,            "middle of the screen snaps to nothing" },
    { 1700, 200,  nil,            "just below the top edge does not snap" },
}

for _, z in ipairs(zones) do
    local got = snap.zone_for(z[1], z[2], g)
    check("snap zone: " .. z[4], got == z[3],
          ("(%d,%d) gave %s, expected %s"):format(z[1], z[2], tostring(got), tostring(z[3])))
end

-- Every zone must produce a rectangle that fits on the monitor, below the bar.
for _, zone in ipairs({ "left", "right", "left-third", "right-third", "centre",
                        "maximise", "top-left", "top-right", "bottom-left", "bottom-right" }) do
    local placed = {}
    local real_dispatch = hl.dispatch
    -- Capture the resize/move pair without a live compositor.
    _G.hl = setmetatable({ dispatch = function(d) table.insert(placed, d) end }, { __index = mock.hl })
    snap.place(zone, g)
    _G.hl = mock.hl
    hl.dispatch = real_dispatch

    check("zone '" .. zone .. "' emits a resize and a move", #placed == 2, ("%d calls"):format(#placed))
    if #placed == 2 then
        local size, pos = placed[1].args, placed[2].args
        local fits = pos.x >= g.x and pos.y >= g.y + g.top
                 and pos.x + size.x <= g.x + g.width
                 and pos.y + size.y <= g.y + g.height - g.bottom + 1
        check("zone '" .. zone .. "' fits on screen below the bar", fits,
              ("%dx%d at %d,%d"):format(size.x, size.y, pos.x, pos.y))
    end
end

-- --- report ----------------------------------------------------------------

print()
if #failures == 0 then
    print(("  all %d checks passed"):format(checks))
    os.exit(0)
end

print(("  %d of %d checks FAILED:"):format(#failures, checks))
for _, f in ipairs(failures) do print("    ✗ " .. f) end
os.exit(1)
