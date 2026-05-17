use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;

use column_rs::{format_table, parse_rows};

fn append_with_trailing_newline(out: &mut String, chunk: &str) {
    out.push_str(chunk);
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

fn read_input(paths: &[String]) -> Result<String, String> {
    let mut data = String::new();

    if paths.is_empty() {
        io::stdin()
            .read_to_string(&mut data)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        return Ok(data);
    }

    for path in paths {
        if path == "-" {
            let mut stdin_buf = String::new();
            io::stdin()
                .read_to_string(&mut stdin_buf)
                .map_err(|e| format!("failed to read stdin: {e}"))?;
            append_with_trailing_newline(&mut data, &stdin_buf);
            continue;
        }

        let mut file_buf = String::new();
        let mut file = File::open(path).map_err(|e| format!("failed to open '{path}': {e}"))?;
        file.read_to_string(&mut file_buf)
            .map_err(|e| format!("failed to read '{path}': {e}"))?;
        append_with_trailing_newline(&mut data, &file_buf);
    }

    Ok(data)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut paths = Vec::new();
    for arg in args {
        if arg == "-h" || arg == "--help" {
            println!("Usage: column-rs [FILE ...]");
            return Ok(());
        }
        paths.push(arg);
    }

    let input = read_input(&paths)?;
    let rows = parse_rows(&input);
    print!("{}", format_table(&rows));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
