--------------------------------------------------------------------------------
--  Core — module loader
--
--  Modules are loaded through here rather than being `require`d directly by the
--  entrypoint, for three reasons:
--
--  1. **Failure is isolated.** A module that throws is caught, reported, and
--     skipped. Previously one bad line anywhere took the whole config with it,
--     and Hyprland's response to that is a desktop with no keybinds.
--  2. **Ordering is explicit and in one list**, instead of implied by the order
--     of require calls scattered down a file.
--  3. **Modules get their dependencies passed in**, so they never reach for a
--     global. Every module is a function of (theme, options) and is therefore
--     testable without a compositor.
--
--  The contract: a module returns a table with `setup(ctx)`. Anything else is
--  a load error and says so.
--------------------------------------------------------------------------------

local M = {}

local function notify(text, urgent)
    -- hyprctl rather than notify-send: this runs before autostart, so a
    -- notification daemon may not exist yet, but the compositor always does.
    local level = urgent and 3 or 1
    os.execute(("hyprctl notify %d 8000 0 'config: %s' >/dev/null 2>&1")
        :format(level, text:gsub("'", "")))
end

--- Load and set up each module in order, isolating failures.
--- @param names string[] module names, relative to modules/
--- @param ctx table shared context handed to every module
--- @return table report { loaded = {...}, failed = { {name, err}, ... } }
function M.load(names, ctx)
    local report = { loaded = {}, failed = {} }

    for _, name in ipairs(names) do
        local path = "modules." .. name

        local ok, module = pcall(require, path)
        if not ok then
            table.insert(report.failed, { name = name, err = tostring(module) })
            notify(name .. " failed to load", true)
            goto continue
        end

        if type(module) ~= "table" or type(module.setup) ~= "function" then
            table.insert(report.failed, {
                name = name,
                err = "module must return a table with setup(ctx)",
            })
            notify(name .. " has no setup()", true)
            goto continue
        end

        local ran, err = pcall(module.setup, ctx)
        if ran then
            table.insert(report.loaded, name)
        else
            table.insert(report.failed, { name = name, err = tostring(err) })
            notify(name .. ": " .. tostring(err):sub(1, 60), true)
        end

        ::continue::
    end

    return report
end

--- Build the context every module receives. Nothing reaches for a global.
function M.context()
    local theme = require("core.theme")
    local options = require("core.options")
    local util = require("core.util")

    -- Machine-local overrides land before modules read options, so a local.lua
    -- can change behaviour rather than only re-stating it afterwards.
    local ok, overrides = pcall(require, "local")
    if ok and type(overrides) == "table" then
        util.deep_merge(options, overrides)
    end

    return { theme = theme, opt = options, util = util }
end

return M
