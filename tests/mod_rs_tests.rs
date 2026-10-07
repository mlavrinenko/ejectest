use std::fs;

use assert_cmd::Command;
use predicates::prelude::predicate;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("ejectest").expect("binary should exist")
}

fn sample_source() -> &'static str {
    "pub fn add(aa: i32, bb: i32) -> i32 {\n    aa + bb\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_add() {\n        assert_eq!(add(1, 2), 3);\n    }\n}\n"
}

fn mod_rs_dir(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let sub = dir.path().join("filter");
    fs::create_dir_all(&sub).expect("mkdir");
    let path = sub.join(name);
    fs::write(&path, sample_source()).expect("write");
    path
}

#[test]
fn mod_rs_tests_tests_writes_plain_tests_rs() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "mod.rs");

    cmd()
        .args(["apply", "--mod-rs-tests", "tests"])
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("tests.rs"));

    let modified = fs::read_to_string(&src_path).expect("read");
    assert!(modified.contains("#[cfg(test)]\nmod tests;"));
    assert!(!modified.contains("#[path"));
    let tests = fs::read_to_string(dir.path().join("filter/tests.rs")).expect("tests.rs");
    assert!(tests.contains("fn test_add"));
    assert!(!dir.path().join("filter/mod_tests.rs").exists());
}

#[test]
fn mod_rs_tests_sibling_is_the_default() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "mod.rs");

    cmd().arg("apply").arg(&src_path).assert().success();

    assert!(dir.path().join("filter/mod_tests.rs").exists());
    assert!(!dir.path().join("filter/tests.rs").exists());
    let modified = fs::read_to_string(&src_path).expect("read");
    assert!(modified.contains("#[path = \"mod_tests.rs\"]"));
}

#[test]
fn mod_rs_tests_tests_leaves_non_mod_rs_files_alone() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "eval.rs");

    cmd()
        .args(["apply", "--mod-rs-tests", "tests"])
        .arg(&src_path)
        .assert()
        .success();

    assert!(dir.path().join("filter/eval_tests.rs").exists());
    assert!(!dir.path().join("filter/tests.rs").exists());
    let modified = fs::read_to_string(&src_path).expect("read");
    assert!(modified.contains("#[path = \"eval_tests.rs\"]"));
}

#[test]
fn mod_rs_tests_tests_refuses_existing_tests_rs() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "mod.rs");
    let existing = dir.path().join("filter/tests.rs");
    fs::write(&existing, "// mine\n").expect("write");

    cmd()
        .args(["apply", "--mod-rs-tests", "tests"])
        .arg(&src_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("tests.rs"));

    assert_eq!(fs::read_to_string(&existing).expect("read"), "// mine\n");
    assert_eq!(
        fs::read_to_string(&src_path).expect("read"),
        sample_source()
    );
}

#[test]
fn mod_rs_tests_tests_json_names_file_written() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "mod.rs");

    cmd()
        .args(["apply", "--format", "json", "--mod-rs-tests", "tests"])
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"test_file\":\"tests.rs\""));
}

#[test]
fn check_treats_plain_mod_tests_as_external() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = mod_rs_dir(&dir, "mod.rs");
    cmd()
        .args(["apply", "--mod-rs-tests", "tests"])
        .arg(&src_path)
        .assert()
        .success();

    cmd().arg("check").arg(dir.path()).assert().success();
}
