#![cfg(feature = "parser-api")]
use dust_dds_gen::syntax::{self, Rule};

#[test]
fn module_union_member_and_discriminator_annotations_keep_direct_owners_and_utf8_spans() {
    let source = "// 身份\n@::autoid(value=HASH) module V {\n @autoid(SEQUENTIAL) module N {\n union Choice switch(@id(value=(1<<2)) long) {\n case 0: case 1: @hashid(\"co\" /* name */ \"lor\") @vendor(mode=7) long reading;\n default: @id(::V::Base+1) long other;\n };\n };\n};\n@autoid(HASH) module V { struct Tail { long count; }; };";
    let nodes: Vec<_> = syntax::parse(source).unwrap().flatten().collect();
    let owned: Vec<_> = nodes
        .iter()
        .filter(|n| {
            matches!(
                n.as_rule(),
                Rule::module_dcl | Rule::element_spec | Rule::switch_type_spec
            )
        })
        .flat_map(|n| {
            n.clone()
                .into_inner()
                .filter(|c| c.as_rule() == Rule::annotation_appl)
                .map(move |a| (n.as_rule(), a.as_str()))
        })
        .collect();
    assert_eq!(
        owned,
        [
            (Rule::module_dcl, "@::autoid(value=HASH)"),
            (Rule::module_dcl, "@autoid(SEQUENTIAL)"),
            (Rule::switch_type_spec, "@id(value=(1<<2))"),
            (Rule::element_spec, "@hashid(\"co\" /* name */ \"lor\")"),
            (Rule::element_spec, "@vendor(mode=7)"),
            (Rule::element_spec, "@id(::V::Base+1)"),
            (Rule::module_dcl, "@autoid(HASH)"),
        ]
    );
    for node in &nodes {
        let span = node.as_span();
        assert_eq!(&source[span.start()..span.end()], node.as_str());
    }
    let first = nodes
        .iter()
        .find(|n| n.as_rule() == Rule::annotation_appl)
        .unwrap();
    assert_eq!(first.as_span().start_pos().line_col(), (2, 1));
    let strings: Vec<_> = nodes
        .iter()
        .filter(|n| n.as_rule() == Rule::string_literal)
        .map(|n| n.as_str())
        .collect();
    assert_eq!(strings, ["\"co\"", "\"lor\""]);
}

#[test]
fn generic_annotations_and_unannotated_constructs_are_syntax_only_without_semantic_validation() {
    for source in [
        "module V { union Choice switch(long) { case 0: long reading; }; };",
        "@unknown(mode=TRUE) module V { struct Reading { long count; }; };",
        "union Choice switch(@vendor(7) unsigned long) {case 0: @::hashid(value=\"é\") long reading;};",
        "@autoid(HASH) @autoid(SEQUENTIAL) module V { struct Reading { long count; }; };",
        "union Choice switch(@id(-1) long) {case 0: @id(0xFFFFFFFF) @hashid long reading;};",
        "union Choice switch(@key long) {case 0: @optional(FALSE) long reading;};",
    ] {
        assert!(syntax::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn incomplete_annotation_arguments_invalid_placement_and_trailing_input_reject_the_entire_source() {
    for source in [
        "@autoid() module V { struct Reading { long count; }; };",
        "@autoid(value=) module V { struct Reading { long count; }; };",
        "union Choice switch(@id( long) { case 0: long reading; };",
        "union Choice switch(long) { case 0: @hashid(\"bad) long reading; };",
        "union Choice switch(long) { @id(3) case 0: long reading; };",
        "union Choice switch(long) { case @id(3) 0: long reading; };",
        "union Choice switch(long) { case 0: long reading @id(3); };",
        "@autoid(HASH) module V { struct Reading { long count; }; }; $",
    ] {
        assert!(syntax::parse(source).is_err(), "{source}");
    }
}
