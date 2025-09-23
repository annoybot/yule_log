// tests/compile.rs
#[test]
fn compilation_tests() {
    let t = trybuild::TestCases::new();

    // Passing tests (should compile)
    t.pass("tests/compile/pass_ulogdata.rs");
    t.pass("tests/compile/pass_ulogmessages.rs");

    // Failing tests (should fail to compile)
    t.compile_fail("tests/compile/fail_ulogdata_enum.rs");
    t.compile_fail("tests/compile/fail_ulogdata_generic.rs");
    t.compile_fail("tests/compile/fail_ulogdata_unnamed.rs");
    t.compile_fail("tests/compile/fail_ulogmessages_forward_other.rs");
    t.compile_fail("tests/compile/fail_ulogmessages_generic.rs");
    t.compile_fail("tests/compile/fail_ulogmessages_multi.rs");
    t.compile_fail("tests/compile/fail_ulogmessages_struct.rs");
}
