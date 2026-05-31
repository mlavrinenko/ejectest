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

#[test]
fn apply_dry_run_shows_plan_without_writing() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "math.rs", sample_source());

    cmd()
        .arg("apply")
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
fn apply_creates_test_file_and_modifies_source() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "math.rs", sample_source());

    cmd()
        .arg("apply")
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
fn apply_no_test_module_fails() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "empty.rs", no_tests_source());

    cmd()
        .arg("apply")
        .arg(&src_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no inline"));
}

#[test]
fn apply_already_external_fails() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(
        &dir,
        "ext.rs",
        "#[cfg(test)]\n#[path = \"ext_tests.rs\"]\nmod tests;\n",
    );

    cmd()
        .arg("apply")
        .arg(&src_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already extracted"));
}

#[test]
fn apply_json_reports_ejected_file() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "math.rs", sample_source());

    cmd()
        .arg("apply")
        .arg("--format")
        .arg("json")
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"action\":\"ejected\""))
        .stdout(predicate::str::contains("\"test_file\":\"math_tests.rs\""))
        .stdout(predicate::str::contains("\"ejected\":1"));
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
fn apply_missing_file_fails() {
    cmd()
        .arg("apply")
        .arg("/tmp/nonexistent_ejectest_file.rs")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn apply_raw_string_in_test_module() {
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

    cmd().arg("apply").arg(&src_path).assert().success();

    let test_path = dir.path().join("raw_tests.rs");
    let test_content = fs::read_to_string(test_path).expect("read test file");
    assert!(test_content.contains("r#\"{\"key\": \"value\"}\"#"));
}

#[test]
fn apply_allow_attrs_on_mod_become_inner_attrs() {
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

    cmd().arg("apply").arg(&src_path).assert().success();

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
fn apply_no_trailing_blank_lines_after_extraction() {
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

    cmd().arg("apply").arg(&src_path).assert().success();

    let modified = fs::read_to_string(&src_path).expect("read modified");
    assert!(modified.ends_with("mod tests;\n"));
    assert!(!modified.ends_with("\n\n"));
}

// --- check mode ---

#[test]
fn check_clean_file_is_silent_and_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "clean.rs", external_source());

    cmd()
        .arg("check")
        .arg(&src_path)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn check_inline_file_fails_and_names_it() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = write_sample(&dir, "inline.rs", sample_source());

    cmd()
        .arg("check")
        .arg(&src_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("inline.rs"));

    // check must not modify anything.
    let after = fs::read_to_string(&src_path).expect("read");
    assert_eq!(after, sample_source());
    assert!(!dir.path().join("inline_tests.rs").exists());
}

#[test]
fn check_directory_recurses_and_reports_inline_only() {
    let dir = TempDir::new().expect("tempdir");
    let sub = dir.path().join("nested");
    fs::create_dir(&sub).expect("mkdir");

    write_sample(&dir, "inline.rs", sample_source());
    write_sample(&dir, "ejected.rs", external_source());
    write_sample(&dir, "plain.rs", no_tests_source());
    fs::write(sub.join("deep_inline.rs"), sample_source()).expect("write nested");

    cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("inline.rs"))
        .stdout(predicate::str::contains("deep_inline.rs"))
        .stdout(predicate::str::contains("ejected.rs").not())
        .stdout(predicate::str::contains("plain.rs").not());
}

#[test]
fn check_already_ejected_tree_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    write_sample(&dir, "a.rs", external_source());
    write_sample(&dir, "b.rs", no_tests_source());

    cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn check_json_emits_schema() {
    let dir = TempDir::new().expect("tempdir");
    write_sample(&dir, "inline.rs", sample_source());
    write_sample(&dir, "ext.rs", external_source());
    write_sample(&dir, "plain.rs", no_tests_source());

    cmd()
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg(dir.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"status\":\"inline\""))
        .stdout(predicate::str::contains("\"status\":\"external\""))
        .stdout(predicate::str::contains("\"status\":\"no_tests\""))
        .stdout(predicate::str::contains(
            "\"summary\":{\"total\":3,\"inline\":1,\"external\":1,\"no_tests\":1}",
        ));
}

#[test]
fn check_respects_gitignore() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join(".gitignore"), "ignored.rs\n").expect("write gitignore");
    write_sample(&dir, "ignored.rs", sample_source());
    write_sample(&dir, "tracked.rs", external_source());

    // Only the gitignored file has an inline module; it must be skipped,
    // so the check passes and is silent.
    cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}
