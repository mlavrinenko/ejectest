//! Render [`Report`]s to text or JSON.

use std::path::Path;

use crate::{Classification, FileResult, OutputFormat, Report};

/// Render a `check` report.
#[must_use]
pub fn render_check(report: &Report, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => render_check_text(report),
        OutputFormat::Json => render_check_json(report),
    }
}

/// Render an `apply` report.
#[must_use]
pub fn render_apply(report: &Report, format: OutputFormat, dry_run: bool) -> String {
    match format {
        OutputFormat::Text => render_apply_text(report, dry_run),
        OutputFormat::Json => render_apply_json(report),
    }
}

fn render_check_text(report: &Report) -> String {
    let mut out = String::new();
    for res in &report.results {
        if res.classification == Classification::Inline {
            out.push_str(&res.path.display().to_string());
            out.push('\n');
        }
    }
    out
}

fn render_apply_text(report: &Report, dry_run: bool) -> String {
    let mut out = String::new();
    for res in &report.results {
        let test_path = test_path_for(res);
        if dry_run {
            out.push_str(&format!("Would create: {test_path}\n"));
            out.push_str(&format!("Would modify: {}\n", res.path.display()));
        } else {
            out.push_str(&format!("Created: {test_path}\n"));
            out.push_str(&format!("Modified: {}\n", res.path.display()));
        }
    }
    out
}

fn render_check_json(report: &Report) -> String {
    let files = report
        .results
        .iter()
        .map(|res| {
            format!(
                "{{\"path\":\"{}\",\"status\":\"{}\"}}",
                json_escape(&res.path.display().to_string()),
                status_str(res.classification),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let total = report.results.len();
    let inline = count(report, Classification::Inline);
    let external = count(report, Classification::External);
    let no_tests = count(report, Classification::NoTests);
    format!(
        "{{\"files\":[{files}],\"summary\":{{\"total\":{total},\"inline\":{inline},\"external\":{external},\"no_tests\":{no_tests}}}}}\n"
    )
}

fn render_apply_json(report: &Report) -> String {
    let files = report
        .results
        .iter()
        .map(apply_file_json)
        .collect::<Vec<_>>()
        .join(",");
    let total = report.results.len();
    let ejected = report.results.iter().filter(|res| res.applied).count();
    let would_eject = report
        .results
        .iter()
        .filter(|res| !res.applied && res.classification == Classification::Inline)
        .count();
    let external = count(report, Classification::External);
    let no_tests = count(report, Classification::NoTests);
    format!(
        "{{\"files\":[{files}],\"summary\":{{\"total\":{total},\"ejected\":{ejected},\"would_eject\":{would_eject},\"external\":{external},\"no_tests\":{no_tests}}}}}\n"
    )
}

fn apply_file_json(res: &FileResult) -> String {
    let path = json_escape(&res.path.display().to_string());
    let action = apply_action(res);
    match &res.test_file {
        Some(name) => format!(
            "{{\"path\":\"{path}\",\"action\":\"{action}\",\"test_file\":\"{}\"}}",
            json_escape(name),
        ),
        None => format!("{{\"path\":\"{path}\",\"action\":\"{action}\"}}"),
    }
}

fn apply_action(res: &FileResult) -> &'static str {
    match res.classification {
        Classification::Inline if res.applied => "ejected",
        Classification::Inline => "would_eject",
        Classification::External => "skipped_external",
        Classification::NoTests => "skipped_no_tests",
    }
}

fn status_str(classification: Classification) -> &'static str {
    match classification {
        Classification::Inline => "inline",
        Classification::External => "external",
        Classification::NoTests => "no_tests",
    }
}

fn count(report: &Report, target: Classification) -> usize {
    report
        .results
        .iter()
        .filter(|res| res.classification == target)
        .count()
}

fn test_path_for(res: &FileResult) -> String {
    match &res.test_file {
        Some(name) => {
            let parent = res.path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(name).display().to_string()
        }
        None => res.path.display().to_string(),
    }
}

/// Escape a string for embedding inside a JSON string literal.
fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 2);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => {
                let code = other as u32;
                if code < 0x20 {
                    out.push_str(&format!("\\u{code:04x}"));
                } else {
                    out.push(other);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn report(results: Vec<FileResult>) -> Report {
        Report { results }
    }

    fn inline(path: &str, applied: bool) -> FileResult {
        FileResult {
            path: PathBuf::from(path),
            classification: Classification::Inline,
            test_file: Some("foo_tests.rs".to_owned()),
            applied,
        }
    }

    fn plain(path: &str, classification: Classification) -> FileResult {
        FileResult {
            path: PathBuf::from(path),
            classification,
            test_file: None,
            applied: false,
        }
    }

    #[test]
    fn check_text_lists_only_inline() {
        let rep = report(vec![
            inline("src/foo.rs", false),
            plain("src/bar.rs", Classification::External),
            plain("src/baz.rs", Classification::NoTests),
        ]);
        let out = render_check(&rep, OutputFormat::Text);
        assert_eq!(out, "src/foo.rs\n");
    }

    #[test]
    fn check_text_silent_when_clean() {
        let rep = report(vec![plain("src/bar.rs", Classification::NoTests)]);
        assert_eq!(render_check(&rep, OutputFormat::Text), "");
    }

    #[test]
    fn check_json_has_summary_and_files() {
        let rep = report(vec![
            inline("src/foo.rs", false),
            plain("src/bar.rs", Classification::External),
        ]);
        let out = render_check(&rep, OutputFormat::Json);
        assert!(out.contains("\"path\":\"src/foo.rs\",\"status\":\"inline\""));
        assert!(out.contains("\"status\":\"external\""));
        assert!(
            out.contains("\"summary\":{\"total\":2,\"inline\":1,\"external\":1,\"no_tests\":0}")
        );
        assert!(out.ends_with("}\n"));
    }

    #[test]
    fn apply_json_reports_ejected() {
        let rep = report(vec![inline("src/foo.rs", true)]);
        let out = render_apply(&rep, OutputFormat::Json, false);
        assert!(out.contains("\"action\":\"ejected\""));
        assert!(out.contains("\"test_file\":\"foo_tests.rs\""));
        assert!(out.contains("\"ejected\":1,\"would_eject\":0"));
    }

    #[test]
    fn apply_json_reports_would_eject_on_dry_run() {
        let rep = report(vec![inline("src/foo.rs", false)]);
        let out = render_apply(&rep, OutputFormat::Json, true);
        assert!(out.contains("\"action\":\"would_eject\""));
        assert!(out.contains("\"would_eject\":1"));
    }

    #[test]
    fn apply_text_dry_run_says_would() {
        let rep = report(vec![inline("src/foo.rs", false)]);
        let out = render_apply(&rep, OutputFormat::Text, true);
        assert!(out.contains("Would create: src/foo_tests.rs"));
        assert!(out.contains("Would modify: src/foo.rs"));
    }

    #[test]
    fn json_escape_handles_quotes_and_control() {
        assert_eq!(json_escape("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape("x\ty"), "x\\ty");
        assert_eq!(json_escape("\u{1}"), "\\u0001");
    }
}
