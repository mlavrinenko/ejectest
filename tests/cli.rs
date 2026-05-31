use std::fs;

use assert_cmd::Command;
use predicates::prelude::predicate;
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

fn write_sample(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write sample file");
    path
}

#[test]
fn dry_run_shows_plan_without_writing() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "math.rs", sample_source());

    cmd()
        .arg("--dry-run")
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Would create"))
        .stdout(predicate::str::contains("math_tests.rs"));

    // Original file unchanged.
    let after = fs::read_to_string(&src_path).expect("read");
    assert_eq!(after, sample_source());

    // Test file not created.
    assert!(!dir.path().join("math_tests.rs").exists());
}

#[test]
fn extraction_creates_test_file_and_modifies_source() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "math.rs", sample_source());

    cmd()
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"))
        .stdout(predicate::str::contains("Modified"));

    let modified = fs::read_to_string(&src_path).expect("read modified");
    assert!(modified.contains("#[path = \"math_tests.rs\"]"));
    assert!(modified.contains("mod tests;"));
    assert!(!modified.contains("fn test_add"));

    let test_path = dir.path().join("math_tests.rs");
    let test_content = fs::read_to_string(test_path).expect("read test file");
    assert!(test_content.contains("fn test_add"));
    assert!(test_content.contains("use super::*;"));
}

#[test]
fn no_test_module_fails() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "empty.rs", "pub fn foo() -> i32 { 42 }\n");

    cmd()
        .arg(&src_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no inline"));
}

#[test]
fn already_external_fails() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(
        &dir,
        "ext.rs",
        "#[cfg(test)]\n#[path = \"ext_tests.rs\"]\nmod tests;\n",
    );

    cmd()
        .arg(&src_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already extracted"));
}

#[test]
fn version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn missing_file_fails() {
    cmd()
        .arg("/tmp/nonexistent_ejectest_file.rs")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn raw_string_in_test_module() {
    let dir = TempDir::new().expect("tempdir");
    let source = concat!(
        "pub fn foo() -> &'static str { \"hello\" }\n",
        "\n",
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    use super::*;\n",
        "    #[test]\n",
        "    fn test_foo() {\n",
        "        let expected = r#\"{\"key\": \"value\"}\"#;\n",
        "        assert_eq!(foo(), \"hello\");\n",
        "    }\n",
        "}\n",
    );
    let src_path = write_sample(&dir, "raw.rs", source);

    cmd().arg(&src_path).assert().success();

    let test_path = dir.path().join("raw_tests.rs");
    let test_content = fs::read_to_string(test_path).expect("read test file");
    assert!(test_content.contains("r#\"{\"key\": \"value\"}\"#"));
}

#[test]
fn allow_attrs_on_mod_become_inner_attrs() {
    let dir = TempDir::new().expect("tempdir");
    let source = concat!(
        "pub fn first(arr: &[i32]) -> i32 {\n",
        "    arr[0]\n",
        "}\n",
        "\n",
        "#[cfg(test)]\n",
        "#[allow(clippy::unwrap_used, clippy::indexing_slicing)]\n",
        "mod tests {\n",
        "    use super::*;\n",
        "    #[test]\n",
        "    fn test_first() {\n",
        "        assert_eq!(first(&[1, 2, 3]), 1);\n",
        "    }\n",
        "}\n",
    );
    let src_path = write_sample(&dir, "lift.rs", source);

    cmd().arg(&src_path).assert().success();

    let test_path = dir.path().join("lift_tests.rs");
    let test_content = fs::read_to_string(test_path).expect("read test file");
    assert!(
        test_content
            .starts_with("#![allow(clippy::unwrap_used, clippy::indexing_slicing)]\nuse super::*;")
    );

    // Stub keeps #[cfg(test)] but not the allow.
    let modified = fs::read_to_string(&src_path).expect("read modified");
    assert!(modified.contains("#[cfg(test)]"));
    assert!(modified.contains("#[path = \"lift_tests.rs\"]"));
    assert!(!modified.contains("#[allow"));
}

#[test]
fn no_trailing_blank_lines_after_extraction() {
    let dir = TempDir::new().expect("tempdir");
    let source = concat!(
        "pub fn foo() -> i32 { 42 }\n",
        "\n",
        "\n",
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    #[test]\n",
        "    fn test_foo() {}\n",
        "}\n",
    );
    let src_path = write_sample(&dir, "trail.rs", source);

    cmd().arg(&src_path).assert().success();

    let modified = fs::read_to_string(&src_path).expect("read modified");
    assert!(modified.ends_with("mod tests;\n"));
    assert!(!modified.ends_with("\n\n"));
}
