--------------------------------------------------------------------------------
--  Small shared helpers
--
--  Deliberately small. Anything that grows a policy decision belongs in a
--  module or in options.lua, not here.
--------------------------------------------------------------------------------

local M = {}

--- Merge `src` into `dst` in place, recursing into tables.
--- Used for local.lua overrides, where partially overriding a nested table
--- should not discard its siblings.
function M.deep_merge(dst, src)
    for key, value in pairs(src) do
        if type(value) == "table" and type(dst[key]) == "table" then
            M.deep_merge(dst[key], value)
        else
            dst[key] = value
        end
    end
    return dst
end

--- First executable in `candidates` that exists, else `fallback`.
--- Lets the config survive swapping Dolphin for Thunar without an edit.
function M.first_installed(candidates, fallback)
    for _, name in ipairs(candidates) do
        for _, dir in ipairs({ "/usr/bin/", "/usr/local/bin/" }) do
            local handle = io.open(dir .. name, "r")
            if handle then
                handle:close()
                return name
            end
        end
    end
    return fallback
end

--- Read a whole file, or nil. No error: absence is normal for optional config.
function M.read(path)
    local file = io.open(path, "r")
    if not file then return nil end
    local body = file:read("*a")
    file:close()
    return body
end

--- Parse `key = value` lines, ignoring blanks and # comments.
--- The format used by keymap and keybinds.conf: trivial to hand-edit, trivial
--- for a GUI to write, and needs no JSON parser in a config file.
function M.read_pairs(path)
    local out = {}
    local body = M.read(path)
    if not body then return out end

    for line in body:gmatch("[^\r\n]+") do
        if not line:match("^%s*#") then
            local key, value = line:match("^%s*(.-)%s*=%s*(.-)%s*$")
            if key and value and key ~= "" and value ~= "" then
                out[key] = value
            end
        end
    end
    return out
end

--- $HOME, without assuming it is set.
function M.home()
    return os.getenv("HOME") or ""
end

--- Path under ~/.config/hypr.
function M.hypr(rest)
    return M.home() .. "/.config/hypr" .. (rest and ("/" .. rest) or "")
end

--- Comparable form of a key combo: modifiers sorted and upper-cased, key last.
--- "ALT + SUPER + T" and "super + alt + t" both become "ALT+SUPER|t", which is
--- what makes collision detection and HyprMod interop reliable.
function M.normalise_keys(keys)
    local mods, key = {}, nil
    for raw in tostring(keys):gmatch("[^+]+") do
        local part = raw:match("^%s*(.-)%s*$")
        if part ~= "" then
            local upper = part:upper()
            if upper == "CONTROL" then upper = "CTRL" end
            if upper == "SUPER" or upper == "ALT" or upper == "CTRL" or upper == "SHIFT" then
                table.insert(mods, upper)
            else
                key = part
            end
        end
    end
    table.sort(mods)
    return table.concat(mods, "+") .. "|" .. string.lower(key or "")
end

return M
