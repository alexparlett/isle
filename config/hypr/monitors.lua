--------------------------------------------------------------------------------
--  Monitors
--  https://wiki.hypr.land/configuring/core/monitors/
--
--  AOC U34G2G4R3 — 3440x1440 ultrawide on DP-4 (NVIDIA). EDID advertises
--  60 / 100 / 120 / 144 Hz; `hyprctl monitors all` lists what is really there.
--------------------------------------------------------------------------------

local MAIN = "DP-4"

hl.monitor({
    output   = MAIN,
    mode     = "3440x1440@144",
    position = "0x0",
    scale    = 1,
    vrr      = 2, -- fullscreen only; see look.lua for why
})

-- Fallback for anything plugged in later: preferred mode, placed to the right.
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })

-- Pin the numbered workspaces to the ultrawide so they stay put if a second
-- screen ever shows up.
for i = 1, 10 do
    hl.workspace_rule({
        workspace = tostring(i),
        monitor   = MAIN,
        default   = (i == 1),
    })
end

return { main = MAIN }
