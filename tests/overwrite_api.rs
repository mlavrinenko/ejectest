//! The overwrite rule through the library API.

use std::fs;

use ejectest::{ModRsTests, apply_path, check_path};
use tempfile::TempDir;

const SOURCE: &str =
    "pub fn foo() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn it() {}\n}\n";

fn setup(src: &str, test: &str, existing: Option<&str>) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let src_path = dir.path().join(src);
    fs::write(&src_path, SOURCE).expect("write");
    if let Some(content) = existing {
        fs::write(dir.path().join(test), content).expect("write");
    }
    (dir, src_path)
}

#[test]
fn apply_path_refuses_a_different_target_in_both_modes() {
    for (src, test, mode) in [
        ("foo.rs", "foo_tests.rs", ModRsTests::Sibling),
        ("mod.rs", "mod_tests.rs", ModRsTests::Sibling),
        ("mod.rs", "tests.rs", ModRsTests::Tests),
    ] {
        let (dir, src_path) = setup(src, test, Some("// mine\n"));
        let err = apply_path(&src_path, false, None, mode, false).expect_err("refused");
        assert!(
            err.to_string()
                .contains("exists with other content; pass --overwrite")
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(test)).expect("read"),
            "// mine\n"
        );
        assert_eq!(fs::read_to_string(&src_path).expect("read"), SOURCE);
    }
}

#[test]
fn apply_path_overwrite_replaces() {
    let (dir, src_path) = setup("mod.rs", "tests.rs", Some("// mine\n"));
    apply_path(&src_path, false, None, ModRsTests::Tests, true).expect("ok");
    assert_eq!(
        fs::read_to_string(dir.path().join("tests.rs")).expect("read"),
        "\n#[test]\nfn it() {}\n"
    );
}

#[test]
fn apply_path_identical_is_a_warning_in_the_report() {
    let (dir, src_path) = setup("foo.rs", "foo_tests.rs", Some("\n#[test]\nfn it() {}\n"));
    let report = apply_path(&src_path, false, None, ModRsTests::Sibling, false).expect("ok");
    let warnings = report.warnings();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings
            .first()
            .is_some_and(|msg| msg.ends_with("foo_tests.rs already holds these tests; left as is"))
    );
    assert!(
        !fs::read_to_string(&src_path)
            .expect("read")
            .contains("fn it")
    );
    assert!(dir.path().join("foo_tests.rs").exists());
}

#[test]
fn check_path_reports_what_apply_would_refuse() {
    let (_dir, src_path) = setup("foo.rs", "foo_tests.rs", Some("// mine\n"));
    let report = check_path(&src_path, None, ModRsTests::Sibling).expect("ok");
    assert_eq!(report.refusals().len(), 1);
}
