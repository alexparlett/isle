//! Games' names and covers from the launchers' own libraries, read and never written
//! (`gaming.md` Games as apps, Game library): Steam, Heroic and Lutris, native and Flatpak.

pub mod heroic;
pub mod lutris;
pub mod steam;
pub mod vdf;

use std::{collections::HashMap, fmt, path::PathBuf};

/// A game by its launcher's own id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GameId {
    Steam(u32),
    Heroic { runner: String, app: String },
    Lutris(i64),
}

impl fmt::Display for GameId {
    /// `steam:570`, `heroic:legendary:<app>`, `lutris:12`: the key favourites and use keep.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameId::Steam(id) => write!(f, "steam:{id}"),
            GameId::Heroic { runner, app } => write!(f, "heroic:{runner}:{app}"),
            GameId::Lutris(id) => write!(f, "lutris:{id}"),
        }
    }
}

impl GameId {
    pub fn parse(key: &str) -> Option<Self> {
        let (launcher, rest) = key.split_once(':')?;
        match launcher {
            "steam" => rest.parse().ok().map(GameId::Steam),
            "lutris" => rest.parse().ok().map(GameId::Lutris),
            "heroic" => {
                let (runner, app) = rest.split_once(':')?;
                Some(GameId::Heroic {
                    runner: runner.into(),
                    app: app.into(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Launcher {
    Steam,
    Heroic,
    Lutris,
}

impl Launcher {
    pub fn name(self) -> &'static str {
        match self {
            Launcher::Steam => "Steam",
            Launcher::Heroic => "Heroic",
            Launcher::Lutris => "Lutris",
        }
    }

    /// The launcher's desktop id, native or Flatpak.
    pub fn desktop_id(self, flatpak: bool) -> &'static str {
        match (self, flatpak) {
            (Launcher::Steam, false) => "steam",
            (Launcher::Steam, true) => "com.valvesoftware.Steam",
            (Launcher::Heroic, false) => "heroic",
            (Launcher::Heroic, true) => "com.heroicgameslauncher.hgl",
            (Launcher::Lutris, _) => "net.lutris.Lutris",
        }
    }
}

/// One installed game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub id: GameId,
    pub name: String,
    /// A portrait cover, 2:3 or near it.
    pub cover: Option<PathBuf>,
    pub launcher: Launcher,
    /// Read from the launcher's Flatpak install.
    pub flatpak: bool,
}

impl Game {
    /// The link its launcher starts it by (`gaming.md` Game library 2).
    pub fn url(&self) -> String {
        match &self.id {
            GameId::Steam(id) => format!("steam://rungameid/{id}"),
            GameId::Heroic { runner, app } => format!("heroic://launch/{runner}/{app}"),
            GameId::Lutris(id) => format!("lutris:rungameid/{id}"),
        }
    }

    /// The link to its page in its launcher, where there is one.
    pub fn page_url(&self) -> Option<String> {
        match &self.id {
            GameId::Steam(id) => Some(format!("steam://nav/games/details/{id}")),
            _ => None,
        }
    }
}

/// Where each launcher keeps its library; every one found is read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    /// Steam's roots, each with whether it is the Flatpak's.
    pub steam: Vec<(PathBuf, bool)>,
    pub heroic: Vec<(PathBuf, bool)>,
    /// Lutris's data folders, each with its cache folder.
    pub lutris: Vec<(PathBuf, Option<PathBuf>, bool)>,
}

impl Roots {
    /// The launchers' native and Flatpak folders that exist under `$HOME`.
    pub fn find() -> Self {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return Self::default();
        };
        let xdg = |var: &str, default: &str| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(default))
        };
        let (data, config, cache) = (
            xdg("XDG_DATA_HOME", ".local/share"),
            xdg("XDG_CONFIG_HOME", ".config"),
            xdg("XDG_CACHE_HOME", ".cache"),
        );
        let var = home.join(".var/app");

        let mut steam: Vec<(PathBuf, bool)> = Vec::new();
        let mut seen = Vec::new();
        for (root, flatpak) in [
            (data.join("Steam"), false),
            (home.join(".steam/root"), false),
            (home.join(".steam/steam"), false),
            (var.join("com.valvesoftware.Steam/.local/share/Steam"), true),
        ] {
            let real = root.canonicalize().unwrap_or_else(|_| root.clone());
            if root.join("steamapps/libraryfolders.vdf").is_file() && !seen.contains(&real) {
                seen.push(real);
                steam.push((root, flatpak));
            }
        }
        let heroic = [
            (config.join("heroic"), false),
            (var.join("com.heroicgameslauncher.hgl/config/heroic"), true),
        ]
        .into_iter()
        .filter(|(r, _)| r.is_dir())
        .collect();
        let lutris = [
            (data.join("lutris"), Some(cache.join("lutris")), false),
            (
                var.join("net.lutris.Lutris/data/lutris"),
                Some(var.join("net.lutris.Lutris/cache/lutris")),
                true,
            ),
        ]
        .into_iter()
        .filter(|(r, _, _)| r.join("pga.db").is_file())
        .collect();
        Self {
            steam,
            heroic,
            lutris,
        }
    }

    /// The folders whose change means the library should be read again.
    pub fn watched(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for (root, _) in &self.steam {
            out.extend(steam::watched(root));
        }
        for (root, _) in &self.heroic {
            out.extend(heroic::watched(root));
        }
        for (root, cache, _) in &self.lutris {
            out.extend(lutris::watched(root, cache.as_deref()));
        }
        out
    }
}

/// Every installed game across the launchers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    games: Vec<Game>,
    by_id: HashMap<GameId, usize>,
    /// Lutris games by slug, for `lutris:rungame/<slug>` in a desktop entry.
    lutris_slugs: HashMap<String, i64>,
}

impl Library {
    /// Reads every library under `roots`; blocking, so never on the main loop. A game in two
    /// roots is kept from the first. A Steam game Lutris imported is left to Steam where Steam
    /// is installed, since Lutris's record of it goes stale.
    pub fn read(roots: &Roots) -> Self {
        let mut library = Self::default();
        for (root, flatpak) in &roots.steam {
            for game in steam::read(root, *flatpak) {
                library.insert(game);
            }
        }
        for (root, flatpak) in &roots.heroic {
            for game in heroic::read(root, *flatpak) {
                library.insert(game);
            }
        }
        for (root, cache, flatpak) in &roots.lutris {
            for row in lutris::read(root, cache.as_deref(), *flatpak) {
                if row.steam.is_some() && !roots.steam.is_empty() {
                    continue;
                }
                if let GameId::Lutris(id) = row.game.id {
                    library.lutris_slugs.entry(row.slug).or_insert(id);
                }
                library.insert(row.game);
            }
        }
        library
    }

    fn insert(&mut self, game: Game) {
        if self.by_id.contains_key(&game.id) {
            return;
        }
        self.by_id.insert(game.id.clone(), self.games.len());
        self.games.push(game);
    }

    pub fn games(&self) -> &[Game] {
        &self.games
    }

    pub fn get(&self, id: &GameId) -> Option<&Game> {
        self.by_id.get(id).map(|i| &self.games[*i])
    }

    /// The game a window's app id names: `steam_app_<id>`.
    pub fn for_app(&self, app: &str) -> Option<&Game> {
        let id = app.strip_prefix("steam_app_")?.parse().ok()?;
        self.get(&GameId::Steam(id))
    }

    /// The library game a launcher's desktop entry starts, by the link in its `Exec`, so the
    /// entry folds into the game.
    pub fn for_exec(&self, exec: &str) -> Option<&Game> {
        let after = |marker: &str| {
            exec.split_once(marker).map(|(_, rest)| {
                rest.split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .next()
                    .unwrap_or("")
            })
        };
        if let Some(id) = after("steam://rungameid/").and_then(|id| id.parse().ok()) {
            return self.get(&GameId::Steam(id));
        }
        if let Some(id) = after("lutris:rungameid/").and_then(|id| id.parse().ok()) {
            return self.get(&GameId::Lutris(id));
        }
        if let Some(id) = after("lutris:rungame/").and_then(|slug| self.lutris_slugs.get(slug)) {
            return self.get(&GameId::Lutris(*id));
        }
        if let Some(rest) = after("heroic://launch/")
            && let Some((runner, app)) = rest.split_once('/')
        {
            return self.get(&GameId::Heroic {
                runner: runner.into(),
                app: app.into(),
            });
        }
        if let Some(query) = after("heroic://launch?") {
            let field = |name: &str| {
                query
                    .split('&')
                    .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
            };
            if let (Some(app), Some(runner)) = (field("appName"), field("runner")) {
                return self.get(&GameId::Heroic {
                    runner: runner.into(),
                    app: app.into(),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isle-games-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn copy(from: &std::path::Path, to: &std::path::Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap().flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    /// The Steam fixture with its second library folder pointed at the fixture's.
    fn steam_root(tmp: &std::path::Path) -> PathBuf {
        let root = tmp.join("steam");
        copy(&fixtures().join("steam"), &root);
        let folders = root.join("steamapps/libraryfolders.vdf");
        let text = fs::read_to_string(&folders)
            .unwrap()
            .replace("LIBRARY", &fixtures().join("library").to_string_lossy());
        fs::write(folders, text).unwrap();
        root
    }

    /// A Lutris database with a Wine game, a Steam import and an uninstalled game.
    fn lutris_root(tmp: &std::path::Path) -> PathBuf {
        let root = tmp.join("lutris");
        fs::create_dir_all(root.join("coverart")).unwrap();
        fs::write(root.join("coverart/witcher.jpg"), b"jpg").unwrap();
        let db = rusqlite::Connection::open(root.join("pga.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE games (id INTEGER PRIMARY KEY, name TEXT, slug TEXT, runner TEXT, installed INTEGER, service TEXT, service_id TEXT);
             INSERT INTO games VALUES (3, 'The Witcher', 'witcher', 'wine', 1, NULL, NULL);
             INSERT INTO games VALUES (12, 'Tainted Grail', 'tainted-grail', 'steam', 1, 'steam', '1466060');
             INSERT INTO games VALUES (13, 'Only In Lutris', 'only', 'steam', 1, 'steam', '999');
             INSERT INTO games VALUES (4, 'Anno 1800', 'anno-1800', NULL, 0, 'ubisoft', 'x');",
        )
        .unwrap();
        root
    }

    #[test]
    fn steam_reads_every_folder_and_cover_layout() {
        let tmp = temp("steam");
        let root = steam_root(&tmp);
        let games = steam::read(&root, false);
        let library = Library::read(&Roots {
            steam: vec![(root, false)],
            ..Default::default()
        });
        assert_eq!(games.len(), 2);
        let tainted = library.for_app("steam_app_1466060").unwrap();
        assert_eq!(tainted.name, "Tainted Grail: The Fall of Avalon");
        assert!(
            tainted
                .cover
                .as_ref()
                .unwrap()
                .ends_with("hash/library_600x900.jpg")
        );
        let stellaris = library.for_app("steam_app_281990").unwrap();
        assert_eq!(
            stellaris.name, "Stellaris",
            "from the second library folder"
        );
        assert!(
            stellaris
                .cover
                .as_ref()
                .unwrap()
                .ends_with("grid/281990p.png"),
            "the user's own cover wins"
        );
        assert_eq!(stellaris.url(), "steam://rungameid/281990");
        assert_eq!(
            stellaris.page_url().as_deref(),
            Some("steam://nav/games/details/281990")
        );
        assert!(library.for_app("steam_app_1").is_none());
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn heroic_reads_installed_games_and_its_cached_covers() {
        let games = heroic::read(&fixtures().join("heroic"), true);
        let names: Vec<&str> = games.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(
            names,
            ["Shadow Game", "Installed By Record", "Sideloaded Thing"]
        );
        let shadow = &games[0];
        assert_eq!(shadow.id.to_string(), "heroic:legendary:Pheasant");
        assert!(
            shadow
                .cover
                .as_ref()
                .unwrap()
                .starts_with(fixtures().join("heroic/images-cache"))
        );
        assert!(
            games[1]
                .cover
                .as_ref()
                .unwrap()
                .ends_with("icons/ByRecord.jpg")
        );
        assert_eq!(games[2].cover, None);
        assert_eq!(shadow.url(), "heroic://launch/legendary/Pheasant");
        assert_eq!(
            shadow.launcher.desktop_id(shadow.flatpak),
            "com.heroicgameslauncher.hgl"
        );
    }

    #[test]
    fn lutris_reads_installed_games_and_leaves_steams_to_steam() {
        let tmp = temp("lutris");
        let library = Library::read(&Roots {
            steam: vec![(steam_root(&tmp), false)],
            lutris: vec![(lutris_root(&tmp), None, false)],
            ..Default::default()
        });
        let witcher = library.get(&GameId::Lutris(3)).unwrap();
        assert_eq!(witcher.name, "The Witcher");
        assert!(
            witcher
                .cover
                .as_ref()
                .unwrap()
                .ends_with("coverart/witcher.jpg")
        );
        assert_eq!(witcher.url(), "lutris:rungameid/3");
        assert!(library.get(&GameId::Lutris(12)).is_none(), "Steam has it");
        assert!(
            library.get(&GameId::Lutris(13)).is_none(),
            "Steam decides, and has not got it"
        );
        assert!(library.get(&GameId::Lutris(4)).is_none(), "not installed");
        let without_steam = Library::read(&Roots {
            lutris: vec![(tmp.join("lutris"), None, false)],
            ..Default::default()
        });
        assert!(
            without_steam.get(&GameId::Lutris(13)).is_some(),
            "no Steam to start it"
        );
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn launchers_desktop_entries_fold_into_their_games() {
        let tmp = temp("fold");
        let library = Library::read(&Roots {
            steam: vec![(steam_root(&tmp), false)],
            heroic: vec![(fixtures().join("heroic"), false)],
            lutris: vec![(lutris_root(&tmp), None, false)],
        });
        let id = |exec: &str| library.for_exec(exec).map(|g| g.id.to_string());
        assert_eq!(
            id("steam steam://rungameid/281990").as_deref(),
            Some("steam:281990")
        );
        assert_eq!(
            id("env LUTRIS_SKIP_INIT=1 lutris lutris:rungameid/3").as_deref(),
            Some("lutris:3")
        );
        assert_eq!(
            id("lutris lutris:rungame/witcher").as_deref(),
            Some("lutris:3")
        );
        assert_eq!(
            id("xdg-open heroic://launch/legendary/Pheasant").as_deref(),
            Some("heroic:legendary:Pheasant")
        );
        assert_eq!(
            id("xdg-open \"heroic://launch?appName=Pheasant&runner=legendary\"").as_deref(),
            Some("heroic:legendary:Pheasant")
        );
        assert_eq!(id("steam steam://rungameid/1"), None);
        assert_eq!(id("firefox %u"), None);
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn ids_round_trip() {
        for key in ["steam:570", "heroic:gog:1207658924", "lutris:12"] {
            assert_eq!(GameId::parse(key).unwrap().to_string(), key);
        }
        assert!(GameId::parse("itch:1").is_none());
    }
}
