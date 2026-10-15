use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn both_generators_reject_aggregate_local_definitions_before_output_or_panics() {
    for definition in [
        "struct Header {long seq;}",
        "enum Mode {idle,active}",
        "union Choice switch(long){case 0:long value;}",
    ] {
        for (source, owner) in [
            (
                format!("// 身份\nstruct Packet {{{definition} primary;}};"),
                "member",
            ),
            (
                format!("// 身份\nunion Packet switch(long){{case 0:{definition} primary;}};"),
                "element_spec",
            ),
        ] {
            let path = std::env::temp_dir().join(format!(
                "dust-aggregate-local-{}-{}.idl",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&path, source).unwrap();
            for compile in [dust_dds_gen::compile_idl, dust_dds_gen::compile_idl_c] {
                let error = std::panic::catch_unwind(|| compile(&path))
                    .expect("source-only definition must not panic")
                    .unwrap_err();
                assert!(error.contains("Aggregate-local definition"), "{error}");
                assert!(error.contains(owner), "{error}");
                assert!(error.contains("not code generation at 2:"), "{error}");
            }
            fs::remove_file(path).unwrap();
        }
    }
}
