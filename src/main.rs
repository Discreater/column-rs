use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;
use std::{cmp, collections::HashSet};

use column_rs::{
    DEFAULT_OUTPUT_WIDTH, ListFormatOptions, Row, TableFormatOptions, format_list, format_table,
    format_table_json, parse_entries, parse_rows,
};

/// Parsed command-line options for selecting mode, formatting, and inputs.
struct CliOptions {
    paths: Vec<String>,
    table_mode: bool,
    keep_empty_lines: bool,
    separators: Option<String>,
    output_separator: String,
    output_width: usize,
    fill_rows: bool,
    json_output: bool,
    table_name: String,
    table_name_set: bool,
    table_columns: Option<Vec<String>>,
    table_noheadings: bool,
    table_hide: Vec<String>,
}

const HELP_TEXT: &str = "\
Usage:
 column-rs [options] [<file>...]

Columnate lists.

Options:
 -t, --table                      create a table
 -n, --table-name <name>          table name for JSON output
 -N, --table-columns <names>      comma separated columns names
 -H, --table-hide <columns>       don't print the columns
 -d, --table-noheadings           don't print header
 -J, --json                       use JSON output format for table
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
    let mut json_output = false;
    let mut table_name = "table".to_string();
    let mut table_name_set = false;
    let mut table_columns: Option<Vec<String>> = None;
    let mut table_noheadings = false;
    let mut table_hide: Vec<String> = Vec::new();
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
        if arg == "-J" || arg == "--json" {
            json_output = true;
            idx += 1;
            continue;
        }
        if arg == "-n" || arg == "--table-name" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -n/--table-name".to_string());
            };
            table_name = next.clone();
            table_name_set = true;
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--table-name=") {
            table_name = value.to_string();
            table_name_set = true;
            idx += 1;
            continue;
        }
        if arg == "-N" || arg == "--table-columns" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -N/--table-columns".to_string());
            };
            let columns = next
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if columns.is_empty() {
                return Err("invalid argument for -N/--table-columns".to_string());
            }
            table_columns = Some(columns);
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--table-columns=") {
            let columns = value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if columns.is_empty() {
                return Err("invalid argument for -N/--table-columns".to_string());
            }
            table_columns = Some(columns);
            idx += 1;
            continue;
        }
        if arg == "-d" || arg == "--table-noheadings" {
            table_noheadings = true;
            idx += 1;
            continue;
        }
        if arg == "-H" || arg == "--table-hide" {
            let Some(next) = args.get(idx + 1) else {
                return Err("missing argument for -H/--table-hide".to_string());
            };
            table_hide.extend(
                next.split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(ToString::to_string),
            );
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--table-hide=") {
            table_hide.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(ToString::to_string),
            );
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
        json_output,
        table_name,
        table_name_set,
        table_columns,
        table_noheadings,
        table_hide,
    })
}

/// Resolves `-H/--table-hide` specs into 0-based column indexes.
///
/// Each spec may be a 1-based numeric index or a column name from `--table-columns`.
/// Returns an error when a spec is out of range or refers to an undefined column name.
fn resolve_hidden_columns(
    max_cols: usize,
    table_columns: Option<&[String]>,
    hidden_specs: &[String],
) -> Result<HashSet<usize>, String> {
    let mut hidden = HashSet::new();
    for spec in hidden_specs {
        if let Ok(col_num) = spec.parse::<usize>() {
            if col_num == 0 || (max_cols > 0 && col_num > max_cols) {
                return Err(format!("undefined column name '{spec}'"));
            }
            hidden.insert(col_num - 1);
            continue;
        }
        let Some(columns) = table_columns else {
            return Err(format!("undefined column name '{spec}'"));
        };
        let Some(idx) = columns.iter().position(|name| name == spec) else {
            return Err(format!("undefined column name '{spec}'"));
        };
        hidden.insert(idx);
    }
    Ok(hidden)
}

/// Removes cells whose indexes are present in `hidden`, mutating the row in place.
fn hide_row_columns(row: &mut Row, hidden: &HashSet<usize>) {
    if let Row::Cells(cells) = row {
        let mut kept = Vec::with_capacity(cells.len());
        for (idx, cell) in cells.iter().enumerate() {
            if !hidden.contains(&idx) {
                kept.push(cell.clone());
            }
        }
        *cells = kept;
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(msg) if msg.is_empty() => return Ok(()),
        Err(msg) => return Err(msg),
    };
    if !options.table_mode
        && !options.json_output
        && (options.table_columns.is_some()
            || options.table_name_set
            || options.table_noheadings
            || !options.table_hide.is_empty())
    {
        return Err("option --table required for all --table-*".to_string());
    }

    let input = read_input(&options.paths)?;
    if options.table_mode || options.json_output {
        let mut rows = parse_rows(
            &input,
            options.separators.as_deref(),
            options.keep_empty_lines,
        );
        let max_cols = cmp::max(
            rows.iter()
                .map(|row| match row {
                    Row::Cells(cols) => cols.len(),
                    Row::Empty => 0,
                })
                .max()
                .unwrap_or(0),
            options.table_columns.as_ref().map_or(0, |cols| cols.len()),
        );
        let hidden_columns = resolve_hidden_columns(
            max_cols,
            options.table_columns.as_deref(),
            &options.table_hide,
        )?;
        if !hidden_columns.is_empty() {
            for row in &mut rows {
                hide_row_columns(row, &hidden_columns);
            }
        }
        if options.json_output {
            let columns = options.table_columns.as_ref().ok_or_else(|| {
                "option --table-columns or --table-column required for --json".to_string()
            })?;
            let filtered_columns = columns
                .iter()
                .enumerate()
                .filter_map(|(idx, name)| (!hidden_columns.contains(&idx)).then_some(name.clone()))
                .collect::<Vec<_>>();
            let out = format_table_json(&rows, &options.table_name, &filtered_columns)?;
            println!("{out}");
        } else {
            if let Some(columns) = options.table_columns.as_ref()
                && !options.table_noheadings
            {
                let header = columns
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, name)| {
                        (!hidden_columns.contains(&idx)).then_some(name.clone())
                    })
                    .collect::<Vec<_>>();
                rows.insert(0, Row::Cells(header));
            }
            print!(
                "{}",
                format_table(
                    &rows,
                    &TableFormatOptions {
                        output_separator: options.output_separator,
                    }
                )
            );
        }
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
