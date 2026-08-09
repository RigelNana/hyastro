#[test]
fn frame_and_quantity_mismatches_do_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    #[cfg(not(feature = "anise"))]
    cases.compile_fail("tests/ui-default/*.rs");
    #[cfg(feature = "anise")]
    cases.compile_fail("tests/ui-anise/*.rs");
}
