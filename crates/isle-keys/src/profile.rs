//! Keyboard profiles: Mac or PC, guessed from a keyboard's name.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Profile {
    Mac,
    Pc,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::Mac => "mac",
            Profile::Pc => "pc",
        }
    }

    /// Mac for the names Apple's and Keychron's keyboards report (v1's list), else PC.
    pub fn guess(name: &str) -> Profile {
        let name = name.to_ascii_lowercase();
        if ["apple", "magic keyboard", "keychron"]
            .iter()
            .any(|n| name.contains(n))
        {
            Profile::Mac
        } else {
            Profile::Pc
        }
    }
}

/// Whether a keyboard has the ISO key left of Z, guessed from its first layout: ANSI for
/// US English, ISO for the rest.
pub fn guess_iso(first_layout: &str) -> bool {
    first_layout != "us"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_guess_profiles() {
        assert_eq!(
            Profile::guess("Keychron  Keychron Link  Keyboard"),
            Profile::Mac
        );
        assert_eq!(Profile::guess("Apple Inc. Magic Keyboard"), Profile::Mac);
        assert_eq!(Profile::guess("AT Translated Set 2 keyboard"), Profile::Pc);
        assert_eq!(
            Profile::guess("Valve Software Steam Controller Puck Keyboard"),
            Profile::Pc
        );
    }
}
