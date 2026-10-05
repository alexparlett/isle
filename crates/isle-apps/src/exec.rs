//! A desktop entry's `Exec` line as a command, its field codes filled in (the Desktop Entry
//! Specification, "The Exec key").

/// The command for `exec`, opening `uris`, for the entry named `name` with `icon`, read from
/// `file`. `None` when the line does not parse.
pub fn command(
    exec: &str,
    uris: &[String],
    name: &str,
    icon: Option<&str>,
    file: &str,
) -> Option<Vec<String>> {
    let words = split(exec)?;
    let mut argv = Vec::with_capacity(words.len());
    for word in words {
        match word.as_str() {
            "%f" | "%u" => {
                if let Some(first) = uris.first() {
                    argv.push(local(first, word == "%f"));
                }
            }
            "%F" | "%U" => argv.extend(uris.iter().map(|u| local(u, word == "%F"))),
            "%i" => {
                if let Some(icon) = icon {
                    argv.push("--icon".into());
                    argv.push(icon.into());
                }
            }
            "%c" => argv.push(name.into()),
            "%k" => argv.push(file.into()),
            _ => {
                let mut out = String::new();
                let mut chars = word.chars().peekable();
                while let Some(c) = chars.next() {
                    if c != '%' {
                        out.push(c);
                        continue;
                    }
                    match chars.next() {
                        Some('%') => out.push('%'),
                        Some('c') => out.push_str(name),
                        Some('k') => out.push_str(file),
                        // Deprecated and unknown codes are dropped.
                        _ => {}
                    }
                }
                // A word that was only a dropped code goes with it.
                if !out.is_empty() || !word.starts_with('%') {
                    argv.push(out);
                }
            }
        }
    }
    (!argv.is_empty()).then_some(argv)
}

/// A `file://` URI as a path, for `%f` and `%F`; anything else as it is.
fn local(uri: &str, as_path: bool) -> String {
    match uri.strip_prefix("file://").filter(|_| as_path) {
        Some(path) => percent_decode(path),
        None => uri.to_owned(),
    }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The line's words: spaces separate them, double quotes group them with `\"`, `\``, `\$`
/// and `\\` escaped inside.
fn split(exec: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quoted = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                in_word = true;
            }
            '\\' if quoted => word.push(chars.next()?),
            ' ' | '\t' if !quoted => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            _ => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if in_word {
        words.push(word);
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(exec: &str, uris: &[&str]) -> Vec<String> {
        let uris: Vec<String> = uris.iter().map(|u| u.to_string()).collect();
        command(
            exec,
            &uris,
            "Files",
            Some("folder"),
            "/usr/share/applications/files.desktop",
        )
        .unwrap()
    }

    #[test]
    fn field_codes_are_filled_in() {
        assert_eq!(run("firefox %u", &[]), ["firefox"]);
        assert_eq!(
            run("firefox %u", &["https://a.b/"]),
            ["firefox", "https://a.b/"]
        );
        assert_eq!(
            run("eog %F", &["file:///tmp/a%20b.png", "file:///c.png"]),
            ["eog", "/tmp/a b.png", "/c.png"]
        );
        assert_eq!(
            run("app %i --name=%c %k", &[]),
            [
                "app",
                "--icon",
                "folder",
                "--name=Files",
                "/usr/share/applications/files.desktop"
            ]
        );
        assert_eq!(run("printf 100%% %d", &[]), ["printf", "100%"]);
    }

    #[test]
    fn quotes_group_words_with_escapes() {
        assert_eq!(
            run(r#""/opt/My App/run" --title "say \"hi\"""#, &[]),
            ["/opt/My App/run", "--title", "say \"hi\""]
        );
        assert!(command("\"unclosed", &[], "", None, "").is_none());
    }
}
