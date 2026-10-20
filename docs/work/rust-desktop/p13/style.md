# Tile margins and tokens

A P13 follow-up from P14a's host run (2026-10-20), where the tiled windows'
margins read as gaps. Drawn on `style-1.html`, published as a private
Artifact; the user answers lgtm or names changes.

1. **The margin is a size.** Settings › Desktop's "Tiled windows have
   margins" switch becomes "Margins around tiled windows", a pop-up of
   None, Small, Medium and Large: 0, `space-s` (8), `space-m` (12, the
   default) and `space-l` (16). `windows.tile-margins` becomes
   `windows.tile-margin` with those four values; a file that still says
   `tile-margins #false` reads as None.
2. **Pictures under the group** show two tiles at each size, the chosen one
   ringed, as macOS shows its choices.
3. **Appearance › Advanced…** shows the tokens file's path and whether it
   loaded (none, how many tokens it changes, or the parse error the
   island's notice gives), with Show Shipped Tokens (a read-only copy
   opened in the editor) and Open in Editor. Settings never edits a token
   or writes the file (B10).
4. **Create in Editor…**, when there is no file, opens the default text
   editor at the path, so the file is the user's from its first save.
