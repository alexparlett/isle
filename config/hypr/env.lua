--------------------------------------------------------------------------------
--  Environment variables
--  https://wiki.hypr.land/configuring/core/environment-variables/
--------------------------------------------------------------------------------

local theme = require("theme")

-- --- Toolkit backends -------------------------------------------------------
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("QT_QPA_PLATFORMTHEME", "qt6ct")
hl.env("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1")
hl.env("QT_AUTO_SCREEN_SCALE_FACTOR", "1")
hl.env("GDK_BACKEND", "wayland,x11,*")
hl.env("SDL_VIDEODRIVER", "wayland")
hl.env("CLUTTER_BACKEND", "wayland")
hl.env("MOZ_ENABLE_WAYLAND", "1")
hl.env("_JAVA_AWT_WM_NONREPARENTING", "1")

-- Electron/CEF apps: native Wayland plus explicit sync, which is what actually
-- stops the flickering on NVIDIA.
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")

-- --- NVIDIA (RTX 5070 Ti, nvidia-open) --------------------------------------
-- Blackwell requires the open kernel modules — the legacy proprietary ones do
-- not support 50xx at all. DRM modeset and fbdev are already on by default in
-- 570+, and Arch ships the modprobe drop-in, so there is nothing to set there.
--
-- Deliberately NOT set: GBM_BACKEND and WLR_NO_HARDWARE_CURSORS. Both were
-- workarounds for pre-555 drivers and cause more breakage than they fix now.
hl.env("LIBVA_DRIVER_NAME", "nvidia")
hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
hl.env("NVD_BACKEND", "direct")

-- This box has an AMD iGPU (Granite Ridge) alongside the RTX 5070 Ti, and the
-- only display hangs off the NVIDIA card. Pin aquamarine to the dGPU so it
-- never picks the headless iGPU as the primary renderer.
--
-- by-path, not /dev/dri/cardN: card numbering is reassigned at boot. 01:00.0 is
-- the NVIDIA PCI address on this machine (`lspci -d ::03xx` to re-check).
--
-- Only set when the node actually exists. Pinning aquamarine to a device that
-- is not there leaves it with no GPU at all and Hyprland does not start — which
-- is exactly what would happen in a VM, on a different machine, or if the card
-- moved slots. Absent the pin, aquamarine picks a device itself, which is the
-- right behaviour everywhere except this specific dual-GPU box.
local PRIMARY_GPU = "/dev/dri/by-path/pci-0000:01:00.0-card"

local node = io.open(PRIMARY_GPU, "r")
if node then
    node:close()
    hl.env("AQ_DRM_DEVICES", PRIMARY_GPU)
end

-- --- Cursor -----------------------------------------------------------------
hl.env("XCURSOR_THEME", theme.cursor.theme)
hl.env("XCURSOR_SIZE", tostring(theme.cursor.size))
hl.env("HYPRCURSOR_THEME", theme.cursor.theme)
hl.env("HYPRCURSOR_SIZE", tostring(theme.cursor.size))
