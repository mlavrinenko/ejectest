//! The overwrite rule for an eject target: missing is written, identical is
//! left as is with a warning, anything else is refused unless `--overwrite`.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::predicate;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("ejectest").expect("binary should exist")
}

const SOURCE: &str = "pub fn add(aa: i32, bb: i32) -> i32 {\n    aa + bb\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_add() {\n        assert_eq!(add(1, 2), 3);\n    }\n}\n";

/// The test file `SOURCE` ejects to.
const TESTS: &str =
    "\nuse super::*;\n\n#[test]\nfn test_add() {\n    assert_eq!(add(1, 2), 3);\n}\n";

/// One `(source name, test file name, extra args)` per target form.
const CASES: [(&str, &str, &[&str]); 4] = [
    ("math.rs", "math_tests.rs", &[]),
    ("mod.rs", "mod_tests.rs", &[]),
    ("mod.rs", "tests.rs", &["--mod-rs-tests", "tests"]),
    ("lib.rs", "lib_tests.rs", &["--mod-rs-tests", "sibling"]),
];

struct Fixture {
    _dir: TempDir,
    src: PathBuf,
    test: PathBuf,
}

fn fixture(src_name: &str, test_name: &str, existing: Option<&str>) -> Fixture {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join(src_name);
    fs::write(&src, SOURCE).expect("write source");
    let test = dir.path().join(test_name);
    if let Some(content) = existing {
        fs::write(&test, content).expect("write existing");
    }
    Fixture {
        _dir: dir,
        src,
        test,
    }
}

#[test]
fn missing_target_is_written() {
    for (src, test, args) in CASES {
        let fx = fixture(src, test, None);
        cmd()
            .arg("apply")
            .args(args)
            .arg(&fx.src)
            .assert()
            .success();
        assert_eq!(fs::read_to_string(&fx.test).expect("test file"), TESTS);
        assert!(
            !fs::read_to_string(&fx.src)
                .expect("src")
                .contains("fn test_add")
        );
    }
}

#[test]
fn identical_target_warns_and_still_edits_source() {
    for (src, test, args) in CASES {
        let fx = fixture(src, test, Some(TESTS));
        cmd()
            .arg("apply")
            .args(args)
            .arg(&fx.src)
            .assert()
            .success()
            .stderr(predicate::str::contains(format!(
                "{} already holds these tests; left as is",
                fx.test.display()
            )));
        assert_eq!(fs::read_to_string(&fx.test).expect("test file"), TESTS);
        assert!(
            !fs::read_to_string(&fx.src)
                .expect("src")
                .contains("fn test_add")
        );
    }
}

#[test]
fn different_target_is_refused_and_nothing_is_written() {
    for (src, test, args) in CASES {
        let fx = fixture(src, test, Some("// mine\n"));
        cmd()
            .arg("apply")
            .args(args)
            .arg(&fx.src)
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!(
                "{} exists with other content; pass --overwrite to replace it",
                fx.test.display()
            )));
        assert_eq!(
            fs::read_to_string(&fx.test).expect("test file"),
            "// mine\n"
        );
        assert_eq!(fs::read_to_string(&fx.src).expect("src"), SOURCE);
    }
}

#[test]
fn overwrite_replaces_a_different_target() {
    for (src, test, args) in CASES {
        let fx = fixture(src, test, Some("// mine\n"));
        cmd()
            .args(["apply", "--overwrite"])
            .args(args)
            .arg(&fx.src)
            .assert()
            .success();
        assert_eq!(fs::read_to_string(&fx.test).expect("test file"), TESTS);
        assert!(
            !fs::read_to_string(&fx.src)
                .expect("src")
                .contains("fn test_add")
        );
    }
}

#[test]
fn directory_run_refuses_before_writing_anything() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("aaa.rs"), SOURCE).expect("write");
    fs::write(dir.path().join("zzz.rs"), SOURCE).expect("write");
    fs::write(dir.path().join("zzz_tests.rs"), "// mine\n").expect("write");

    cmd().arg("apply").arg(dir.path()).assert().failure();

    assert_eq!(
        fs::read_to_string(dir.path().join("aaa.rs")).expect("read"),
        SOURCE
    );
    assert!(!dir.path().join("aaa_tests.rs").exists());
}

#[test]
fn dry_run_reports_a_refusal_and_writes_nothing() {
    let fx = fixture("math.rs", "math_tests.rs", Some("// mine\n"));
    cmd()
        .args(["apply", "--dry-run", "--format", "json"])
        .arg(&fx.src)
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"action\":\"refused\""))
        .stdout(predicate::str::contains("\"target\":\"conflict\""))
        .stderr(predicate::str::contains("exists with other content"));
    assert_eq!(
        fs::read_to_string(&fx.test).expect("test file"),
        "// mine\n"
    );
    assert_eq!(fs::read_to_string(&fx.src).expect("src"), SOURCE);
}

#[test]
fn dry_run_reports_an_identical_target() {
    let fx = fixture("math.rs", "math_tests.rs", Some(TESTS));
    cmd()
        .args(["apply", "--dry-run", "--format", "json"])
        .arg(&fx.src)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"target\":\"identical\""))
        .stderr(predicate::str::contains(
            "already holds these tests; left as is",
        ));
}

#[test]
fn check_reports_the_target_state_in_json() {
    for (existing, state) in [
        (None, "missing"),
        (Some(TESTS), "identical"),
        (Some("// mine\n"), "conflict"),
    ] {
        let fx = fixture("math.rs", "math_tests.rs", existing);
        let assert = cmd()
            .args(["check", "--format", "json"])
            .arg(&fx.src)
            .assert()
            .failure()
            .stdout(predicate::str::contains(format!("\"target\":\"{state}\"")))
            .stdout(predicate::str::contains("\"test_file\":\"math_tests.rs\""));
        if state == "conflict" {
            assert.stderr(predicate::str::contains("exists with other content"));
        }
    }
}

#[test]
fn check_text_output_stays_one_path_per_line() {
    let fx = fixture("math.rs", "math_tests.rs", Some("// mine\n"));
    cmd()
        .arg("check")
        .arg(&fx.src)
        .assert()
        .failure()
        .stdout(format!("{}\n", fx.src.display()));
}
