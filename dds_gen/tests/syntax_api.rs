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

#[test]
fn wide_literals_precede_the_l_identifier_without_changing_literal_nodes() {
    for (source, rule, token) in [
        (
            r"const wchar Code=L'\u3BC';",
            Rule::wide_character_literal,
            r"L'\u3BC'",
        ),
        (
            r#"const wstring Text=L"value";"#,
            Rule::wide_string_literal,
            r#"L"value""#,
        ),
    ] {
        let node = syntax::parse(source)
            .unwrap()
            .flatten()
            .find(|p| p.as_rule() == rule)
            .unwrap();
        assert_eq!(node.as_str(), token);
    }
    for source in [
        "const long L=1; const long Copy=L;",
        "const long LCount=1; const long Copy=LCount;",
        "const long TRUEish=1; const long Copy=TRUEish;",
        "module L { const long Value=1; }; const long Copy=L::Value;",
    ] {
        assert!(syntax::parse(source).is_ok(), "{source}");
    }
    for source in [
        "const wchar Code=L'A' suffix;",
        "const wchar Code=L'';",
        "const wchar Code=L'AB';",
    ] {
        assert!(syntax::parse(source).is_err(), "{source}");
    }
}

#[test]
fn parses_double_extended_keyword_pair_with_whitespace_comments_and_original_spans() {
    for spelling in [
        "long double",
        "long\n double",
        "long /* precision */ double",
    ] {
        let source = format!("typedef {spelling} Extended; struct Reading {{ Extended value; }};");
        let floating = syntax::parse(&source)
            .unwrap()
            .flatten()
            .find(|p| p.as_rule() == Rule::floating_pt_type)
            .unwrap();
        assert_eq!(floating.as_str(), spelling);
        let span = floating.as_span();
        assert_eq!(&source[span.start()..span.end()], spelling);
    }
    for source in [
        "typedef longdouble Extended;",
        "typedef long doubleX Extended;",
    ] {
        // Scoped-name grammar may accept these; neither may become a floating keyword pair.
        if let Ok(nodes) = syntax::parse(source) {
            assert!(
                !nodes
                    .flatten()
                    .any(|p| p.as_rule() == Rule::floating_pt_type)
            );
        }
    }
}

#[test]
fn wide_literal_and_double_extended_rules_coexist_in_original_source_spans() {
    let source = r#"module L { const wchar Tag=L'\u3BC'; const wstring Label=L"val" /* split */ L"ue"; typedef long /* precision */ double Extended; struct Root { Extended reading; }; };"#;
    let nodes: Vec<_> = syntax::parse(source).unwrap().flatten().collect();
    for rule in [
        Rule::wide_character_literal,
        Rule::wide_string_literal,
        Rule::floating_pt_type,
    ] {
        let node = nodes.iter().find(|n| n.as_rule() == rule).unwrap();
        let span = node.as_span();
        assert_eq!(node.as_str(), &source[span.start()..span.end()]);
    }
}

#[test]
fn adjacent_string_expression_literals_keep_individual_tokens_and_utf8_spans() {
    for (source, rule, expected) in [
        (
            r#"const string Text="\xA" /* 注释 */ "B";"#,
            Rule::string_literal,
            vec![r#""\xA""#, r#""B""#],
        ),
        (
            r#"const wstring Text=L"A" L"\u3BC";"#,
            Rule::wide_string_literal,
            vec![r#"L"A""#, r#"L"\u3BC""#],
        ),
        (
            "const string Empty=\"\"\"\";",
            Rule::string_literal,
            vec!["\"\"", "\"\""],
        ),
    ] {
        let parsed = syntax::parse(source).unwrap();
        let literal = parsed
            .clone()
            .flatten()
            .find(|node| node.as_rule() == Rule::literal)
            .unwrap();
        let tokens: Vec<_> = literal.into_inner().collect();
        assert_eq!(tokens.len(), expected.len());
        for (token, spelling) in tokens.iter().zip(expected) {
            assert_eq!(token.as_rule(), rule);
            assert_eq!(token.as_str(), spelling);
            let span = token.as_span();
            assert_eq!(&source[span.start()..span.end()], spelling);
        }
    }
}

#[test]
fn adjacent_string_expressions_do_not_accept_other_kinds_or_change_identifier_resolution() {
    for source in [
        "const long L=1; const long Copy=L;",
        "const long TRUEish=1; const long Copy=TRUEish;",
        "const string Text=\"A\"; const string Copy=Text;",
        r#"const string Text=("A" "B");"#,
    ] {
        assert!(syntax::parse(source).is_ok(), "{source}");
    }
    for source in [
        r#"const string Text="A" L"B";"#,
        r#"const wstring Text=L"A" "B";"#,
        "const char Text='A' 'B';",
        "const long Count=1 2;",
        r#"const string Text="A" "B" suffix;"#,
        r#"const string Text="A" "unterminated;"#,
    ] {
        assert!(syntax::parse(source).is_err(), "{source}");
    }
}
