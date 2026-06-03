use std::fs;

use assert_cmd::Command;
use predicates::prelude::{PredicateBooleanExt, predicate};
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("ejectest").expect("binary should exist")
}

fn sample_source() -> &'static str {
    concat!(
        "use std::collections::HashMap;\n",
        "\n",
        "pub fn add(aa: i32, bb: i32) -> i32 {\n",
        "    aa + bb\n",
        "}\n",
        "\n",
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    use super::*;\n",
        "\n",
        "    #[test]\n",
        "    fn test_add() {\n",
        "        assert_eq!(add(1, 2), 3);\n",
        "    }\n",
        "}\n",
    )
}

fn external_source() -> &'static str {
    "#[cfg(test)]\n#[path = \"ext_tests.rs\"]\nmod tests;\n"
}

fn no_tests_source() -> &'static str {
    "pub fn foo() -> i32 { 42 }\n"
}

fn write_sample(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write sample file");
    path
}

fn write_list(dir: &TempDir, name: &str, lines: &[&str]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, lines.join("\n")).expect("write file list");
    path
}

#[test]
fn apply_files_from_processes_only_listed() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let bb = write_sample(&dir, "bb.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[aa.to_str().expect("path")]);

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .success()
        .stdout(predicate::str::contains("aa_tests.rs"))
        .stdout(predicate::str::contains("bb_tests.rs").not());

    assert!(dir.path().join("aa_tests.rs").exists());
    assert!(!dir.path().join("bb_tests.rs").exists());
    let bb_after = fs::read_to_string(&bb).expect("read bb");
    assert_eq!(bb_after, sample_source());
}

#[test]
fn apply_files_from_stdin_processes_only_listed() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let _bb = write_sample(&dir, "bb.rs", sample_source());

    use std::io::Write;
    use std::process::Stdio;
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("ejectest"))
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(aa.to_str().expect("path").as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    assert!(output.status.success());
    assert!(dir.path().join("aa_tests.rs").exists());
    assert!(!dir.path().join("bb_tests.rs").exists());
}

#[test]
fn apply_files_from_outside_root_errors() {
    let dir = TempDir::new().expect("tempdir");
    let other = TempDir::new().expect("tempdir2");
    let outside = write_sample(&other, "outside.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[outside.to_str().expect("path")]);

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .failure()
        .stderr(predicate::str::contains("outside root"));
}

#[test]
fn apply_files_from_missing_file_errors() {
    let dir = TempDir::new().expect("tempdir");
    let list_path = dir.path().join("list.txt");
    fs::write(&list_path, "/nonexistent/path/foo.rs").expect("write list");

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot resolve path"));
}

#[test]
fn apply_files_from_lenient_skips_outside_and_missing() {
    let dir = TempDir::new().expect("tempdir");
    let other = TempDir::new().expect("tempdir2");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let outside = write_sample(&other, "outside.rs", sample_source());
    let list = write_list(
        &dir,
        "list.txt",
        &[
            aa.to_str().expect("path"),
            outside.to_str().expect("path"),
            "/nonexistent/foo.rs",
        ],
    );

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .arg("--lenient")
        .assert()
        .success();

    assert!(dir.path().join("aa_tests.rs").exists());
    assert!(!other.path().join("outside_tests.rs").exists());
}

#[test]
fn apply_files_from_dry_run_works() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[aa.to_str().expect("path")]);

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would create"));

    assert!(!dir.path().join("aa_tests.rs").exists());
}

#[test]
fn check_files_from_checks_only_listed() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let _bb = write_sample(&dir, "bb.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[aa.to_str().expect("path")]);

    cmd()
        .arg("check")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .failure()
        .stdout(predicate::str::contains("aa.rs"))
        .stdout(predicate::str::contains("bb.rs").not());
}

#[test]
fn check_files_from_clean_list_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", external_source());
    let bb = write_sample(&dir, "bb.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[aa.to_str().expect("path")]);

    cmd()
        .arg("check")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let bb_after = fs::read_to_string(&bb).expect("read bb");
    assert_eq!(bb_after, sample_source());
}

#[test]
fn check_files_from_outside_root_errors() {
    let dir = TempDir::new().expect("tempdir");
    let other = TempDir::new().expect("tempdir2");
    let outside = write_sample(&other, "outside.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[outside.to_str().expect("path")]);

    cmd()
        .arg("check")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .failure()
        .stderr(predicate::str::contains("outside root"));
}

#[test]
fn check_files_from_lenient_skips_invalid() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", external_source());
    let list = write_list(
        &dir,
        "list.txt",
        &[aa.to_str().expect("path"), "/nonexistent/foo.rs"],
    );

    cmd()
        .arg("check")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .arg("--lenient")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn apply_files_from_json_reports_correctly() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let bb = write_sample(&dir, "bb.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[aa.to_str().expect("path")]);

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"action\":\"ejected\""))
        .stdout(predicate::str::contains("\"total\":1"));

    let bb_after = fs::read_to_string(&bb).expect("read bb");
    assert_eq!(bb_after, sample_source());
}

#[test]
fn apply_files_from_empty_list_processes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    write_sample(&dir, "aa.rs", sample_source());
    let list = write_list(&dir, "list.txt", &[]);

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(!dir.path().join("aa_tests.rs").exists());
}

#[test]
fn apply_files_from_with_external_and_no_tests() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let bb = write_sample(&dir, "bb.rs", external_source());
    let cc = write_sample(&dir, "cc.rs", no_tests_source());
    let list = write_list(
        &dir,
        "list.txt",
        &[
            aa.to_str().expect("path"),
            bb.to_str().expect("path"),
            cc.to_str().expect("path"),
        ],
    );

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Summary: 1 ejected, 1 external, 1 no tests (3 scanned)",
        ));
}

#[test]
fn apply_files_from_lenient_with_missing_file() {
    let dir = TempDir::new().expect("tempdir");
    let aa = write_sample(&dir, "aa.rs", sample_source());
    let list = write_list(
        &dir,
        "list.txt",
        &[aa.to_str().expect("path"), "/nonexistent/missing.rs"],
    );

    cmd()
        .arg("apply")
        .arg(dir.path())
        .arg("--files-from")
        .arg(&list)
        .arg("--lenient")
        .assert()
        .success();

    assert!(dir.path().join("aa_tests.rs").exists());
}
