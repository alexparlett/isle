# Wallpaper

P1 stage 5, wave A. How the wallpaper looks and changes, in Frost, from
`structure/wallpaper.md` (binding). Values are `tokens.kdl` names; numbers
not yet tokens are listed at the end.

## Anatomy

The wallpaper is the bottom layer of each screen, drawn by the compositor.
It carries no text and takes no focus. Per screen, keyed by the monitor's
identity; every desktop on a screen shares it.

**Image**, decoded and scaled on the service worker to the screen's size in
device pixels, once per change:

| Fit | Drawing |
|---|---|
| Fill screen (default) | Scaled to cover, cropped about the centre. A 16:9 picture on the ultrawide loses its top and bottom; a 21:9 one on the 1920 screen loses its sides |
| Fit to screen | Scaled to fit whole, centred, on the colour |
| Centre | One image pixel to one device pixel, centred on the colour; cropped about the centre if larger |

**Colour**: the fill behind Fit and Centre, and the whole screen when a
colour is chosen. Its default for a fitted image is the image's mean
colour, so the bars read as part of the picture; the user can pick
another in Settings.

**Mean colour**: the average of the scaled copy, in linear light, kept in
state per screen with the choice. It is the first frame at session start
and the fill while a new image decodes.

**Drawn backdrop** (Isle's default and the fallback), Frost's neutral
greys and nothing else, never the accent (brief P6), drawn by a shader with
no image to decode, so it can be the session's first frame:

| Layer | Dark | Light |
|---|---|---|
| Base | `colour-window-sunken` | `colour-window-sunken` |
| Overhead light: an ellipse centred at 50% of the width and 10% above the top edge, radii 70% of the width and 95% of the height, eased to transparent | `colour-window-raised` | `colour-view` |
| Falloff: a radial from the centre, transparent to two thirds out, to its colour at the corners | `colour-scrim` | `colour-fill-strong` |
| Grain over all, against banding on a 3440-wide gradient | `material-noise-opacity` | `material-noise-opacity` |

It reads as a soft light from behind the island over a quiet grey floor:
top centre about #28292e, corners about #0f1012 in dark; #ffffff to about
#d9d9db in light. Proportional to the screen, so the light stays broad on
the ultrawide and the 1920 screen alike. The glass over it takes Frost's
neutral grey and nothing else.

**Softened form**: the same picture, made once per change per screen and
appearance, for the lock and the widgets dashboard. The scaled copy,
blurred at `material-softened-blur`, saturated at
`material-softened-saturation`, then washed with `material-softened`
(`rgba(16,17,20,0.45)` in dark, `rgba(246,246,248,0.50)` in light). Shapes
do not read in it; colour does. Light washes toward the glass's own
light base rather than darkening, so the lock's dark text has a light
ground in light appearance (the sheet says darkened; for the gate).

**Menu** (secondary click): the menu component on glass, as menus over the
desktop are, at the pointer: "Change wallpaper…", "Edit widgets…".

**Bundled set.** Isle ships its drawn backdrop as the default and five
pictures, each with a dark and a light variant, all dark and quiet in their
dark form (brief, interview). Each meets these rules:

- Mastered at 6880 × 2880 (the ultrawide at 2×), with what matters inside
  the centre 16:9, so Fill on the 1920 screen keeps it.
- Dark variants: mean luminance under 10%, nothing above 40%, no hard
  edges; low detail across the top centre, where the island and the
  on-demand bar sit, and the centre, where the switcher and Spotlight open.
- No subject, text, logo or figure: glass carries colour from it, never
  shapes (brief P4).
- Free to ship: rendered for Isle with sources in the repo under its
  licence, or public domain with the source page recorded.

| Name | Dark | Light | Source |
|---|---|---|---|
| Frost | The drawn backdrop above | The drawn backdrop, light | Shader, from tokens |
| Ridge | Layers of mountain ridges receding into haze at night, flat-shaded like cut paper, slate blue to near black | The same at dawn in pale blue-grey mist | Rendered for Isle (Blender, flat shading) |
| Tide | A still sea under an overcast blue-hour sky, the horizon on the lower third, almost no texture on the water | The same under a silver noon overcast | Rendered for Isle (procedural water and sky) |
| Pine | A conifer slope in fog, trees fading to dark green-grey toward the top | The same in snow fog, near white | Rendered for Isle (Blender, volumetric fog) |
| Dune | Sand ridges lit low from one side, warm grey shadows filling most of the frame | The same at midday, pale and flat | Rendered for Isle (procedural terrain) |
| Night side | The Earth's limb at night from orbit, a thin airglow band and scattered city lights low in the frame | The day-side limb, thin blue atmosphere over cloud | NASA ISS crew photographs, public domain; each frame's page checked for third-party credit |

The harness's Slate, Pine and Dusk stand in for Frost, Pine and Ridge
until the pictures exist.

## States

| State | What shows |
|---|---|
| Rest | The image per its fit, the colour, or the drawn backdrop; still. |
| Empty | The drawn backdrop in the current appearance. |
| Loading, session start | First frame: the cached mean colour, or the drawn backdrop with none cached; the image crossfades in once decoded. |
| Loading, a change | The old picture stays until the new one is decoded, then crossfades. |
| Error | The last good picture stays, else the drawn backdrop; nothing in the shell changes. |
| Folder cycling | Each due change is a crossfade; one due while covered or locked waits. |
| Appearance change | The other variant crossfades in, starting with the appearance change. |
| Second screen | Its own picture and fit, or the shared one cropped to 16:9. |
| Full-screen window or game | Not drawn; no render element. A change is applied without a transition. |
| Game Mode (windowed game) | Shown, still; a change is a cut. |
| Locked | Unseen; the lock draws the softened form. |
| Show Desktop | Plain, with nothing over it but the island. |
| Mission Control | Plain, under `colour-veil`, behind the spread (`mission-control.md`). |
| Idle dim | Dimmed with everything (`lock.md`). |
| Focus on | No change. |
| Reduce Motion | The crossfade stays at full length (Choices 2). |
| Reduce Transparency | No change; the materials over it turn solid. |
| Increase Contrast | No change. |
| Largest text size | Not applicable. |

## Motion

| Transition | Form |
|---|---|
| Change | A crossfade from old to new over `motion-wallpaper-fade-ms` (600), on that screen only, on the same curve as Isle's other fades. Nothing scales, pans or zooms. |
| Session start | Mean colour (or backdrop) to image over the same token. |
| Appearance change | The variant crossfades over the same token, beginning with the appearance's own change. |
| Softened form's arrival | The lock's and the dashboard's (`lock.md`, `widgets.md`); this surface hands over the picture, made ahead. |
| Reduce Motion | Unchanged: a crossfade moves nothing, and a shorter one is a flash. |
| Game Mode, covered | A cut on the next frame it is drawn. |

Exits do not apply: the wallpaper is never opened or closed.

## Accessibility

- **No text on it.** Contrast is promised by the materials over it
  (DESIGN.md, Contrast), never against the wallpaper. The softened form
  likewise promises no contrast: surfaces over it put their text on their
  own material, which `lock.md` and `widgets.md` decide in wave B and C.
- **Increase Contrast and Reduce Transparency** change nothing here; the
  surfaces over it change.
- **Focus.** Never focused; no ring. The menu follows the menu component's
  rules.
- **Largest text size.** Not applicable; the menu's items scale as menus do.
- **Hit targets.** The whole screen; the menu's rows are menu rows.
- **Dark variants** stay under 10% mean luminance, so the island's dark
  pill and the dark glass never sit on a bright field by default.

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it; whole screens are
scaled, and each scene has a crop of 1600 × 670 at the top centre with
the island over it, so grain, banding and the island's edge read at
about actual size.

1. **Frost backdrop**, dark and light, both screens.
2. **Each bundled picture**, dark and light, as described (placeholder
   gradients until rendered), with the island at rest.
3. **Fit modes**: one portrait picture on the ultrawide in Fill, Fit (mean
   colour bars) and Centre; crop at the image's edge.
4. **One picture on both screens** in Fill, showing the two crops.
5. **Crossfade**: three frames (0, 50, 100%) of a change, token named.
6. **Session start**: the mean-colour first frame beside the image.
7. **Softened form**, dark and light, for Frost and two pictures; a glass
   card over each so the wash can be judged with the material on it.
8. **Over the softened form**: a stand-in lock clock and the dashboard's
   grid outline, to judge the wash (not their design).
9. **Secondary click**: the menu over the wallpaper, crop 800 × 400.
10. **Glass over each wallpaper**: Control Centre's panel over Frost,
    Ridge and Night side in both appearances, to check that colour comes
    through and shapes do not.

## Choices

1. **The bundled set.** (a) The drawn backdrop plus five pictures rendered
   for Isle or public domain, each with both variants, as above; (b) the
   drawn backdrop alone, with folders of the user's own. **Recommended:
   (a).** The brief asks for dark and quiet pictures, a first boot should
   show something chosen, and the lock needs one before a session exists.
   Rendering them keeps the licence the repo's and both variants exact.
2. **Reduce Motion.** (a) The crossfade at full length; (b) a shorter one,
   `motion-reduced-fade-ms`. **Recommended: (a)**, as above.
3. **Crossfade length.** (a) 600; (b) `motion-fade-ms` (160).
   **Recommended: (a).** A full-screen picture changing in 160 is a flash;
   the change is seldom and nothing waits on it.

## New tokens

| Group | Name | Value |
|---|---|---|
| `motion` | `wallpaper-fade-ms` | 600 |
| `material` | `softened` (per appearance) | dark `rgba(16,17,20,0.45)`; light `rgba(246,246,248,0.50)` |
| `material` | `softened-blur` | 80 |
| `material` | `softened-saturation` | 1.4 |
