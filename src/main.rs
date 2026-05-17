use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;

use column_rs::{TableFormatOptions, format_table, parse_rows};

struct CliOptions {
    paths: Vec<String>,
    keep_empty_lines: bool,
    separators: Option<String>,
    output_separator: String,
}

const HELP_TEXT: &str = "\
Usage:
 column-rs [options] [<file>...]

Columnate lists.

Options:
 -t, --table                      create a table
 -L, --keep-empty-lines           don't ignore empty lines
 -o, --output-separator <string>  columns separator for table output (default is two spaces)
 -s, --separator <string>         possible table delimiters
 -h, --help                       display this help
 -V, --version                    display version";

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

fn parse_args(args: Vec<String>) -> Result<CliOptions, String> {
    let mut keep_empty_lines = false;
    let mut separators = None;
    let mut output_separator = "  ".to_string();
    let mut paths = Vec::new();

    let mut idx = 0usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--" {
            paths.extend(args[idx + 1..].iter().cloned());
            break;
        }
        if arg == "-h" || arg == "--help" {
            println!("{HELP_TEXT}");
            return Err(String::new());
        }
        if arg == "-V" || arg == "--version" {
            println!("column-rs {}", env!("CARGO_PKG_VERSION"));
            return Err(String::new());
        }
        if arg == "-t" || arg == "--table" {
            idx += 1;
            continue;
        }
        if arg == "-L" || arg == "--keep-empty-lines" {
            keep_empty_lines = true;
            idx += 1;
            continue;
        }
        if arg == "-s" || arg == "--separator" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -s/--separator".to_string());
            };
            separators = Some(next.clone());
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--separator=") {
            separators = Some(value.to_string());
            idx += 1;
            continue;
        }
        if arg == "-o" || arg == "--output-separator" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -o/--output-separator".to_string());
            };
            output_separator = next.clone();
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--output-separator=") {
            output_separator = value.to_string();
            idx += 1;
            continue;
        }
        if arg.starts_with('-') {
            return Err(format!("unsupported option: {arg}"));
        }
        paths.push(arg.clone());
        idx += 1;
    }

    Ok(CliOptions {
        paths,
        keep_empty_lines,
        separators,
        output_separator,
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(msg) if msg.is_empty() => return Ok(()),
        Err(msg) => return Err(msg),
    };

    let input = read_input(&options.paths)?;
    let rows = parse_rows(
        &input,
        options.separators.as_deref(),
        options.keep_empty_lines,
    );
    print!(
        "{}",
        format_table(
            &rows,
            &TableFormatOptions {
                output_separator: options.output_separator,
            }
        )
    );
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
