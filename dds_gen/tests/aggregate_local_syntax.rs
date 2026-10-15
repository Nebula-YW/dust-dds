#![cfg(feature = "parser-api")]
use dust_dds_gen::syntax::{self, Rule};

#[test]
fn full_definitions_keep_direct_member_or_branch_owners_and_utf8_spans() {
    for definition in [
        "struct Header {long seq;}",
        "enum Mode {idle,active}",
        "union Choice switch(long){case 0:long value;}",
    ] {
        for (source, owner) in [
            (
                format!("// 身份\nmodule V {{struct Packet {{{definition} primary;}};}};"),
                Rule::member,
            ),
            (
                format!("// 身份\nunion Packet switch(long){{case 0:{definition} primary;}};"),
                Rule::element_spec,
            ),
        ] {
            let nodes: Vec<_> = syntax::parse(&source).unwrap().flatten().collect();
            let node = nodes
                .iter()
                .find(|n| {
                    n.as_rule() == owner && (*n).clone().into_inner().any(|c| c.as_str() == definition)
                })
                .unwrap();
            assert_eq!(
                node.clone()
                    .into_inner()
                    .filter(|n| n.as_str() == definition)
                    .count(),
                1
            );
            for node in nodes {
                let span = node.as_span();
                assert_eq!(&source[span.start()..span.end()], node.as_str());
            }
        }
    }
}

#[test]
fn nested_struct_positions_and_original_declarators_are_not_rewritten() {
    let source = "// 身份\nstruct Packet {struct Header {struct Item {long seq;} first;Item second;} primary[2],backup;};";
    let nodes: Vec<_> = syntax::parse(source).unwrap().flatten().collect();
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.as_rule() == Rule::struct_def)
            .count(),
        3
    );
    assert!(
        nodes
            .iter()
            .any(|n| n.as_rule() == Rule::array_declarator && n.as_str() == "primary[2]")
    );
    assert!(
        nodes
            .iter()
            .any(|n| n.as_rule() == Rule::declarators && n.as_str() == "primary[2],backup")
    );
}

#[test]
fn candidate_does_not_expand_template_discriminator_or_standalone_declaration_positions() {
    for source in [
        "struct Packet {struct Header {long seq;};};",
        "struct Packet {typedef long Counter;Counter value;};",
        "struct Packet {struct Header primary;};",
        "struct Packet {sequence<struct Header {long seq;},2> values;};",
        "union Packet switch(enum Mode {idle,active}){case idle:long value;};",
        "union Packet switch(long){case 0:struct Header {long seq;};};",
        "struct Packet {struct Header {long seq;} primary;",
    ] {
        assert!(syntax::parse(source).is_err(), "{source}");
    }
}
