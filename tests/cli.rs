use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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
fn formats_stdin_table() {
    let (stdout, stderr, code) = run_column(&["-t"], "name age\nalice 8\n");
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "name   age\nalice  8\n");
}

#[test]
fn keeps_input_order_with_file_and_stdin() {
    let path = unique_temp_path("order");
    fs::write(&path, "file 1\n").expect("failed to write temp file");

    let path_string = path.to_string_lossy().to_string();
    let (stdout, stderr, code) = run_column(&[path_string.as_str(), "-"], "stdin 2\n");

    let _ = fs::remove_file(&path);

    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    assert_eq!(stdout, "file   1\nstdin  2\n");
}
