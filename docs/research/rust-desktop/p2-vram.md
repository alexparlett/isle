# Compositor VRAM on the host (P2)

Measured 2026-10-03 on the host (RTX 5070 Ti, NVIDIA 615.71.09), the
compositor nested under Hyprland with the winit backend, debug build at
`fcace6d`, read from `nvidia-smi`'s process table.

| Run | At start | After 30 kitty windows opened and closed, one open |
|---|---|---|
| No application profile | 26 MiB | 46 MiB |
| `data/nvidia/50-isle.json` in `~/.nv/nvidia-application-profiles-rc.d/` | 26 MiB | 46 MiB |
| No profile, again | 26 MiB | 46 MiB |

Nested, the profile makes no difference. The growth it addresses is the
driver keeping freed buffers in its heap (niri wiki `Nvidia.md`, about
100 MiB with the profile against near 1 GiB without, over a session), which
involves the GBM buffers of the DRM path and hours of use, not a short
nested run. The comparison that counts is on the host's DRM, over a long
session: the host smoke run in P2 and the 24-hour soak in `compositor.md`'s
acceptance bars. Whether the user profile directory was read at all is not
shown by these numbers.
