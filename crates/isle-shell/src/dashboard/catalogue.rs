//! The built-in widgets: their names, sizes and whether more than one may be placed
//! (`p1/surfaces/widgets.md` The built-in widgets).

use super::grid::Sizes;

/// A setting a widget's form shows (`settings.md`'s control types).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setting {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: SettingKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingKind {
    Toggle(bool),
    /// Values and their labels, the first the default.
    Choice(&'static [(&'static str, &'static str)]),
    Number { min: i64, max: i64, default: i64 },
    /// Empty by default; what it means empty is the widget's.
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Builtin {
    pub kind: &'static str,
    pub name: &'static str,
    pub sizes: &'static [(i32, i32)],
    pub default: (i32, i32),
    pub several: bool,
    /// Its group in the gallery.
    pub group: &'static str,
    pub settings: &'static [Setting],
}

impl Builtin {
    pub fn sizes(&self) -> Sizes {
        Sizes::new(self.sizes, self.default)
    }
}

pub const BUILTINS: &[Builtin] = &[
    Builtin { kind: "clock", name: "Clock", sizes: &[(2, 1), (2, 2), (4, 2)], default: (2, 2), several: true, group: "Time and calendar", settings: CLOCK },
    Builtin { kind: "calendar", name: "Calendar", sizes: &[(2, 2), (4, 2), (4, 4)], default: (4, 2), several: true, group: "Time and calendar", settings: &[] },
    Builtin { kind: "timer", name: "Timer", sizes: &[(2, 1), (2, 2)], default: (2, 2), several: true, group: "Time and calendar", settings: &[] },
    Builtin { kind: "weather", name: "Weather", sizes: &[(2, 1), (2, 2), (4, 2), (4, 4)], default: (2, 2), several: true, group: "Weather", settings: &[] },
    Builtin { kind: "media", name: "Media", sizes: &[(2, 2), (4, 1), (4, 2)], default: (4, 1), several: false, group: "Media", settings: &[] },
    Builtin { kind: "system", name: "System", sizes: &[(2, 1), (2, 2), (4, 1), (4, 2), (6, 2)], default: (2, 2), several: true, group: "System", settings: &[] },
    Builtin { kind: "storage", name: "Storage", sizes: &[(4, 1), (4, 2)], default: (4, 1), several: false, group: "System", settings: &[] },
    Builtin { kind: "devices", name: "Devices", sizes: &[(2, 1), (2, 2), (4, 2)], default: (2, 1), several: false, group: "Devices", settings: &[] },
    Builtin { kind: "desktops", name: "Desktops", sizes: &[(4, 1), (4, 2), (6, 2)], default: (4, 1), several: false, group: "Desktop", settings: &[] },
    Builtin { kind: "controls", name: "Controls", sizes: &[(4, 2), (4, 3), (4, 4)], default: (4, 2), several: true, group: "Desktop", settings: &[] },
    Builtin { kind: "session", name: "Session", sizes: &[(4, 1), (4, 2)], default: (4, 2), several: false, group: "Desktop", settings: &[] },
    Builtin { kind: "notifications", name: "Notifications", sizes: &[(2, 1), (2, 2), (2, 4)], default: (2, 2), several: false, group: "Desktop", settings: &[] },
];

const CLOCK: &[Setting] = &[
    Setting { key: "zone", label: "Time zone", kind: SettingKind::Text },
    Setting {
        key: "face",
        label: "Face",
        kind: SettingKind::Choice(&[("digital", "Digital"), ("analogue", "Analogue")]),
    },
];

pub fn builtin(kind: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|b| b.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_one_of_its_sizes_and_kinds_are_unique() {
        for b in BUILTINS {
            assert!(b.sizes.contains(&b.default), "{}", b.kind);
            assert_eq!(BUILTINS.iter().filter(|o| o.kind == b.kind).count(), 1);
        }
    }

    #[test]
    fn the_default_layout_uses_sizes_its_widgets_allow() {
        use super::super::grid::{Extent, Metrics, default_layout};
        let e = Extent::of(
            3440.0,
            1440.0,
            Metrics { cell: 72.0, gap: 16.0, side: 32.0, top: 64.0, bottom: 86.0 },
        );
        for p in default_layout(&e).widgets {
            let b = builtin(p.kind()).unwrap();
            assert!(b.sizes().allows(p.at.w, p.at.h), "{}", p.id);
        }
    }
}
