//! Lutris's installed games from its database, opened read only, and their covers.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags};
use tracing::debug;

use crate::{Game, GameId, Launcher};

/// One row of Lutris's `games` table, as read.
pub struct Row {
    pub game: Game,
    pub slug: String,
    /// The Steam app id when Lutris imported the game from Steam.
    pub steam: Option<u32>,
}

/// Every installed game in Lutris's data folder `root`; `cache` is its cache folder, where
/// older versions kept covers.
pub fn read(root: &Path, cache: Option<&Path>, flatpak: bool) -> Vec<Row> {
    match query(root, cache, flatpak) {
        Ok(rows) => rows,
        Err(err) => {
            debug!(root = %root.display(), "Lutris's database not read: {err}");
            Vec::new()
        }
    }
}

fn query(root: &Path, cache: Option<&Path>, flatpak: bool) -> rusqlite::Result<Vec<Row>> {
    let db = Connection::open_with_flags(
        root.join("pga.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(Duration::from_millis(500))?;
    let mut statement = db.prepare(
        "SELECT id, name, slug, coalesce(runner, ''), coalesce(service, ''), coalesce(service_id, '') \
         FROM games WHERE installed = 1",
    )?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, name, slug, runner, service, service_id) = row?;
        let steam = (service == "steam" || runner == "steam")
            .then(|| service_id.parse().ok())
            .flatten();
        out.push(Row {
            game: Game {
                id: GameId::Lutris(id),
                name,
                cover: cover(root, cache, &slug),
                launcher: Launcher::Lutris,
                flatpak,
            },
            slug,
            steam,
        });
    }
    Ok(out)
}

fn cover(root: &Path, cache: Option<&Path>, slug: &str) -> Option<PathBuf> {
    [Some(root), cache]
        .into_iter()
        .flatten()
        .map(|dir| dir.join("coverart").join(format!("{slug}.jpg")))
        .find(|p| p.is_file())
}

pub fn watched(root: &Path, cache: Option<&Path>) -> Vec<PathBuf> {
    let mut out = vec![root.to_path_buf(), root.join("coverart")];
    out.extend(cache.map(|c| c.join("coverart")));
    out
}
