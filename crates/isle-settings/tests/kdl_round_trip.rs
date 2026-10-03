use std::path::Path;

fn tokens() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/tokens/tokens.kdl");
    std::fs::read_to_string(path).unwrap()
}

fn check_spans(source: &str, doc: &kdl::KdlDocument, seen: &mut usize) {
    for node in doc.nodes() {
        let name = node.name().span();
        assert_eq!(
            &source[name.offset()..name.offset() + name.len()],
            node.name().value()
        );
        for entry in node.entries() {
            let span = entry.span();
            let text = &source[span.offset()..span.offset() + span.len()];
            let repr = entry.format().unwrap().value_repr.as_str();
            assert!(
                text.trim_start().ends_with(repr),
                "{text:?} does not end with {repr:?}"
            );
            *seen += 1;
        }
        if let Some(children) = node.children() {
            check_spans(source, children, seen);
        }
    }
}

#[test]
fn every_value_span_in_tokens_kdl_points_at_its_text() {
    let source = tokens();
    let doc: kdl::KdlDocument = source.parse().unwrap();
    let mut seen = 0;
    check_spans(&source, &doc, &mut seen);
    assert!(seen > 500, "only {seen} values checked");
}

#[test]
fn splicing_a_value_by_its_span_keeps_every_other_byte() {
    let source = "// output settings\noutput \"AOC U34G2G4R3\" {\n    scale 1.0 // native\n    vrr \"fullscreen\"\n}\n";
    let doc: kdl::KdlDocument = source.parse().unwrap();
    let scale = doc
        .get("output")
        .unwrap()
        .children()
        .unwrap()
        .get("scale")
        .unwrap();
    let span = scale.entries()[0].span();
    let end = span.offset() + span.len();
    let start = end - scale.entries()[0].format().unwrap().value_repr.len();
    let edited = format!("{}{}{}", &source[..start], "1.25", &source[end..]);
    assert_eq!(
        edited,
        "// output settings\noutput \"AOC U34G2G4R3\" {\n    scale 1.25 // native\n    vrr \"fullscreen\"\n}\n"
    );
    edited.parse::<kdl::KdlDocument>().unwrap();
}
