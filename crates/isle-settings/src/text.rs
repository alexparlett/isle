//! Edits to `settings.kdl` made in its own text, so every byte not changed stays as typed.
//! `kdl` only parses here: its printer loses whitespace (kdl-rs #178, #179, #193).

use kdl::{KdlDocument, KdlNode};

use crate::Seg;

const INDENT: &str = "    ";

/// Returns `source` with the leaf at `path` holding `args`, adding the nodes it lacks.
pub fn set_leaf(source: &str, doc: &KdlDocument, path: &[Seg], args: &str) -> String {
    let (at, remove, insert) = edit(source, doc, None, path, args);
    let mut out = String::with_capacity(source.len() + insert.len());
    out.push_str(&source[..at]);
    out.push_str(&insert);
    out.push_str(&source[at + remove..]);
    out
}

/// The edit as (offset, bytes removed, text inserted). `parent` is the node whose block
/// `doc` is, or `None` at the root.
fn edit(
    source: &str,
    doc: &KdlDocument,
    parent: Option<&KdlNode>,
    path: &[Seg],
    args: &str,
) -> (usize, usize, String) {
    let seg = &path[0];
    let Some(node) = doc.nodes().iter().find(|n| matches(n, seg)) else {
        let indent = match parent {
            Some(p) => format!("{}{INDENT}", line_indent(source, p.span().offset())),
            None => String::new(),
        };
        let text = nested(path, args, &indent);
        return insert_into(source, doc, parent, &text);
    };

    if path.len() == 1 {
        let entries = node.entries();
        let skip = usize::from(seg.key.is_some());
        let head = match skip {
            0 => node.name().span(),
            _ => entries[0].span(),
        };
        let head_end = trimmed_end(source, head.offset() + head.len());
        return match entries.get(skip..).and_then(|e| e.last()) {
            Some(last) => {
                let end = trimmed_end(source, last.span().offset() + last.span().len());
                (head_end, end - head_end, spaced(args))
            }
            None => (head_end, 0, spaced(args)),
        };
    }

    match node.children() {
        Some(children) => edit(source, children, Some(node), &path[1..], args),
        None => {
            let indent = line_indent(source, node.span().offset());
            let inner = format!("{indent}{INDENT}");
            let end = trimmed_end(source, node.span().offset() + node.span().len());
            let text = format!(
                " {{\n{inner}{}\n{indent}}}",
                nested(&path[1..], args, &inner)
            );
            (end, 0, text)
        }
    }
}

/// Returns `source` without the node at `path`, its line too when the node had it to
/// itself; `None` when the file does not hold that node.
pub fn remove_node(source: &str, doc: &KdlDocument, path: &[Seg]) -> Option<String> {
    let mut nodes = doc.nodes();
    let mut found = None;
    for seg in path {
        let node = nodes.iter().find(|n| matches(n, seg))?;
        nodes = node.children().map_or(&[], |c| c.nodes());
        found = Some(node);
    }
    let node = found?;
    let start = node.span().offset();
    let end = trimmed_end(source, start + node.span().len());
    let line_start = source[..start].rfind('\n').map_or(0, |n| n + 1);
    let alone_before = source[line_start..start].trim().is_empty();

    let rest = &source[end..];
    let mut after = end + (rest.len() - rest.trim_start_matches([' ', '\t']).len());
    let ended_by_semicolon = source[after..].starts_with(';');
    if ended_by_semicolon {
        after += 1;
        let rest = &source[after..];
        after += rest.len() - rest.trim_start_matches([' ', '\t']).len();
    }
    let line_end = source[after..].find('\n').map_or(source.len(), |n| after + n);
    let tail = source[after..line_end].trim();

    let (from, to) = if alone_before && (tail.is_empty() || tail.starts_with("//")) {
        (line_start, (line_end + 1).min(source.len()))
    } else if ended_by_semicolon {
        (start, after)
    } else {
        let before = source[..start].trim_end_matches([' ', '\t']);
        match before.strip_suffix(';') {
            Some(kept) => (kept.trim_end_matches([' ', '\t']).len(), end),
            None => (start, end),
        }
    };
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..from]);
    out.push_str(&source[to..]);
    Some(out)
}

/// Arguments after a node's name: a space first, or nothing for a bare node.
fn spaced(args: &str) -> String {
    if args.is_empty() {
        String::new()
    } else {
        format!(" {args}")
    }
}

fn matches(node: &KdlNode, seg: &Seg) -> bool {
    if node.name().value() != seg.name {
        return false;
    }
    match &seg.key {
        None => true,
        Some(key) => node
            .entries()
            .first()
            .is_some_and(|e| e.name().is_none() && e.value().as_string() == Some(key)),
    }
}

/// A new node for `path`, nested blocks indented under `indent`.
fn nested(path: &[Seg], args: &str, indent: &str) -> String {
    let seg = &path[0];
    let head = match &seg.key {
        Some(key) => format!("{} {}", seg.name, crate::quote(key)),
        None => seg.name.clone(),
    };
    if path.len() == 1 {
        return format!("{head}{}", spaced(args));
    }
    let inner = format!("{indent}{INDENT}");
    format!(
        "{head} {{\n{inner}{}\n{indent}}}",
        nested(&path[1..], args, &inner)
    )
}

/// Adds the node text `text` at the end of `doc`, the block of `parent`.
fn insert_into(
    source: &str,
    doc: &KdlDocument,
    parent: Option<&KdlNode>,
    text: &str,
) -> (usize, usize, String) {
    let Some(parent) = parent else {
        let sep = if source.is_empty() || source.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        return (source.len(), 0, format!("{sep}{text}\n"));
    };
    let close = parent.span().offset() + parent.span().len() - 1;
    debug_assert_eq!(&source[close..=close], "}");
    let open = source[..close].rfind('{').unwrap_or(close);
    let block = &source[open + 1..close];

    if block.contains('\n') {
        let line_start = source[..close].rfind('\n').map_or(0, |n| n + 1);
        let indent = line_indent(source, parent.span().offset());
        if source[line_start..close].trim().is_empty() {
            return (line_start, 0, format!("{indent}{INDENT}{text}\n"));
        }
        return (close, 0, format!("\n{indent}{INDENT}{text}\n{indent}"));
    }
    match doc.nodes().last() {
        Some(last) => {
            let end = trimmed_end(source, last.span().offset() + last.span().len());
            (end, 0, format!("; {}", text.trim_start()))
        }
        None if block.is_empty() => (close, 0, format!(" {} ", text.trim_start())),
        None => (close, 0, format!("{} ", text.trim_start())),
    }
}

fn line_indent(source: &str, offset: usize) -> &str {
    let start = source[..offset].rfind('\n').map_or(0, |n| n + 1);
    let line = &source[start..offset];
    &line[..line.len() - line.trim_start().len()]
}

fn trimmed_end(source: &str, end: usize) -> usize {
    source[..end].trim_end().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_path;

    fn set(source: &str, path: &str, args: &str) -> String {
        let doc: KdlDocument = source.parse().unwrap();
        let out = set_leaf(source, &doc, &parse_path(path).unwrap(), args);
        out.parse::<KdlDocument>()
            .unwrap_or_else(|e| panic!("{out:?} does not parse: {e}"));
        out
    }

    #[test]
    fn replacing_a_value_keeps_its_comment() {
        assert_eq!(
            set(
                "// mine\nkeyboard {\n    repeat-rate 25 // fast\n}\n",
                "keyboard.repeat-rate",
                "40"
            ),
            "// mine\nkeyboard {\n    repeat-rate 40 // fast\n}\n"
        );
    }

    fn remove(source: &str, path: &str) -> Option<String> {
        let doc: KdlDocument = source.parse().unwrap();
        let out = remove_node(source, &doc, &parse_path(path).unwrap())?;
        out.parse::<KdlDocument>()
            .unwrap_or_else(|e| panic!("{out:?} does not parse: {e}"));
        Some(out)
    }

    #[test]
    fn an_empty_list_is_a_bare_node() {
        assert_eq!(
            set("windows {\n    keep-running \"kitty\" \"steam\" // mine\n}\n", "windows.keep-running", ""),
            "windows {\n    keep-running // mine\n}\n"
        );
        assert_eq!(
            set("windows {\n}\n", "windows.keep-running", ""),
            "windows {\n    keep-running\n}\n"
        );
        assert_eq!(
            set("windows { keep-running; }", "windows.keep-running", "\"kitty\""),
            "windows { keep-running \"kitty\"; }"
        );
        assert_eq!(
            set("output \"A\" { mode \"3440x1440\" }", "output[A].mode", "\"1920x1080\""),
            "output \"A\" { mode \"1920x1080\" }"
        );
    }

    #[test]
    fn removing_a_node_takes_its_line_and_comment() {
        assert_eq!(
            remove("keyboard {\n    layouts \"us\" // mine\n    repeat-rate 25\n}\n", "keyboard.layouts"),
            Some("keyboard {\n    repeat-rate 25\n}\n".into())
        );
        assert_eq!(
            remove("// top\noutput \"A\" {\n    scale 1.5\n}\nkeyboard {}\n", "output[A]"),
            Some("// top\nkeyboard {}\n".into())
        );
        assert_eq!(remove("keyboard {}\n", "keyboard.layouts"), None);
        assert_eq!(remove("output \"A\" {}\n", "output[B]"), None);
    }

    #[test]
    fn removing_a_node_from_a_line_keeps_its_neighbours() {
        assert_eq!(
            remove("output \"A\" { scale 1.0; vrr \"always\" }\n", "output[A].scale"),
            Some("output \"A\" { vrr \"always\" }\n".into())
        );
        assert_eq!(
            remove("output \"A\" { scale 1.0; vrr \"always\" }\n", "output[A].vrr"),
            Some("output \"A\" { scale 1.0 }\n".into())
        );
        assert_eq!(
            remove("keyboard {\n    repeat-rate 25; numlock #true\n}\n", "keyboard.numlock"),
            Some("keyboard {\n    repeat-rate 25\n}\n".into())
        );
    }

    #[test]
    fn replacing_a_list_replaces_every_entry() {
        assert_eq!(
            set(
                "keyboard { layouts \"us\" \"gb\"; }",
                "keyboard.layouts",
                "\"fr\""
            ),
            "keyboard { layouts \"fr\"; }"
        );
    }

    #[test]
    fn a_keyed_node_keeps_its_key() {
        assert_eq!(
            set(
                "output \"AOC\" {\n    scale 1.0\n}\n",
                "output[AOC].scale",
                "1.25"
            ),
            "output \"AOC\" {\n    scale 1.25\n}\n"
        );
    }

    #[test]
    fn a_missing_leaf_joins_its_block_at_its_indent() {
        assert_eq!(
            set(
                "  keyboard {\n      layouts \"us\"\n  }\n",
                "keyboard.repeat-rate",
                "40"
            ),
            "  keyboard {\n      layouts \"us\"\n      repeat-rate 40\n  }\n"
        );
    }

    #[test]
    fn a_missing_leaf_joins_an_inline_block() {
        assert_eq!(
            set(
                "output \"A\" { scale 1.0 }\n",
                "output[A].vrr",
                "\"always\""
            ),
            "output \"A\" { scale 1.0; vrr \"always\" }\n"
        );
        assert_eq!(
            set("debug {}\n", "debug.watchdog-abort", "#false"),
            "debug { watchdog-abort #false }\n"
        );
    }

    #[test]
    fn a_missing_section_is_appended_at_the_end() {
        assert_eq!(
            set("// only a comment", "output[AOC U34].scale", "1.5"),
            "// only a comment\noutput \"AOC U34\" {\n    scale 1.5\n}\n"
        );
        assert_eq!(
            set("", "render-device", "\"/dev/dri/renderD129\""),
            "render-device \"/dev/dri/renderD129\"\n"
        );
    }

    #[test]
    fn a_node_without_a_block_gains_one() {
        assert_eq!(
            set("output \"A\"\n", "output[A].scale", "2.0"),
            "output \"A\" {\n    scale 2.0\n}\n"
        );
    }

    #[test]
    fn nested_missing_sections_are_built_whole() {
        assert_eq!(
            set("app \"x\" {\n}\n", "app[x].panel.width", "300"),
            "app \"x\" {\n    panel {\n        width 300\n    }\n}\n"
        );
    }
}
