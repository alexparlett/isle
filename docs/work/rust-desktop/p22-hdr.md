# P22 · HDR

Planned 2026-11-02. Reviewed by the user the same day: every
recommendation taken, with the drift of choice 3's port accepted. Built
now, ahead of P14c, P20 and P21, at the user's word. The evidence is in
`docs/research/rust-desktop/hdr-2026.md`, with the host's own probe in its
§3.2–3.4.

B23 put HDR after Isle replaces v1. It is built now: Isle already runs
the host's desktop (P14a, P14b). The host
monitor (AOC U34G2G4R3 on DP-4, RTX 5070 Ti, NVIDIA open 615.78.08) takes
HDR10, with an EDID peak of about 302 cd/m², so there is about 1.5 times
headroom over SDR's 203. That makes the plan's job narrower than "HDR"
sounds:
- HDR games and video show their highlights;
- the SDR desktop beside them looks as it does today;
- nothing about the change can leave the screen black.

Today every pixel path is 8-bit sRGB, blended in gamma space:
- the swapchain is `Argb8888`;
- Skia draws into `RGBA8` with no colour space;
- the shaders work on encoded values;
- captures, streams and the colour pick are 8-bit sRGB.

Neither Isle nor its Smithay fork serves colour management or touches
`Colorspace`, `HDR_OUTPUT_METADATA` or a colour pipeline. Night light is
the only colour transform: a CRTC `GAMMA_LUT` on the sRGB curve (B91).

What the evidence settles:
- **Upstream Smithay will not have colour management soon.** It waits on
  Drakulix's Vulkan renderer (PR #2165, an early draft), and may never
  come to GLES.
- **A community port exists.** `dividebysandwich/smithay` `ds-hdr-master`
  (MIT, 2026-07) carries the connector state and the protocol.
  franzjeger's COSMIC reference ran HDR10 at 3440x1440 on an RTX 5090
  with driver 610.57.04.
- **This host's NVIDIA KMS has the connector side.** It offers `Colorspace`
  (BT2020_RGB) and `HDR_OUTPUT_METADATA`. It has no `max bpc`. Its only
  10-bit order is ABGR (XB30, AB30), and its only FP16 formats are
  XB4H and AB4H.
- **The plane colour pipelines offer PQ curves only.** They have
  stages that cannot be bypassed, and KWin ships them disabled on every
  driver. The cursor plane takes AR24 alone and has no pipeline.
- **Colour state must ride a modeset.** Setting `Colorspace` in a commit
  of its own hung NVIDIA in the niri fork.

## What it answers to

- Decisions: B6 (the Smithay fork), B7 (Skia on the compositor's EGL),
  B23, B80 (captures re-render), B81, B82, B85, B86 (scan-out, overlays off
  on NVIDIA), B87, B88 (tearing on scan-out only), B91 (night light), B96
  (display changes and Keep), B132–B137 (P19p's bars), B140 (the greeter).
- Intent: `compositor.md` Outputs 8 and its HDR line (`:535`), the
  Not-served list (`:435`); `gaming.md` §2 and the HDR line (`:332`);
  `apps.md` Settings 13 (Displays).
- Design: `p1/structure/control-centre.md`'s Displays row (mode, scale,
  HDR, VRR and tearing per display); P13's Displays pane.
- Evidence: `hdr-2026.md`, `smithay-and-gaming.md` §2.1–2.3,
  `capture-2026.md` §7 (the pick's sRGB until P22).

## Choices this plan makes

### Scope

1. **One packet.** HDR10 only:
   PQ with BT.2020, on screens whose EDID lists ST2084. Not covered:
   - HLG, which the monitor does not list;
   - ICC profiles;
   - the AMD iGPU's outputs, which are P14c's.
2. **A spike before any of it** (22.0). A throwaway program in
   `spikes/hdr-modeset/` runs from a free console with the session
   stopped. It makes one atomic modeset with:
   - `Colorspace=BT2020_RGB`;
   - the EDID's metadata;
   - an XB30 framebuffer holding a PQ test pattern.

   It holds that for ten seconds, then sets SDR back. It also checks that
   NVIDIA's GLES can render to an FP16 texture (`EXT_color_buffer_half_float`,
   choice 11). The packet goes on only if both of these hold:
   - the monitor's OSD reports HDR;
   - the driver neither hangs nor leaves the screen black.

   If either fails, the plan comes back to the user before any code is
   written. The rollback to 610.57.04 named in the evidence is the first
   thing tried.

### Smithay

3. **Port the community pieces onto the fork's `isle` branch,** as two
   PRs like the fork's others:
   - **Connector colour state.** `Colorspace`, `HDR_OUTPUT_METADATA` and
     `max bpc` where it exists are staged on the surface and committed
     with the mode. They are probed per connector, read back from a
     foreign state on creation and on resume, and reset to SDR.
   - **wp-color-management-v1, version 3:** parametric descriptions,
     surface feedback, the Windows scRGB and BT.2100 descriptions.

   Taken from `ds-hdr-master` (MIT). The connector code is compared with
   franzjeger's, which keeps TEST_ONLY and the commit consistent. niri's
   and cosmic-comp's colour code is GPL-3.0: it is read for reference,
   not copied.

   **Not taken:**
   - The fork's GLES shader overrides. They change `renderer/gles/mod.rs`,
     which upstream changes most. Isle converts colour in its own elements
     (choice 13), and that code can move to the Vulkan renderer when
     upstream has one.
   - Colour representation, which P22 does not serve.

   **How isolated it is.** Checked 2026-11-02: both pieces apply cleanly
   onto the `isle` branch at `3e5b6fe4`, with a three-way fallback. That
   is upstream from 2026-10-20, 156 commits past the fork's base.
   - **The protocol** is a new directory, `src/wayland/color/`, plus one
     line in `wayland/mod.rs`.
   - **The connector state** is a new file, `drm/color.rs`, plus additions
     to `drm/surface/atomic.rs`, `surface/mod.rs` and `compositor/mod.rs`.
     They carry the colour state beside VRR's, and remove six lines.
     `atomic.rs` is where a future upstream merge can conflict, and it
     already carries the fork's async-flip patch.

   Whether the ported code compiles was not checked: building outside code
   was refused in the planning session. 22.1 starts there.

   Recommended. The alternatives:
   - waiting for upstream, which has no date;
   - **a smaller patch modelled on VRR,** with the protocol written in
     Isle against `wayland-protocols`' bindings. That leaves less in the
     fork, but it means writing about 1,700 lines of protocol, version 3's
     scRGB and BT.2100 handling among them, which the port already has.
4. **Colour state changes only in a modeset commit,** never on a page flip
   and never in a commit of its own. A screen's HDR is switched on or off
   by relighting it, the way a mode change is.

### The screen

5. **HDR is a per-screen setting, off by default.** It is a row in Settings
   › Displays, shown only for a screen that qualifies:
   - its EDID lists ST2084;
   - its connector has `Colorspace` with BT2020_RGB and
     `HDR_OUTPUT_METADATA`.

   **No automatic switching** when a full-screen HDR client appears. Every
   switch is a modeset, and a modeset blanks the screen, so a game
   starting or a video going full screen would flash it. macOS and Windows
   both leave HDR to a switch. Recommended. The alternative is the niri
   fork's "auto".
6. **With HDR on:**
   - **The connector:** `Colorspace=BT2020_RGB`.
   - **The metadata:** `HDR_OUTPUT_METADATA` from the EDID: ST2084, the
     panel's chromaticities as the mastering display, and the desired
     maximum, maximum frame-average and minimum luminance (302, 302 and
     0 cd/m² here).
   - **The swapchain:** `Abgr2101010`/`Xbgr2101010`, the only 10-bit order
     NVIDIA scans out and GLES's `RGB10_A2`.
   - **Fallback:** if the test commit refuses 10-bit, HDR runs on an 8-bit
     swapchain, and Displays says so ("8-bit", choice 22).

   The link's bit depth is the driver's call, since there is no `max bpc`.
7. **Turning HDR on or off goes through B96's Keep alert,** so a screen
   left black reverts by itself when the countdown ends.
8. **HDR never outlives the session.**
   - Leaving the console, ending the session, or the compositor exiting
     resets every connector to SDR: default `Colorspace`, no metadata.
   - The greeter always runs SDR, and resets any HDR state it finds.
   - The greeter's look file does not carry HDR.
9. **SDR Brightness** is a slider under the toggle. It sets the reference
   white in cd/m², from 80 to the panel's peak, default 203 (BT.2408).
   Everything that is not HDR (windows, the shell, the wallpaper) is shown
   at this brightness. It is the one control that decides whether HDR
   mode feels dim. The spike found 203 dimmer than this monitor's SDR
   white, so 22.10 sets the default against the panel.

### Composition

10. **Blend in extended sRGB, in FP16, then encode PQ in a final pass.**
    With HDR on, a screen's frame is composited into an `Abgr16161616f`
    texture. Its values are:
    - BT.709 primaries;
    - a gamma 2.2 encoding, extended with its sign mirrored;
    - 1.0 meaning the reference white.

    SDR content is already in this space as it stands, so none of what
    draws it changes, and it blends exactly as today:
    - window textures;
    - the shell's Skia surfaces;
    - the wallpaper and its frost;
    - glass;
    - shadows and corner clips;
    - solid colours.

    A final pass then draws that texture into the 10-bit swapchain. For
    each pixel it decodes the 2.2 curve, scales by the reference white in
    nits, converts BT.709 to BT.2020, clamps, applies night light
    (choice 18), and encodes PQ.

    Recommended: it is Mutter's space, and it leaves the most code alone.
    The alternatives:
    - **KWin's space,** 10-bit gamma 2.2 at BT.2020 with 1.0 meaning the
      panel's peak. Every SDR texture, shader and colour would need
      converting, and SDR would keep about five-sixths of the code range.
    - **Linear FP16.** Translucency and anti-aliased edges would look
      different from today; KWin dropped it for that.
    - **Blending in PQ,** as the niri fork does. It is an approximation,
      and again every shader converts.
11. **The final pass keeps damage.** The intermediate texture has its own
    damage tracker, and the final pass redraws only the damaged
    rectangles. An idle HDR screen still draws no frames (P19p's bar).
12. **The shell stays 8-bit sRGB in Skia.** Its surfaces are SDR content at
    the reference white, like any app's. An HDR wallpaper and HDR
    thumbnails are not in P22.
13. **HDR surfaces are converted by Isle's own window element.** A surface
    whose image description is anything but sRGB with gamma 2.2 gets a
    conversion step in the window shader into the blend space:
    - decode its transfer function: PQ, extended linear, gamma 2.2,
      BT.1886, compound power 2.4;
    - convert its primaries to BT.709 in linear light, negatives kept;
    - scale it by its reference white, and encode it as extended gamma
      2.2.

    **Content brighter than the panel.** When content's mastering maximum
    is above the panel's peak, it is tone-mapped by BT.2390's EETF, on
    luminance so hue holds. Otherwise it is clamped at the peak. Extended
    linear (scRGB) is clamped, never tone-mapped, as the niri fork does.
14. **Glass over HDR content.** Glass and backdrop textures take the
    intermediate's format, so the pill blurs a bright window without
    clipping it. The contrast floor reads luminance clamped to SDR.
15. **The pointer on an HDR screen** is a copy of the cursor image encoded
    in PQ at the reference white. The cursor plane takes AR24 only and
    has no colour pipeline, so the values must already be PQ.
16. **Direct scan-out with HDR on.** The final pass ends scan-out with one
    exception. A full-screen surface is put straight on the primary plane,
    with no transform, when all of these hold:
    - it is tagged BT.2020 with PQ;
    - its buffer is XB30 or AB30;
    - it covers the screen;
    - nothing is over it but the pointer.

    Async flips and VRR then work as they do today.
    - **SDR and scRGB games on an HDR screen are composited,** so they do
      not tear (B88 tears only on scan-out).
    - **The monitor keeps the EDID's metadata,** not the game's, because a
      metadata change outside a modeset is the commit NVIDIA hangs on.
      Games read the panel's peak from the preferred description
      (choice 19) and aim at it.
17. **NVIDIA's plane colour pipeline is not used,** and Isle does not set
    the cap. The pipeline has no sRGB or gamma 2.2 curve, has stages that
    cannot be bypassed, and KWin ships it disabled. Moving the PQ encode
    to the CRTC (`NV_CRTC_REGAMMA_TF`) is not in P22 either.

### Night light

18. **On an HDR screen, night light is applied in the final pass.**
    - **What it applies:** the same blackbody gains, in linear light.
    - **The CRTC ramp:** left as identity and never written, since
      `gamma.rs` commits on its own.
    - **SDR screens** keep B91's ramp.
    - **Game Mode** still turns night light off.
    - **Captures** stay untinted, because they render apart from the
      final pass.

    B91 rejected a shader pass because it cost a pass and scan-out; on an
    HDR screen both are already paid. B91 is revised to say so.

### The protocol

19. **wp_color_manager_v1 version 3 is served on every screen, HDR or
    not,** so that an HDR video is tone-mapped correctly on an SDR screen
    too.
    - **Intent:** perceptual.
    - **Features:** `parametric`, `set_primaries`, `set_luminances`,
      `set_mastering_display_primaries`, `extended_target_volume`,
      `windows_scrgb`, `windows_bt2100`.
    - **Transfer functions:** `gamma22`, `st2084_pq`, `ext_linear`,
      `bt1886`, `compound_power_2_4`. Never the deprecated `srgb`.
    - **Primaries:** `srgb`, `bt2020`, `display_p3`.

    This is the set nested gamescope and Mesa's WSI check before they offer
    HDR and scRGB. Not served: ICC and colour representation.

    **Each screen's preferred description:**
    - **HDR on:** BT.2020 with PQ. The reference is SDR Brightness, and the
      maximum target is the EDID's peak.
    - **HDR off:** sRGB primaries with gamma 2.2, maximum equal to
      reference.

    `preferred_changed` is sent when a surface moves screens or a setting
    changes. gamescope offers HDR only when the maximum is above the
    reference, so SDR Brightness at the panel's peak hides HDR from games.
    Its footnote says so (choice 23).

### Capture

20. **Everything captured from an HDR screen is SDR.** This covers:
    - screenshots and image-copy;
    - PipeWire streams and the recorder;
    - the colour pick and its loupe;
    - window images, and the switcher's and Mission Control's previews.

    Each re-renders into the blend space as today (B80), then maps the
    result to 8-bit sRGB. Values above the reference white roll off with a
    soft knee into 1.0, on luminance; out-of-gamut values are clipped. A
    screenshot of the SDR desktop is the same as one taken with HDR off.
    HDR highlights are compressed, not clipped flat.

    Recommended. The portal carries no colour information, OBS reads no
    transfer function from a stream, WebRTC fails on HDR10 streams, and
    viewers wash out `cICP` PNGs. 16-bit PNGs with `cICP` and 10-bit PQ
    streams are not in P22.

### Gaming

21. **Isle sets no HDR variables for games.** HDR is each game's launch
    option, by either of two routes:
    - proton-cachyos with `PROTON_ENABLE_WAYLAND=1 PROTON_ENABLE_HDR=1`;
    - nested `gamescope --hdr-enabled`.

    The host run tries both, and `gaming.md` records the launch options
    that worked. X11 games stay SDR. Game Mode does not touch HDR.

### Settings and D-Bus

22. **The settings and the interface:**
    - **Settings:** `Output` gains `hdr` (a bool) and `sdr-white`
      (cd/m²).
    - **`Displays1`:**
      - `DisplayInfo` gains `hdr_capable`, `hdr`, `sdr_white`,
        `peak_nits` and `bit_depth`;
      - `Configure` takes `hdr` and `sdr_white`, and an HDR change returns
        the Keep alert;
      - SDR Brightness applies live, with no modeset.
    - **`isle perf`** reports which screens run the final pass and how
      many rectangles it redraws.
23. **No design gate.** P1's structure already puts HDR among each
    display's settings, and the rows are P13's toggle and slider. Two
    copy lines are for review, under the screen's VRR rows:
    - **High Dynamic Range** (toggle). Footnote: "Shows HDR games and
      video with more brightness and colour. The screen goes dark for a
      moment when this changes."
    - **SDR Brightness** (slider, shown while HDR is on). Footnote: "How
      bright everything that isn't HDR looks. Games see HDR only while
      this is below the screen's peak."

    With an 8-bit fallback, the toggle's footnote ends "Running at 8-bit."
    The rows join the pane's search index.

### Proof

24. **Headless tests in `test-hooks` builds:**
    - **A pretend HDR screen.** The headless backend gains an "HDR" output
      that runs the intermediate and the final pass into a 10-bit texture.
      A check on the readback:
      - SDR white encodes to PQ for 203 cd/m²;
      - an SDR window's pixels match today's within 1/1023 after decoding;
      - a PQ surface at 1000 cd/m² with mastering at 1000 lands at or
        below the peak;
      - scRGB at 2.0 lands at 160 cd/m².
    - **Damage.** With windows mapped and nothing committing, the HDR
      screen draws 0 frames over 2 s, and one committing client redraws
      only its damage.
    - **Capture.** A screenshot of the pretend HDR screen with SDR content
      equals the SDR screenshot within 1/255.
    - **The protocol.** A test client checks:
      - the advertised features, transfer functions and primaries;
      - its preferred description before and after HDR is switched on;
      - a parametric description built and set;
      - the protocol's errors for bad descriptions;
      - `preferred_changed` on a move.
    - **Night light.** On the pretend HDR screen the gains appear in the
      final pass. The ramp is never written.
    - **Settings.** `Displays1` round-trips `hdr` and `sdr_white`.
    - **Smithay.** The connector state's tests come with the port.

    **The VM can show no HDR,** since virtio-gpu has no `Colorspace`. It
    checks only that the toggle is hidden and that SDR is unchanged. HDR
    itself is proven on the host (22.10).

**Bars, on the host's release package:**

| | Bar |
|---|---|
| Idle, HDR on | 0 frames and 0 final passes over 60 s; P19p's idle CPU bar holds |
| One client at 144 Hz, HDR on | no missed frames; compositor CPU per frame within 10% of the same scene with HDR off |
| A full-screen HDR10 game | scanned out (`isle perf` shows the primary plane), with and without the pointer shown |
| Switching | HDR on, off and on again three times with no hang; DPMS off and on, and a console switch, come back in HDR |
| Look | the SDR desktop at SDR Brightness 203, side by side with HDR off: no visible shift in hue, near-black or translucency |

## The workspace

```
~/Development/smithay (isle branch)
  src/backend/drm/color.rs        connector colour state (ported)
  src/backend/drm/surface/        staged with the modeset
  src/wayland/color/management/   wp-color-management-v1 v3 (ported)
spikes/hdr-modeset/               22.0's throwaway
crates/
  isle-compositor/src/
    colour/                       the blend space, the PQ and 2.2
                                  curves, BT.2390, matrices, the final
                                  pass and conversion shaders (new)
    colour_management.rs          the protocol's handler, descriptions
                                  per surface, preferred per screen (new)
    backend/udev.rs               HDR state per screen, the 10-bit
                                  swapchain, reset on pause and exit,
                                  the scan-out exception
    backend/headless.rs           the pretend HDR screen
    backend/gamma.rs              held off HDR screens
    night_light.rs                gains for the final pass
    cursor.rs                     the PQ pointer
    capture.rs, cast.rs,
    image_copy.rs, freeze.rs      the SDR map
    displays.rs, dbus.rs          Displays1's fields
  isle-settings/                  `hdr`, `sdr-white`
  isle-ipc/                       DisplayInfo's fields
  isle-system-settings/src/pane/displays.rs   the two rows
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 22.0 · The spike and the record

- **The spike** (choice 2) on the host with the user, from a free
  console. It covers:
  - HDR on and off three times;
  - DPMS off and on while in HDR;
  - the FP16 render-target check;
  - a photo of the OSD.
- **The research note's §3.4** gains the spike's result.
- **The record:**
  - the roadmap's P22 entry;
  - `compositor.md`'s HDR line and its Not-served list;
  - ARCHITECTURE's forks section, whose upstream base is stale (it names
    `118e34f`; the branch is at `24647fc8`).

### 22.1 · Smithay: connector colour state

The port, onto the fork's `isle` branch, with its tests: staging, probing,
the foreign read-back and the reset. It starts by building the port as it
applies. It is a PR on `alexparlett/smithay`, and the workspace pins its
merge.

### 22.2 · Smithay: the colour-management protocol

The port of v3, with its tests. A PR as above.

### 22.3 · The screen's HDR state

- the EDID's HDR block through libdisplay-info, which is already in the
  tree;
- the settings keys;
- the relight with 10-bit and colour state, and the 8-bit fallback;
- the Keep alert;
- the reset on pause, exit and in the greeter;
- `Displays1`'s fields.

### 22.4 · The composite

- the FP16 intermediate and its damage;
- the final pass;
- glass and backdrops in its format;
- the PQ pointer;
- the pretend HDR screen and its tests.

### 22.5 · HDR clients

- the protocol wired, with each surface's description;
- the conversion step and BT.2390;
- the preferred description per screen, and `preferred_changed`.

### 22.6 · Scan-out of HDR10

The exception in choice 16, with the plane trace saying why a surface was
or was not scanned out.

### 22.7 · Night light

The final pass's gains, with the ramp held off on HDR screens.

### 22.8 · Captures

The SDR map for every path in choice 20, with the capture test.

### 22.9 · Settings › Displays

The two rows, the 8-bit line and the search entries.

### 22.10 · The host

On the installed package, with the user:
1. HDR on through Settings, with the OSD checked and the Keep alert
   tried, letting it revert once.
2. The SDR desktop at 203 beside HDR off: the look bar. SDR Brightness is
   moved and its choice noted.
3. `vulkaninfo` under Isle: NVIDIA's HDR colour spaces on Wayland.
4. A full-screen HDR10 game through proton-cachyos, scanned out, with the
   pointer hidden and then shown.
5. The same game in nested `gamescope --hdr-enabled`.
6. mpv with an HDR10 clip, windowed, on the HDR screen and then with HDR
   off.
7. DPMS, a console switch, and a sleep and wake, all in HDR.
8. Night light on, with HDR on.
9. A screenshot, a recording and a colour pick from the HDR screen.
10. The bars, with `isle perf --watch`.

### 22.11 · Close

- The decisions are recorded from the next free number. Among them, B23
  is closed, and B91 is revised for HDR screens.
- ARCHITECTURE gains How a frame reaches an HDR screen; `compositor.md`,
  `gaming.md`, the roadmap and `state.md` follow.
- This file ends with its outcome.

## Reviewed

The user reviewed the plan on 2026-11-02 and took every recommendation.
Choice 3 was questioned: would not taking the community fork change
anything? It changes where code lives, not the design. The alternative
was a VRR-shaped connector patch with the protocol written in Isle. The
user accepted the port's drift risk as long as the port stays isolated,
which the trial apply showed. Building began the same day.

## Not in P22

- NVIDIA's plane colour pipeline, and moving the PQ encode to the CRTC.
- HDR files and streams: 16-bit PNG with `cICP`, and 10-bit PQ PipeWire
  formats.
- ICC profiles, calibration, HLG, and colour representation (YUV
  coefficients and alpha modes).
- HDR switching on by itself for full-screen clients.
- HDR in the shell: an HDR wallpaper, Skia in FP16, HDR thumbnails.
- Sending a game's own metadata to the monitor.
- Outputs on the second GPU (P14c), and HDR in the VM.
- Moving to Smithay's Vulkan renderer when it lands. The colour maths is
  Isle's own so it can move then.

## Depends on, and feeds

- Needs:
  - P2 (the output loop, scan-out);
  - P11 (captures);
  - P12 (night light, VRR, tearing);
  - P13 (the Displays pane, B96's Keep);
  - P14b (the greeter's reset);
  - P19p (the bars and `isle perf`);
- Feeds `gaming.md`'s HDR launch options.

## Done when

- `cargo test` passes headless with choice 24's tests, on the fork's two
  merged PRs.
- In the VM, the HDR rows are hidden and SDR is unchanged.
- On the host, 22.10's run passes with the user, and the bars hold.

## Risks

- **The driver hangs or the screen stays black.** This is the KDE crash
  of B23 and the niri fork's hang. The spike (choice 2) comes first.
  Colour state rides only modesets (choice 4). The Keep alert reverts an
  unconfirmed change (choice 7), and the console keys still reach a TTY.
- **615's black-after-wake on DisplayPort.** Reports tie it to DSC at
  higher refresh rates; this mode should not need DSC. The spike's first
  run hit it once, in HDR, by `ACTIVE`; later runs did not. Isle sleeps
  a screen by disabling it, which came back each time. Phase 22.10's
  point 7 tests it. If it hits, the screen relights in SDR before DPMS
  off and in HDR after wake, at the cost of one more blank.
- **FP16 render targets on NVIDIA's GLES.** The spike checks
  `EXT_color_buffer_half_float`. If it is missing, choice 10 falls back
  to KWin's 10-bit space, which converts every SDR path. That is a much
  larger packet, and it comes back to the user.
- **The port drifts from upstream.** Accepted by the user (2026-11-02),
  if the port stays isolated. The protocol is a directory of its own.
  The connector state's additions to `atomic.rs` are where merges can
  conflict, and they sit beside VRR's, which upstream rarely moves. When
  upstream's own colour management lands, both PRs are dropped and the
  handler moves to it.
- **SDR looks dim or washed out in HDR.** This is the commonest complaint
  in every compositor. The 2.2 decode matches what SDR content was made
  for, SDR Brightness is the control, and the look bar is checked side by
  side.
- **Wine reads equal luminances as SDR.** A game may see no HDR when the
  reference equals the peak. SDR Brightness's footnote says so, and its
  default of 203 is under the panel's 302.
