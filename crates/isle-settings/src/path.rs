//! Key paths: `keyboard.repeat-rate`, `output[AOC U34G2G4R3 0].scale`, `app[id].key`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub name: String,
    /// The argument naming one of several nodes with this name.
    pub key: Option<String>,
}

pub fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
    let mut segs = Vec::new();
    let mut rest = path;
    while !rest.is_empty() {
        let end = rest.find(['.', '[']).unwrap_or(rest.len());
        let name = &rest[..end];
        if name.is_empty() {
            return Err(format!("`{path}` is not a key"));
        }
        rest = &rest[end..];
        let key = if let Some(after) = rest.strip_prefix('[') {
            let close = after
                .find(']')
                .ok_or_else(|| format!("`{path}` has an unclosed `[`"))?;
            let key = after[..close].to_owned();
            rest = &after[close + 1..];
            Some(key)
        } else {
            None
        };
        segs.push(Seg {
            name: name.to_owned(),
            key,
        });
        if let Some(after) = rest.strip_prefix('.') {
            if after.is_empty() {
                return Err(format!("`{path}` ends with `.`"));
            }
            rest = after;
        } else if !rest.is_empty() {
            return Err(format!("`{path}` is not a key"));
        }
    }
    if segs.is_empty() {
        return Err("an empty key".into());
    }
    Ok(segs)
}

pub fn format_path(segs: &[Seg]) -> String {
    segs.iter()
        .map(|s| match &s.key {
            Some(k) => format!("{}[{k}]", s.name),
            None => s.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_round_trip_with_keys_that_hold_dots_and_spaces() {
        for p in [
            "keyboard.repeat-rate",
            "output[Dell Inc. U2720Q 7J2].scale",
            "app[io.github.alexparlett.Isle.Files].show-hidden",
            "debug",
        ] {
            assert_eq!(format_path(&parse_path(p).unwrap()), p);
        }
        let segs = parse_path("output[Dell Inc. U2720Q 7J2].scale").unwrap();
        assert_eq!(segs[0].key.as_deref(), Some("Dell Inc. U2720Q 7J2"));
        assert!(parse_path("a..b").is_err());
        assert!(parse_path("a[b").is_err());
        assert!(parse_path("a.").is_err());
    }
}
