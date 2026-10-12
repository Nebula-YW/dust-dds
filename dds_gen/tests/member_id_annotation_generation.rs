use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

fn with_source(source: &str, test: impl FnOnce(&Path)) {
    let path = std::env::temp_dir().join(format!(
        "dust-member-id-{}-{}.idl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, source).unwrap();
    test(&path);
    fs::remove_file(path).unwrap();
}

#[test]
fn newly_accepted_syntax_is_rejected_before_rust_and_c_generation_can_lose_metadata_or_panic() {
    for (source, owner) in [
        (
            "@autoid(HASH) module V { struct Reading { long count; }; };",
            "module_dcl",
        ),
        (
            "union Choice switch(long) { case 0: @id(3) long reading; };",
            "element_spec",
        ),
        (
            "union Choice switch(@id(3) long) { case 0: long reading; };",
            "switch_type_spec",
        ),
        (
            "module Outer { @vendor(mode=7) module Inner { struct Reading { long count; }; }; };",
            "module_dcl",
        ),
    ] {
        with_source(source, |path| {
            for compile in [dust_dds_gen::compile_idl, dust_dds_gen::compile_idl_c] {
                let result = std::panic::catch_unwind(|| compile(path));
                let error = result
                    .expect("unsupported annotations must not panic")
                    .unwrap_err();
                assert!(error.contains(owner), "{error}");
                assert!(error.contains("not code generation"), "{error}");
                assert!(error.contains("1:"), "{error}");
            }
        });
    }
}

#[test]
fn existing_unannotated_module_and_union_generation_remains_available() {
    with_source(
        "module V { union Choice switch(long) {case 0: long reading;}; struct Reading {long count;}; };",
        |path| {
            assert!(dust_dds_gen::compile_idl(path).is_ok());
            // C union generation is an existing unsupported generator path, so use its supported module/struct profile.
        },
    );
    with_source("module V { struct Reading { long count; }; };", |path| {
        assert!(dust_dds_gen::compile_idl_c(path).is_ok());
    });
}
