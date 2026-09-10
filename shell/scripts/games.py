#!/usr/bin/env python3
"""The game library for Big Picture, as JSON: Steam appmanifests, Heroic's store caches, Lutris' list."""
import glob, json, os, re, subprocess, shutil

HOME = os.path.expanduser("~")
games = []

# Steam: every installed app, with its library art when cached.
for root in [HOME + "/.local/share/Steam", HOME + "/.steam/steam", HOME + "/.var/app/com.valvesoftware.Steam/.local/share/Steam"]:
    if not os.path.isdir(root):
        continue
    libs = [root + "/steamapps"]
    vdf = root + "/steamapps/libraryfolders.vdf"
    if os.path.exists(vdf):
        libs += [p + "/steamapps" for p in re.findall(r'"path"\s+"([^"]+)"', open(vdf, errors="ignore").read())]
    seen = set()
    for lib in libs:
        for m in glob.glob(lib + "/appmanifest_*.acf"):
            txt = open(m, errors="ignore").read()
            appid = re.search(r'"appid"\s+"(\d+)"', txt)
            name = re.search(r'"name"\s+"([^"]+)"', txt)
            last = re.search(r'"LastPlayed"\s+"(\d+)"', txt)
            if not appid or not name or appid.group(1) in seen:
                continue
            seen.add(appid.group(1))
            aid = appid.group(1)
            if re.search(r"(Proton|Steam Linux Runtime|Steamworks Common)", name.group(1)):
                continue
            art = ""
            for cand in [root + f"/appcache/librarycache/{aid}/library_600x900.jpg", root + f"/appcache/librarycache/{aid}_library_600x900.jpg",
                         root + f"/appcache/librarycache/{aid}/header.jpg", root + f"/appcache/librarycache/{aid}_header.jpg"]:
                if os.path.exists(cand):
                    art = cand
                    break
            if not art:
                found = glob.glob(root + f"/appcache/librarycache/{aid}/*.jpg")
                art = found[0] if found else ""
            games.append({"source": "steam", "id": aid, "name": name.group(1), "art": art, "last": int(last.group(1)) if last else 0,
                          "cmd": f"steam steam://rungameid/{aid}"})
    break

# Heroic: Epic, GOG and Amazon libraries it has cached, installed ones only.
for cache in glob.glob(HOME + "/.config/heroic/store_cache/*_library.json") + glob.glob(HOME + "/.var/app/com.heroicgameslauncher.hgl/config/heroic/store_cache/*_library.json"):
    try:
        data = json.load(open(cache))
    except Exception:
        continue
    for g in data.get("library", []):
        if not g.get("is_installed"):
            continue
        # --no-gui: the game alone, no Heroic window in front of it.
        games.append({"source": "heroic", "id": g.get("app_name", ""), "name": g.get("title", ""), "art": g.get("art_cover") or g.get("art_square") or "",
                      "last": 0, "cmd": f"heroic --no-gui heroic://launch/{g.get('runner', 'legendary')}/{g.get('app_name', '')}"})

# Lutris: whatever it lists, with the cover or banner it cached for the slug.
if shutil.which("lutris"):
    try:
        out = subprocess.run(["lutris", "-l", "--json"], capture_output=True, text=True, timeout=5).stdout
        for g in json.loads(out or "[]"):
            slug = g.get("slug", "")
            art = ""
            for cand in [f"{HOME}/.local/share/lutris/coverart/{slug}.jpg", f"{HOME}/.cache/lutris/coverart/{slug}.jpg",
                         f"{HOME}/.local/share/lutris/banners/{slug}.jpg", f"{HOME}/.cache/lutris/banners/{slug}.jpg"]:
                if slug and os.path.exists(cand):
                    art = cand
                    break
            games.append({"source": "lutris", "id": str(g.get("id")), "name": g.get("name", ""), "art": art, "last": int(g.get("lastplayed") or 0),
                          "cmd": f"lutris lutris:rungameid/{g.get('id')}"})
    except Exception:
        pass

games.sort(key=lambda g: (-g["last"], g["name"].lower()))
print(json.dumps({"games": games, "steam": shutil.which("steam") is not None, "heroic": shutil.which("heroic") is not None,
                  "lutris": shutil.which("lutris") is not None, "gamescope": shutil.which("gamescope") is not None}))
