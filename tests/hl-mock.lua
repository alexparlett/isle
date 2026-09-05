-- A stand-in for Hyprland's `hl` API, built from the documented surface at
-- https://wiki.hypr.land/configuring/
--
-- The point is not to emulate Hyprland. It is to make every call in the config
-- resolve against a known-good list of names, so a typo like
-- `hl.dsp.window.centre()` or `hl.workspace_rules{}` fails here rather than at
-- login with a black screen and a log to go spelunking in.
--
-- Anything not listed raises. That is the whole mechanism: an __index that
-- errors on unknown keys turns "silently does nothing" into "test fails".

local M = {}

M.calls = { config = {}, bind = {}, window_rule = {}, layer_rule = {},
            workspace_rule = {}, monitor = {}, env = {}, on = {}, curve = {},
            animation = {}, gesture = {}, device = {}, exec_cmd = {}, dispatch = {} }

local function record(bucket, value)
    table.insert(M.calls[bucket], value)
end

-- Guard a table so unknown keys raise with a useful message.
local function sealed(name, tbl)
    return setmetatable(tbl, {
        __index = function(_, key)
            error(("%s.%s does not exist in the Hyprland API"):format(name, tostring(key)), 3)
        end,
    })
end

-- Dispatchers return opaque markers; hl.bind and hl.dispatch just need a value.
local function dispatcher(path)
    return function(...)
        return { __dispatcher = path, args = ... }
    end
end

local function dispatch_group(name, methods)
    local t = {}
    for _, m in ipairs(methods) do
        t[m] = dispatcher(name .. "." .. m)
    end
    return sealed("hl.dsp." .. name, t)
end

-- https://wiki.hypr.land/configuring/core/dispatchers/
local dsp = {
    exec_cmd = dispatcher("exec_cmd"),
    exec_raw = dispatcher("exec_raw"),
    focus = dispatcher("focus"),
    exit = dispatcher("exit"),
    submap = dispatcher("submap"),
    pass = dispatcher("pass"),
    send_shortcut = dispatcher("send_shortcut"),
    send_key_state = dispatcher("send_key_state"),
    layout = dispatcher("layout"),
    dpms = dispatcher("dpms"),
    event = dispatcher("event"),
    global = dispatcher("global"),
    force_idle = dispatcher("force_idle"),
    no_op = dispatcher("no_op"),
    force_renderer_reload = dispatcher("force_renderer_reload"),
    release_input_capture = dispatcher("release_input_capture"),

    window = dispatch_group("window", {
        "close", "kill", "signal", "float", "fullscreen", "fullscreen_state",
        "pseudo", "move", "swap", "center", "cycle_next", "tag", "clear_tags",
        "toggle_swallow", "pin", "alter_zorder", "set_prop", "deny_from_group",
        "drag", "resize", "bring_to_top",
    }),
    workspace = dispatch_group("workspace", {
        "change_id", "rename", "move", "swap_monitors", "toggle_special",
    }),
    group = dispatch_group("group", {
        "toggle", "next", "prev", "active", "move_window", "lock", "lock_active",
    }),
    cursor = dispatch_group("cursor", { "move_to_corner", "move" }),
}

local handle = {
    set_enabled = function() end,
    is_enabled = function() return true end,
    unbind = function() end,
    remove = function() end,
}

local hl
hl = {
    dsp = sealed("hl.dsp", dsp),

    config = function(t) record("config", t) return t end,
    env = function(k, v) record("env", { k, v }) end,
    monitor = function(t) record("monitor", t) end,
    bind = function(keys, action, flags)
        assert(type(keys) == "string", "bind keys must be a string")
        assert(action ~= nil, "bind for " .. keys .. " has no action")
        record("bind", { keys = keys, action = action, flags = flags })
        return handle
    end,
    unbind = function() end,
    dispatch = function(d) record("dispatch", d) end,
    exec_cmd = function(c) record("exec_cmd", c) end,
    on = function(event, fn)
        assert(type(event) == "string", "hl.on event must be a string")
        assert(type(fn) == "function", "hl.on handler must be a function")
        record("on", { event = event, fn = fn })
    end,
    window_rule = function(t) record("window_rule", t) return handle end,
    layer_rule = function(t) record("layer_rule", t) return handle end,
    workspace_rule = function(t) record("workspace_rule", t) return handle end,
    curve = function(n, t) record("curve", { n, t }) end,
    animation = function(t) record("animation", t) end,
    gesture = function(t) record("gesture", t) end,
    device = function(t) record("device", t) end,
    permission = function() end,
    timer = function() return handle end,
    notification = sealed("hl.notification", { create = function() end }),

    get_config = function() return {} end,
    get_active_window = function() return nil end,
    get_windows = function() return {} end,
    get_window = function() return nil end,
    get_urgent_window = function() return nil end,
    get_workspaces = function() return {} end,
    get_workspace = function() return nil end,
    get_active_workspace = function() return nil end,
    get_active_special_workspace = function() return nil end,
    get_monitors = function() return {} end,
    get_monitor = function() return nil end,
    get_active_monitor = function() return nil end,
    get_monitor_at = function() return nil end,
    get_monitor_at_cursor = function() return nil end,
    get_cursor_pos = function() return { x = 0, y = 0 } end,
    get_last_window = function() return nil end,
    get_last_workspace = function() return nil end,
    get_layers = function() return {} end,
    get_workspace_windows = function() return {} end,
    get_current_submap = function() return "" end,
    get_loaded_plugins = function() return {} end,
    is_key_down = function() return false end,
    version = function() return { tag = "mock" } end,
    exec_scheduled_prop_refresh_immediately = function() end,
}

M.hl = sealed("hl", hl)
return M
