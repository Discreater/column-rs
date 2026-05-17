use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;

use column_rs::{
    DEFAULT_OUTPUT_WIDTH, ListFormatOptions, TableFormatOptions, format_list, format_table,
    parse_entries, parse_rows,
};

struct CliOptions {
    paths: Vec<String>,
    table_mode: bool,
    keep_empty_lines: bool,
    separators: Option<String>,
    output_separator: String,
    output_width: usize,
    fill_rows: bool,
}

const HELP_TEXT: &str = "\
Usage:
 column-rs [options] [<file>...]

Columnate lists.

Options:
 -t, --table                      create a table
 -c, --output-width <width>       width of output in number of characters
 -x, --fillrows                   fill rows before columns
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
    let mut table_mode = false;
    let mut keep_empty_lines = false;
    let mut separators = None;
    let mut output_separator = "  ".to_string();
    let mut output_width = DEFAULT_OUTPUT_WIDTH;
    let mut fill_rows = false;
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
            table_mode = true;
            idx += 1;
            continue;
        }
        if arg == "-x" || arg == "--fillrows" {
            fill_rows = true;
            idx += 1;
            continue;
        }
        if arg == "-c" || arg == "--output-width" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -c/--output-width".to_string());
            };
            output_width = next
                .parse::<usize>()
                .map_err(|_| format!("invalid output width: {next}"))?;
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--output-width=") {
            output_width = value
                .parse::<usize>()
                .map_err(|_| format!("invalid output width: {value}"))?;
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
        if arg == "-" {
            paths.push(arg.clone());
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
        table_mode,
        keep_empty_lines,
        separators,
        output_separator,
        output_width,
        fill_rows,
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
    if options.table_mode {
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
    } else {
        let entries = parse_entries(
            &input,
            options.separators.as_deref(),
            options.keep_empty_lines,
        );
        print!(
            "{}",
            format_list(
                &entries,
                &ListFormatOptions {
                    output_width: options.output_width,
                    fill_rows: options.fill_rows,
                }
            )
        );
    }
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
