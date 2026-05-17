use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;

use column_rs::{format_table, parse_rows};

fn read_input(paths: &[String]) -> Result<String, String> {
    let mut data = String::new();

    if paths.is_empty() || paths.iter().any(|p| p == "-") {
        io::stdin()
            .read_to_string(&mut data)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
    }

    for path in paths {
        if path == "-" {
            continue;
        }

        let mut file = File::open(path).map_err(|e| format!("failed to open '{path}': {e}"))?;
        file.read_to_string(&mut data)
            .map_err(|e| format!("failed to read '{path}': {e}"))?;
        if !data.ends_with('\n') {
            data.push('\n');
        }
    }

    Ok(data)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut paths = Vec::new();
    for arg in args {
        if arg == "-t" || arg == "--table" {
            continue;
        }
        if arg == "-h" || arg == "--help" {
            println!("Usage: column-rs [-t|--table] [FILE ...]");
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
