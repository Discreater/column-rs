use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::process::ExitCode;
use std::{cmp, collections::HashSet};

use clap::{Arg, ArgAction, Command, error::ErrorKind};
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
    let raw_args = args.clone();
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("column-rs".to_string());
    argv.extend(args);

    let matches = build_cli_command()
        .try_get_matches_from(argv)
        .map_err(|err| map_clap_error(err, &raw_args))?;

    if matches.get_flag("help") {
        println!("{HELP_TEXT}");
        return Err(String::new());
    }
    if matches.get_flag("version") {
        println!("column-rs {}", env!("CARGO_PKG_VERSION"));
        return Err(String::new());
    }

    let table_columns = matches
        .get_one::<String>("table-columns")
        .map(|value| parse_non_empty_csv(value, "-N/--table-columns"))
        .transpose()?;
    let table_hide = matches
        .get_many::<String>("table-hide")
        .map(|values| {
            values
                .flat_map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(ToString::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(CliOptions {
        paths: matches
            .get_many::<String>("file")
            .map(|values| values.map(ToString::to_string).collect())
            .unwrap_or_default(),
        table_mode: matches.get_flag("table"),
        keep_empty_lines: matches.get_flag("keep-empty-lines"),
        separators: matches
            .get_one::<String>("separator")
            .map(ToString::to_string),
        output_separator: matches
            .get_one::<String>("output-separator")
            .map_or_else(|| "  ".to_string(), ToString::to_string),
        output_width: matches
            .get_one::<usize>("output-width")
            .copied()
            .unwrap_or(DEFAULT_OUTPUT_WIDTH),
        fill_rows: matches.get_flag("fillrows"),
        json_output: matches.get_flag("json"),
        table_name: matches
            .get_one::<String>("table-name")
            .map_or_else(|| "table".to_string(), ToString::to_string),
        table_name_set: matches.contains_id("table-name"),
        table_columns,
        table_noheadings: matches.get_flag("table-noheadings"),
        table_hide,
    })
}

fn build_cli_command() -> Command {
    Command::new("column-rs")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("table")
                .short('t')
                .long("table")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("table-name")
                .short('n')
                .long("table-name")
                .value_name("name"),
        )
        .arg(
            Arg::new("table-columns")
                .short('N')
                .long("table-columns")
                .value_name("names"),
        )
        .arg(
            Arg::new("table-hide")
                .short('H')
                .long("table-hide")
                .value_name("columns")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("table-noheadings")
                .short('d')
                .long("table-noheadings")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("json")
                .short('J')
                .long("json")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output-width")
                .short('c')
                .long("output-width")
                .value_name("width")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("fillrows")
                .short('x')
                .long("fillrows")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("keep-empty-lines")
                .short('L')
                .long("keep-empty-lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output-separator")
                .short('o')
                .long("output-separator")
                .value_name("string"),
        )
        .arg(
            Arg::new("separator")
                .short('s')
                .long("separator")
                .value_name("string"),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("file").value_name("file").num_args(0..))
}

fn parse_non_empty_csv(value: &str, arg_name: &str) -> Result<Vec<String>, String> {
    let items = value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if items.is_empty() {
        return Err(format!("invalid argument for {arg_name}"));
    }
    Ok(items)
}

fn map_clap_error(err: clap::Error, args: &[String]) -> String {
    if err.kind() == ErrorKind::UnknownArgument
        && let Some(arg) = find_unknown_option(args)
    {
        return format!("unsupported option: {arg}");
    }

    err.to_string().trim().to_string()
}

fn find_unknown_option(args: &[String]) -> Option<String> {
    let known_flags = [
        "-t",
        "--table",
        "-x",
        "--fillrows",
        "-J",
        "--json",
        "-d",
        "--table-noheadings",
        "-L",
        "--keep-empty-lines",
        "-h",
        "--help",
        "-V",
        "--version",
    ];
    let value_options = [
        "-n",
        "--table-name",
        "-N",
        "--table-columns",
        "-H",
        "--table-hide",
        "-c",
        "--output-width",
        "-s",
        "--separator",
        "-o",
        "--output-separator",
    ];

    let mut idx = 0usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--" {
            break;
        }
        if arg == "-" || !arg.starts_with('-') {
            idx += 1;
            continue;
        }

        if let Some((name, _)) = arg.split_once('=')
            && value_options.contains(&name)
        {
            idx += 1;
            continue;
        }

        if known_flags.contains(&arg.as_str()) {
            idx += 1;
            continue;
        }

        if value_options.contains(&arg.as_str()) {
            idx += 2;
            continue;
        }

        return Some(arg.clone());
    }

    None
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
