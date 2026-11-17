# Store: the catalogues as Store reads them

P14e phase 14e.0, 2026-11-17, in the test guest (Arch, pacman 7.1,
`archlinux-appstream-data` 20261017, flatpak from `extra`) and against
the AUR's live RPC.

## 1. CachyOS's repositories: AppStream

- `archlinux-appstream-data` (21 MB installed) puts
  `/usr/share/swcatalog/xml/{core,extra,multilib}.xml.gz` and cached icons
  under `/usr/share/swcatalog/icons/archlinux-arch-{core,extra,multilib}/
  {48x48,64x64,128x128}/`. `extra` has 1,604 components, 1,390 of them
  `desktop-application`.
- A component carries `<pkgname>`, so a component maps to the package of
  that name; CachyOS's repositories rebuild Arch's packages under the same
  names.
- Every field has a translation per language (`xml:lang`); the untagged
  one is English. Krita's component is mostly translations.
- Icons: `cached` (4,095, by size, a file name) and `stock` (1,439, an
  icon theme name, present once the package is installed). The cached
  files are **JPEG XL** (`krita_krita.jxl`): Skia and `image` as Isle
  builds them decode neither, so Store needs a JPEG XL decoder
  (`jxl-oxide`, pure Rust) or falls back to the stock icon.
- Screenshots: `<image type="source">` URLs on the developers' own sites
  (`github.com/...`, `amsynth.github.io/...`), fetched over the network.
- `<categories>` from the desktop entry (`Graphics`, `2DGraphics`, …);
  `<releases>` with notes on 858 components; `<project_license>`,
  `<url type="homepage">`.

## 2. What an install brings and a removal takes

- `pacman -S --print --print-format '%n %v %s' krita`, as the user,
  lists the 13 packages it would install with versions and sizes.
- `pacman -Rns --print` is refused ("'--nosave' and '--print' may not be
  used together"); `pacman -Rs --print` gives the same list.
- A removal another package needs fails before any change:
  ":: removing libplacebo breaks dependency 'libplacebo' required by
  ffmpeg", one line per dependent, which names who to say.

## 3. Flathub

- `flatpak remote-add --user --if-not-exists flathub …` then
  `flatpak update --user --appstream` (3.6 s) fetches the catalogue to
  `~/.local/share/flatpak/appstream/flathub/x86_64/active/appstream.xml.gz`
  with PNG icons at 64 and 128. 4,843 components; each has a
  `<bundle type="flatpak">app/<id>/x86_64/stable</bundle>`.
- `flatpak remote-ls --user --updates` prints one line per update and
  nothing when none, exit 0. User installs need no root.

## 4. The AUR

- `https://aur.archlinux.org/rpc/v5/search/<term>?by=name` returns JSON:
  `Name`, `Version`, `Description`, `NumVotes`, `Popularity`,
  `OutOfDate` (a time or null), `LastModified`, `Maintainer`, `URL`,
  `PackageBase`. "obs" matched 293. No icons, screenshots or categories.

## 5. AppImages

- Isle already registers images in `~/Applications` as desktop entries
  (`isle-apps::appimage`). Update information lives in an image's
  `.upd_info` ELF section (zsync or GitHub releases); applying it needs
  `appimageupdatetool`, which is in the AUR only.

## For the plan

- Choice 5: Store decodes JPEG XL for catalogue icons, the installed
  stock icon first.
- Choice 10: the removal list is `pacman -Rs --print`; the run is
  `pacman -Rns`.
- Choice 8: AppImage updates need `appimageupdatetool` (AUR); without it
  an AppImage shows "Update from its developer" with its download page.
