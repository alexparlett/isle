#!/usr/bin/env lua
--------------------------------------------------------------------------------
--  Render component configs from core/theme.lua
--
--      lua tools/render-theme.lua [--check]
--
--  The Catppuccin palette used to exist as a hardcoded copy in six files —
--  waybar, rofi, swaync, kitty, wlogout and hyprlock — which had already
--  started to drift. This makes theme.lua the only place a colour is written
--  down, and everything else a template rendered from it.
--
--  One mechanism for every format: `*.in` templates with {{placeholders}},
--  substituted and written next to the template. No per-format include
--  semantics to get wrong, and the same approach already used for
--  hypridle.conf.in.
--
--  Placeholders:
--      {{css:accent}}        #cba6f7          CSS, rasi, kitty
--      {{rgba:accent}}       rgba(cba6f7ff)   Hyprland and hyprlock
--      {{rgba:accent:ee}}    rgba(cba6f7ee)   with alpha
--      {{raw:accent}}        cba6f7ff         slurp, bare RGBA
--      {{hex:accent}}        cba6f7           when the format adds its own prefix
--      {{font.family}}       JetBrainsMono Nerd Font
--      {{metrics.radius}}    12
--
--  Names resolve as semantic roles first (accent, urgent, surface…), falling
--  back to raw palette names (mauve, red…). Prefer roles: they are what makes
--  a palette swap a one-line change.
--
--  --check renders to memory and reports whether anything on disk is stale,
--  without writing. That is what CI and tests/run.sh use.
--------------------------------------------------------------------------------

package.path = "config/hypr/?.lua;config/hypr/?/init.lua;" .. package.path

local ok, theme = pcall(require, "core.theme")
if not ok then
    io.stderr:write("cannot load core/theme.lua — run this from the repo root\n")
    os.exit(1)
end

local check_only = (arg[1] == "--check")

--- Every template, relative to config/. Add a file here and it is rendered.
local TEMPLATES = {
    "waybar/style.css",
    "waybar/config.jsonc",
    "rofi/catppuccin-mocha.rasi",
    "rofi/launchbar.rasi",
    "swaync/style.css",
    "swayosd/style.css",
    "wlogout/style.css",
    "kitty/kitty.conf",
    "hypr/hyprlock.conf",
}

local RESOLVERS = {
    css  = function(name, alpha) return alpha and ("#" .. theme.hex(name) .. alpha) or theme.css(name) end,
    rgba = function(name, alpha) return theme.rgba(name, alpha) end,
    raw  = function(name, alpha) return theme.raw(name, alpha) end,
    hex  = function(name) return theme.hex(name) end,
}

--- Look up a dotted path like "font.family" or "metrics.radius".
local function lookup(path)
    local node = theme
    for part in path:gmatch("[^.]+") do
        if type(node) ~= "table" then return nil end
        node = node[part]
    end
    return node
end

local errors = {}

local function render(body, where)
    -- {{kind:name}} or {{kind:name:alpha}}
    body = body:gsub("{{(%w+):([%w_]+):?(%w*)}}", function(kind, name, alpha)
        local resolver = RESOLVERS[kind]
        if not resolver then
            table.insert(errors, ("%s: unknown placeholder kind '%s'"):format(where, kind))
            return "{{" .. kind .. ":" .. name .. "}}"
        end
        local okr, value = pcall(resolver, name, alpha ~= "" and alpha or nil)
        if not okr then
            table.insert(errors, ("%s: %s"):format(where, tostring(value):gsub("^.*: ", "")))
            return "INVALID"
        end
        return value
    end)

    -- {{font.family}}, {{metrics.gap}} and friends.
    body = body:gsub("{{([%w_]+%.[%w_]+)}}", function(path)
        local value = lookup(path)
        if value == nil then
            table.insert(errors, ("%s: unknown theme path '%s'"):format(where, path))
            return "INVALID"
        end
        return tostring(value)
    end)

    return body
end

local function read(path)
    local file = io.open(path, "r")
    if not file then return nil end
    local body = file:read("*a")
    file:close()
    return body
end

local banner = {
    ["css"]  = "/* Generated from core/theme.lua by tools/render-theme.lua — edit the .in */\n",
    ["rasi"] = "/* Generated from core/theme.lua by tools/render-theme.lua — edit the .in */\n",
    ["conf"] = "# Generated from core/theme.lua by tools/render-theme.lua — edit the .in\n",
    ["jsonc"] = "// Generated from core/theme.lua by tools/render-theme.lua — edit the .in\n",
}

local written, stale, missing = 0, {}, {}

for _, relative in ipairs(TEMPLATES) do
    local target = "config/" .. relative
    local template = target .. ".in"

    local body = read(template)
    if not body then
        table.insert(missing, template)
        goto continue
    end

    do
        local extension = relative:match("%.([%w]+)$") or "conf"
        local output = (banner[extension] or banner.conf) .. render(body, relative)

        if check_only then
            if read(target) ~= output then table.insert(stale, relative) end
        else
            local file, err = io.open(target, "w")
            if not file then
                table.insert(errors, ("cannot write %s: %s"):format(target, tostring(err)))
            else
                file:write(output)
                file:close()
                written = written + 1
            end
        end
    end

    ::continue::
end

for _, path in ipairs(missing) do
    print("  no template: " .. path)
end

if #errors > 0 then
    io.stderr:write("\n")
    for _, err in ipairs(errors) do io.stderr:write("  error: " .. err .. "\n") end
    os.exit(1)
end

if check_only then
    if #stale > 0 then
        io.stderr:write("  stale, re-run tools/render-theme.lua:\n")
        for _, path in ipairs(stale) do io.stderr:write("    " .. path .. "\n") end
        os.exit(1)
    end
    print(("  all %d generated files match core/theme.lua"):format(#TEMPLATES - #missing))
else
    print(("  rendered %d files from core/theme.lua"):format(written))
end
