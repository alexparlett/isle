--------------------------------------------------------------------------------
--  Drag-to-edge window snapping
--
--  Hyprland has no Aero Snap. What it does have:
--    - tiled windows arrange themselves, and dragging one rearranges the tree
--    - `general.snap` pulls floating windows to edges and each other on release
--    - `binds.drag_threshold` plus the `drag` bind flag, which fires on release
--      only if the cursor actually moved
--
--  That last one is enough to build the missing piece. On releasing a ⌘-drag,
--  if the cursor ended in an edge or corner zone and the window is floating,
--  it snaps to that region. Tiled windows are left alone — the tiler already
--  handled the drag, and second-guessing it would be worse than useless.
--
--  Zones on a 3440x1440 ultrawide:
--
--      ┌──────────┬───────────────┬──────────┐
--      │ ¼        │   maximise    │        ¼ │   top edge and corners
--      ├──────────┼───────────────┼──────────┤
--      │ left ½   │               │  right ½ │   side edges
--      ├──────────┼───────────────┼──────────┤
--      │ ¼        │  centre ⅓     │        ¼ │   bottom edge and corners
--      └──────────┴───────────────┴──────────┘
--
--  Halves are 1720px here, which is a lot of window. The bottom-centre zone
--  gives the centred third instead, which is the shape that actually gets used
--  on a screen this wide.
--------------------------------------------------------------------------------

local EDGE = 40   -- how close to an edge counts as being in its zone
local GAP  = 10   -- matches general.gaps_out

-- Monitor object field names are not part of the documented API, so every read
-- is guarded and falls back to the geometry monitors.lua configures.
local FALLBACK = { x = 0, y = 0, width = 3440, height = 1440, top = 50, bottom = 10 }

local function geometry()
    local ok, g = pcall(function()
        local m = hl.get_active_monitor()
        if type(m) ~= "table" then return nil end

        local w = tonumber(m.width) or 0
        local h = tonumber(m.height) or 0
        if w <= 0 or h <= 0 then return nil end

        -- `reserved` is where the bar lives. Order matches hyprctl monitors:
        -- top, bottom, left, right.
        local r = m.reserved or {}
        local top = tonumber(r[1]) or tonumber(r.top) or 0
        local bottom = tonumber(r[2]) or tonumber(r.bottom) or 0

        return {
            x = tonumber(m.x) or 0,
            y = tonumber(m.y) or 0,
            width = w,
            height = h,
            top = top > 0 and top + GAP or FALLBACK.top,
            bottom = bottom > 0 and bottom + GAP or GAP,
        }
    end)
    return (ok and g) or FALLBACK
end

--- Which zone is (cx, cy) in? Returns nil when the cursor is nowhere near an edge.
local function zone_for(cx, cy, g)
    local left   = cx <= g.x + EDGE
    local right  = cx >= g.x + g.width - EDGE
    local top    = cy <= g.y + EDGE
    local bottom = cy >= g.y + g.height - EDGE

    if top and not (left or right) then return "maximise" end
    if left and top then return "top-left" end
    if right and top then return "top-right" end
    if left and bottom then return "bottom-left" end
    if right and bottom then return "bottom-right" end
    if left then return "left" end
    if right then return "right" end
    if bottom then return "centre" end
    return nil
end

local function place(zone, g)
    local usable_h = g.height - g.top - g.bottom
    local half_w   = math.floor((g.width - GAP * 3) / 2)
    local half_h   = math.floor((usable_h - GAP) / 2)
    local third_w  = math.floor((g.width - GAP * 4) / 3)

    local right_x  = g.x + g.width - half_w - GAP
    local bottom_y = g.y + g.top + half_h + GAP

    local rects = {
        ["left"]         = { g.x + GAP, g.y + g.top, half_w, usable_h },
        ["right"]        = { right_x,   g.y + g.top, half_w, usable_h },
        ["left-third"]   = { g.x + GAP, g.y + g.top, third_w, usable_h },
        ["right-third"]  = { g.x + g.width - third_w - GAP, g.y + g.top, third_w, usable_h },
        ["maximise"]     = { g.x + GAP, g.y + g.top, g.width - GAP * 2, usable_h },
        ["centre"]       = { g.x + math.floor((g.width - third_w) / 2), g.y + g.top, third_w, usable_h },
        ["top-left"]     = { g.x + GAP, g.y + g.top, half_w, half_h },
        ["top-right"]    = { right_x,   g.y + g.top, half_w, half_h },
        ["bottom-left"]  = { g.x + GAP, bottom_y,    half_w, half_h },
        ["bottom-right"] = { right_x,   bottom_y,    half_w, half_h },
    }

    local r = rects[zone]
    if not r then return end

    hl.dispatch(hl.dsp.window.resize({ x = r[3], y = r[4], relative = false }))
    hl.dispatch(hl.dsp.window.move({ x = r[1], y = r[2], relative = false }))
end

local function snap_on_release()
    local ok = pcall(function()
        local w = hl.get_active_window()
        -- Only floating windows. A tiled window's drag was already handled by
        -- the layout, and overriding it would fight the tiler.
        if type(w) ~= "table" or not w.floating then return end

        local pos = hl.get_cursor_pos()
        if type(pos) ~= "table" then return end

        local g = geometry()
        local zone = zone_for(tonumber(pos.x) or -1, tonumber(pos.y) or -1, g)
        if zone then place(zone, g) end
    end)
    return ok
end

-- Distinguishing a click from a drag needs a threshold; without one every
-- click would count as a zero-pixel drag.
hl.config({ binds = { drag_threshold = 8 } })

-- Two binds on the same button: the first does the interactive move, the
-- second fires on release once the cursor has travelled past the threshold.
-- binds.lua owns the drag itself; this only adds the release behaviour.
hl.bind("SUPER + mouse:272", snap_on_release, { mouse = true, drag = true })

--------------------------------------------------------------------------------
--  Keyboard equivalents
--
--  Same geometry, so the keyboard and the mouse always agree — and unlike the
--  hardcoded pixel values these replace, they follow the bar's actual reserved
--  height rather than assuming it.
--------------------------------------------------------------------------------

local function snap_to(zone)
    return function()
        hl.dispatch(hl.dsp.window.float({ action = "enable" }))
        place(zone, geometry())
    end
end

hl.bind("ALT + SUPER + 1", snap_to("left-third"),  { description = "Snap to left third" })
hl.bind("ALT + SUPER + 2", snap_to("centre"),      { description = "Snap to centre third" })
hl.bind("ALT + SUPER + 3", snap_to("right-third"), { description = "Snap to right third" })
hl.bind("ALT + SUPER + 4", snap_to("left"),        { description = "Snap to left half" })
hl.bind("ALT + SUPER + 5", snap_to("right"),       { description = "Snap to right half" })
hl.bind("ALT + SUPER + 6", snap_to("maximise"),    { description = "Snap to full width" })

return { zone_for = zone_for, geometry = geometry, place = place }
