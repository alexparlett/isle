//! Control Centre's grid: four columns, modules of one, two, two by two or four cells,
//! packed in reading order each into the first place it fits (`structure/control-centre.md`).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TileSize {
    Small,
    #[default]
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

/// A direction the arrows move in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Left,
    Right,
    Up,
    Down,
}

/// The module the arrows reach from `from`, by position: the nearest in that direction,
/// overlapping it across the way it moves first. Left and Right fall back to reading order.
pub fn neighbour(
    sizes: &[TileSize],
    places: &[(usize, usize)],
    from: usize,
    toward: Toward,
) -> Option<usize> {
    let span = |i: usize| {
        let (w, h) = sizes[i].cells();
        let (c, r) = places[i];
        (c, c + w, r, r + h)
    };
    let (c0, c1, r0, r1) = span(from);
    let overlap = |a0: usize, a1: usize, b0: usize, b1: usize| a0 < b1 && b0 < a1;
    let gap = |a0: usize, a1: usize, b0: usize, b1: usize| {
        if overlap(a0, a1, b0, b1) {
            0
        } else if b0 >= a1 {
            b0 - a1 + 1
        } else {
            a0 - b1 + 1
        }
    };
    let best = (0..sizes.len())
        .filter(|&i| i != from)
        .filter_map(|i| {
            let (d0, d1, s0, s1) = span(i);
            let (ahead, across) = match toward {
                Toward::Down if s0 >= r1 => (s0 - r1, gap(c0, c1, d0, d1)),
                Toward::Up if s1 <= r0 => (r0 - s1, gap(c0, c1, d0, d1)),
                Toward::Right if d0 >= c1 && overlap(r0, r1, s0, s1) => (d0 - c1, 0),
                Toward::Left if d1 <= c0 && overlap(r0, r1, s0, s1) => (c0 - d1, 0),
                _ => return None,
            };
            Some(((ahead, across, d0.abs_diff(c0)), i))
        })
        .min()
        .map(|(_, i)| i);
    best.or(match toward {
        Toward::Left => from.checked_sub(1),
        Toward::Right => (from + 1 < sizes.len()).then_some(from + 1),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use TileSize::*;

    #[test]
    fn the_arrows_move_by_place_in_the_grid() {
        // Wi-Fi, Now Playing (2 × 2), Bluetooth, Focus, VPN, Sound (wide).
        let sizes = [Medium, Large, Medium, Medium, Medium, Wide];
        let (places, _) = pack(&sizes);
        assert_eq!(neighbour(&sizes, &places, 0, Toward::Right), Some(1));
        assert_eq!(neighbour(&sizes, &places, 0, Toward::Down), Some(2));
        assert_eq!(neighbour(&sizes, &places, 2, Toward::Right), Some(1));
        assert_eq!(neighbour(&sizes, &places, 1, Toward::Down), Some(4));
        assert_eq!(neighbour(&sizes, &places, 4, Toward::Down), Some(5));
        assert_eq!(neighbour(&sizes, &places, 5, Toward::Up), Some(3));
        assert_eq!(neighbour(&sizes, &places, 0, Toward::Up), None);
        // At a row's end Right goes on in reading order.
        assert_eq!(neighbour(&sizes, &places, 1, Toward::Right), Some(2));
    }

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
