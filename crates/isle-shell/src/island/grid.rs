//! Control Centre's grid: four columns, modules of one, two, two by two or four cells,
//! packed in reading order each into the first place it fits (`structure/control-centre.md`).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileSize {
    Small,
    Medium,
    Large,
    Wide,
}

impl TileSize {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "s" | "small" => Self::Small,
            "m" | "medium" => Self::Medium,
            "l" | "large" => Self::Large,
            "w" | "wide" => Self::Wide,
            _ => return None,
        })
    }

    /// Columns and rows it takes.
    pub fn cells(self) -> (usize, usize) {
        match self {
            Self::Small => (1, 1),
            Self::Medium => (2, 1),
            Self::Large => (2, 2),
            Self::Wide => (4, 1),
        }
    }
}

pub const COLUMNS: usize = 4;

/// Where each module goes, as its column and row, in the order given; and the rows used.
pub fn pack(sizes: &[TileSize]) -> (Vec<(usize, usize)>, usize) {
    let mut taken: Vec<[bool; COLUMNS]> = Vec::new();
    let mut places = Vec::with_capacity(sizes.len());
    for size in sizes {
        let (w, h) = size.cells();
        let mut row = 0;
        let place = loop {
            while taken.len() < row + h {
                taken.push([false; COLUMNS]);
            }
            let fits = |col: usize| (0..h).all(|dy| (0..w).all(|dx| !taken[row + dy][col + dx]));
            if let Some(col) = (0..=COLUMNS - w).find(|&c| fits(c)) {
                break (col, row);
            }
            row += 1;
        };
        for dy in 0..h {
            for dx in 0..w {
                taken[place.1 + dy][place.0 + dx] = true;
            }
        }
        places.push(place);
    }
    let rows = taken
        .iter()
        .rposition(|r| r.iter().any(|c| *c))
        .map_or(0, |i| i + 1);
    (places, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use TileSize::*;

    #[test]
    fn the_default_arrangement_packs_as_the_sheet_draws_it() {
        // Wi-Fi, Now Playing, Bluetooth, Focus, VPN, Sound, Display, Night Light, Energy
        // Mode, Updates, Batteries.
        let sizes = [
            Medium, Large, Medium, Medium, Medium, Wide, Wide, Small, Small, Medium, Medium,
        ];
        let (places, rows) = pack(&sizes);
        assert_eq!(
            places,
            [
                (0, 0),
                (2, 0),
                (0, 1),
                (0, 2),
                (2, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (1, 5),
                (2, 5),
                (0, 6)
            ]
        );
        assert_eq!(rows, 7);
    }

    #[test]
    fn an_absent_module_leaves_no_hole() {
        // With no Wi-Fi, Now Playing moves to the top left and the rest close up.
        let (places, rows) = pack(&[Large, Medium, Medium, Small]);
        assert_eq!(places, [(0, 0), (2, 0), (2, 1), (0, 2)]);
        assert_eq!(rows, 3);
    }
}
