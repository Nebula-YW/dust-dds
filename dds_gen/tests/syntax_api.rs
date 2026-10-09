#![cfg(feature = "parser-api")]
use dust_dds_gen::syntax::{self, Rule};

#[test]
fn parses_array_aliases_without_running_codegen_and_preserves_utf8_spans() {
    let source = "// 类型\ntypedef float Values[4]; typedef Values History; @final struct State { History values; };";
    let nodes: Vec<_> = syntax::parse(source).unwrap().flatten().collect();
    let declaration = nodes
        .iter()
        .find(|p| p.as_rule() == Rule::typedef_dcl)
        .unwrap();
    assert_eq!(declaration.as_span().start_pos().line_col(), (2, 1));
    for node in nodes {
        let span = node.as_span();
        assert_eq!(
            &source.as_bytes()[span.start()..span.end()],
            node.as_str().as_bytes()
        );
    }
}

#[test]
fn retains_positional_and_named_annotation_arguments() {
    for (source, expected) in [
        ("@vendor(7) struct State { long value; };", "7"),
        (
            "@vendor(mode=7, enabled=TRUE) struct State { long value; };",
            "mode=7, enabled=TRUE",
        ),
    ] {
        let arguments = syntax::parse(source)
            .unwrap()
            .flatten()
            .find(|p| p.as_rule() == Rule::annotation_appl_params)
            .unwrap();
        assert_eq!(arguments.as_str(), expected);
    }
}

#[test]
fn rejects_prefix_only_parses_and_preprocessing() {
    for source in [
        "",
        "// empty",
        "struct State { long value; }; $",
        "#define COUNT 4\nstruct State { long value; };",
    ] {
        assert!(syntax::parse(source).is_err(), "{source}");
    }
}
