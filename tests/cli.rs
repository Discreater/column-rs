use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

fn run_column(args: &[&str], stdin_data: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_column-rs"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn column-rs");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .expect("failed to write stdin");
    }

    let output = child.wait_with_output().expect("failed to wait for child");
    (
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
        output.status.code().unwrap_or(-1),
    )
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("column-rs-{name}-{nanos}.txt"))
}

#[test]
fn default_mode_columnates_list() {
    let (stdout, stderr, code) = run_column(&["-c", "4"], "1\n2\n3\n4\n5\n6\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "1  4\n2  5\n3  6\n");
}

#[test]
fn keeps_input_order_with_file_and_stdin() {
    let path = unique_temp_path("order");
    fs::write(&path, "file 1\n").expect("failed to write temp file");

    let path_string = path.to_string_lossy().to_string();
    let (stdout, stderr, code) = run_column(&["-t", path_string.as_str(), "-"], "stdin 2\n");

    fs::remove_file(&path).expect("failed to remove temp file");

    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "file   1\nstdin  2\n");
}

#[test]
fn supports_keep_empty_lines() {
    let (stdout, stderr, code) = run_column(&["-t", "-L"], "a b\n\nc d\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "a  b\n   \nc  d\n");
}

#[test]
fn supports_custom_separator() {
    let (stdout, stderr, code) = run_column(&["-t", "-s", ":"], "a::b\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "a    b\n");
}

#[test]
fn supports_custom_output_separator() {
    let (stdout, stderr, code) = run_column(&["-t", "-o", " | "], "a b\nc d\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "a | b\nc | d\n");
}

#[test]
fn supports_fill_rows_in_default_mode() {
    let (stdout, stderr, code) = run_column(&["-x", "-c", "4"], "1\n2\n3\n4\n5\n6\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "1  2\n3  4\n5  6\n");
}

#[test]
fn supports_explicit_table_mode() {
    let (stdout, stderr, code) = run_column(&["-t"], "name age\nalice 8\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "name   age\nalice  8\n");
}

#[test]
fn supports_table_columns_header_in_table_mode() {
    let (stdout, stderr, code) = run_column(&["-t", "-N", "c1,c2"], "a b\nc d\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "c1  c2\na   b\nc   d\n");
}

#[test]
fn supports_table_noheadings_with_named_columns() {
    let (stdout, stderr, code) = run_column(&["-t", "-N", "c1,c2", "-d"], "a b\nc d\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "a  b\nc  d\n");
}

#[test]
fn supports_table_hide_by_index() {
    let (stdout, stderr, code) = run_column(&["-t", "-H", "2"], "a b c\n1 2 3\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "a  c\n1  3\n");
}

#[test]
fn supports_table_hide_by_name() {
    let (stdout, stderr, code) =
        run_column(&["-t", "-N", "c1,c2,c3", "-H", "c2"], "a b c\n1 2 3\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "c1  c3\na   c\n1   3\n");
}

#[test]
fn supports_json_table_hide_by_name() {
    let (stdout, stderr, code) =
        run_column(&["--json", "-N", "c1,c2,c3", "-H", "c2"], "a b c\n1 2 3\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(
        value,
        json!({
            "table": [
                {"c1": "a", "c3": "c"},
                {"c1": "1", "c3": "3"}
            ]
        })
    );
}

#[test]
fn rejects_undefined_hidden_column() {
    let (_, stderr, code) = run_column(&["-t", "-H", "c2"], "a b c\n");
    assert_eq!(code, 1);
    assert!(stderr.contains("undefined column name 'c2'"));
}

#[test]
fn rejects_table_options_without_table_or_json_mode() {
    let (_, stderr, code) = run_column(&["-N", "c1,c2"], "");
    assert_eq!(code, 1);
    assert!(stderr.contains("option --table required for all --table-*"));
}

#[test]
fn rejects_unsupported_option() {
    let (_, stderr, code) = run_column(&["--tree", "id"], "");
    assert_eq!(code, 1);
    assert!(stderr.contains("unsupported option: --tree"));
}

#[test]
fn json_output_requires_table_columns() {
    let (_, stderr, code) = run_column(&["--json"], "a b\n");
    assert_eq!(code, 1);
    assert!(stderr.contains("option --table-columns or --table-column required for --json"));
}

#[test]
fn supports_json_output_with_named_columns() {
    let (stdout, stderr, code) = run_column(&["--json", "-N", "c1,c2"], "a b\nc d\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(
        value,
        json!({
            "table": [
                {"c1": "a", "c2": "b"},
                {"c1": "c", "c2": "d"}
            ]
        })
    );
}

#[test]
fn supports_json_custom_table_name() {
    let (stdout, stderr, code) = run_column(&["--json", "-N", "k,v", "-n", "mytab"], "a b\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(
        value,
        json!({
            "mytab": [
                {"k": "a", "v": "b"}
            ]
        })
    );
}
