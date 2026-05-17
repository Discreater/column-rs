use serde_json::{Map, Value};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Cells(Vec<String>),
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableFormatOptions {
    pub output_separator: String,
}

impl Default for TableFormatOptions {
    fn default() -> Self {
        Self {
            output_separator: "  ".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListFormatOptions {
    pub output_width: usize,
    pub fill_rows: bool,
}

pub const DEFAULT_OUTPUT_WIDTH: usize = 80;

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

impl Default for ListFormatOptions {
    fn default() -> Self {
        Self {
            output_width: DEFAULT_OUTPUT_WIDTH,
            fill_rows: false,
        }
    }
}

pub fn parse_rows(input: &str, separators: Option<&str>, keep_empty_lines: bool) -> Vec<Row> {
    input
        .lines()
        .filter_map(|line| {
            if line.is_empty() {
                return keep_empty_lines.then_some(Row::Empty);
            }

            let cols = match separators {
                Some(seps) => line
                    .split(|c| seps.contains(c))
                    .map(str::trim)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                None => line
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            };

            if cols.is_empty() {
                keep_empty_lines.then_some(Row::Empty)
            } else {
                Some(Row::Cells(cols))
            }
        })
        .collect()
}

pub fn parse_entries(input: &str, separators: Option<&str>, keep_empty_lines: bool) -> Vec<String> {
    let mut entries = Vec::new();
    for line in input.lines() {
        if line.is_empty() {
            if keep_empty_lines {
                entries.push(String::new());
            }
            continue;
        }

        match separators {
            Some(seps) => {
                for item in line.split(|c| seps.contains(c)).map(str::trim) {
                    entries.push(item.to_string());
                }
            }
            None => {
                for item in line.split_whitespace() {
                    entries.push(item.to_string());
                }
            }
        }
    }
    entries
}

pub fn format_table(rows: &[Row], options: &TableFormatOptions) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let max_cols = rows
        .iter()
        .map(|row| match row {
            Row::Cells(cols) => cols.len(),
            Row::Empty => 0,
        })
        .max()
        .unwrap_or(0);
    let mut widths = vec![0usize; max_cols];
    for row in rows {
        if let Row::Cells(cols) = row {
            for (idx, cell) in cols.iter().enumerate() {
                widths[idx] = widths[idx].max(display_width(cell));
            }
        }
    }

    let mut output = String::new();
    for row in rows {
        match row {
            Row::Cells(cols) => {
                for (idx, cell) in cols.iter().enumerate() {
                    output.push_str(cell);
                    if idx < cols.len() - 1 {
                        let pad = widths[idx].saturating_sub(display_width(cell));
                        for _ in 0..pad {
                            output.push(' ');
                        }
                        output.push_str(&options.output_separator);
                    }
                }
            }
            Row::Empty => {
                if max_cols > 1 {
                    for width in widths.iter().take(max_cols - 1) {
                        for _ in 0..*width {
                            output.push(' ');
                        }
                        output.push_str(&options.output_separator);
                    }
                }
            }
        }
        output.push('\n');
    }

    output
}

pub fn format_table_json(
    rows: &[Row],
    table_name: &str,
    column_names: &[String],
) -> Result<String, String> {
    if column_names.is_empty() {
        return Err("option --table-columns or --table-column required for --json".to_string());
    }

    let mut table_rows = Vec::with_capacity(rows.len());
    for (row_idx, row) in rows.iter().enumerate() {
        let mut obj = Map::new();
        match row {
            Row::Cells(cols) => {
                if cols.len() > column_names.len() {
                    return Err(format!(
                        "line {}: for JSON the name of the column {} is required",
                        row_idx + 1,
                        cols.len()
                    ));
                }
                for (idx, name) in column_names.iter().enumerate() {
                    let value = cols
                        .get(idx)
                        .map_or(Value::Null, |cell| Value::String(cell.clone()));
                    obj.insert(name.clone(), value);
                }
            }
            Row::Empty => {
                for name in column_names {
                    obj.insert(name.clone(), Value::Null);
                }
            }
        }
        table_rows.push(Value::Object(obj));
    }

    let mut root = Map::new();
    root.insert(table_name.to_string(), Value::Array(table_rows));
    serde_json::to_string_pretty(&Value::Object(root))
        .map_err(|e| format!("failed to format json: {e}"))
}

/// Cached metrics for a list layout candidate under a fixed column count.
struct ListLayoutMetrics {
    rows: usize,
    widths: Vec<usize>,
    entry_display_widths: Vec<usize>,
}

/// Computes row count and per-column widths for a list layout candidate.
fn list_layout_widths(entries: &[String], cols: usize, fill_rows: bool) -> ListLayoutMetrics {
    let rows = entries.len().div_ceil(cols);
    let mut widths = vec![0usize; cols];
    let mut entry_display_widths = Vec::with_capacity(entries.len());
    for entry in entries {
        entry_display_widths.push(display_width(entry));
    }

    for (idx, width) in entry_display_widths.iter().enumerate() {
        let col = if fill_rows { idx % cols } else { idx / rows };
        widths[col] = widths[col].max(*width);
    }

    ListLayoutMetrics {
        rows,
        widths,
        entry_display_widths,
    }
}

pub fn format_list(entries: &[String], options: &ListFormatOptions) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut best_cols = 1usize;
    let mut best_rows = entries.len();
    let mut best_widths = vec![display_width(&entries[0])];
    let mut best_entry_display_widths =
        entries.iter().map(|v| display_width(v)).collect::<Vec<_>>();

    for cols in 1..=entries.len() {
        let metrics = list_layout_widths(entries, cols, options.fill_rows);
        let line_width = metrics.widths.iter().sum::<usize>() + cols.saturating_sub(1) * 2;
        if line_width <= options.output_width {
            best_cols = cols;
            best_rows = metrics.rows;
            best_widths = metrics.widths;
            best_entry_display_widths = metrics.entry_display_widths;
        } else {
            break;
        }
    }

    let mut out = String::new();
    for row in 0..best_rows {
        let mut first = true;
        for col in 0..best_cols {
            let idx = if options.fill_rows {
                row * best_cols + col
            } else {
                col * best_rows + row
            };
            if idx >= entries.len() {
                continue;
            }
            if !first {
                let prev_col = col - 1;
                let prev_idx = if options.fill_rows {
                    row * best_cols + prev_col
                } else {
                    prev_col * best_rows + row
                };
                let prev_len = best_entry_display_widths[prev_idx];
                let pad = best_widths[prev_col].saturating_sub(prev_len) + 2;
                for _ in 0..pad {
                    out.push(' ');
                }
            }
            out.push_str(&entries[idx]);
            first = false;
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        ListFormatOptions, Row, TableFormatOptions, format_list, format_table, format_table_json,
        parse_entries, parse_rows,
    };

    #[test]
    fn parse_rows_skips_blank_lines() {
        let rows = parse_rows("a b\n\n c   d \n", None, false);
        assert_eq!(
            rows,
            vec![
                Row::Cells(vec!["a".to_string(), "b".to_string()]),
                Row::Cells(vec!["c".to_string(), "d".to_string()])
            ]
        );
    }

    #[test]
    fn parse_rows_keeps_blank_lines() {
        let rows = parse_rows("a b\n\nc d\n", None, true);
        assert_eq!(
            rows,
            vec![
                Row::Cells(vec!["a".to_string(), "b".to_string()]),
                Row::Empty,
                Row::Cells(vec!["c".to_string(), "d".to_string()])
            ]
        );
    }

    #[test]
    fn parse_rows_uses_custom_separator() {
        let rows = parse_rows("a::b\n", Some(":"), false);
        assert_eq!(
            rows,
            vec![Row::Cells(vec![
                "a".to_string(),
                "".to_string(),
                "b".to_string()
            ])]
        );
    }

    #[test]
    fn parse_entries_parses_whitespace_words() {
        let entries = parse_entries("a b\nc d\n", None, false);
        assert_eq!(entries, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn parse_entries_keeps_empty_lines() {
        let entries = parse_entries("a\n\nb\n", None, true);
        assert_eq!(entries, vec!["a", "", "b"]);
    }

    #[test]
    fn format_table_aligns_columns() {
        let rows = vec![
            Row::Cells(vec!["name".to_string(), "age".to_string()]),
            Row::Cells(vec!["alice".to_string(), "8".to_string()]),
            Row::Cells(vec!["bob".to_string(), "12".to_string()]),
        ];

        let out = format_table(&rows, &TableFormatOptions::default());
        assert_eq!(out, "name   age\nalice  8\nbob    12\n");
    }

    #[test]
    fn format_table_aligns_display_width() {
        let rows = vec![
            Row::Cells(vec!["你".to_string(), "1".to_string()]),
            Row::Cells(vec!["ab".to_string(), "22".to_string()]),
        ];
        let out = format_table(&rows, &TableFormatOptions::default());
        assert_eq!(out, "你  1\nab  22\n");
    }

    #[test]
    fn format_table_keeps_empty_lines_shape() {
        let rows = vec![
            Row::Cells(vec!["a".to_string(), "b".to_string()]),
            Row::Empty,
            Row::Cells(vec!["c".to_string(), "d".to_string()]),
        ];
        let out = format_table(&rows, &TableFormatOptions::default());
        assert_eq!(out, "a  b\n   \nc  d\n");
    }

    #[test]
    fn format_table_supports_custom_output_separator() {
        let rows = vec![
            Row::Cells(vec!["a".to_string(), "b".to_string()]),
            Row::Cells(vec!["c".to_string(), "d".to_string()]),
        ];
        let out = format_table(
            &rows,
            &TableFormatOptions {
                output_separator: " | ".to_string(),
            },
        );
        assert_eq!(out, "a | b\nc | d\n");
    }

    #[test]
    fn format_table_json_emits_named_columns() {
        let rows = vec![
            Row::Cells(vec!["a".to_string(), "b".to_string()]),
            Row::Cells(vec!["c".to_string()]),
        ];
        let out = format_table_json(&rows, "table", &["c1".to_string(), "c2".to_string()])
            .expect("json formatting should succeed");
        let value: Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(
            value,
            serde_json::json!({
                "table": [
                    { "c1": "a", "c2": "b" },
                    { "c1": "c", "c2": null }
                ]
            })
        );
    }

    #[test]
    fn format_table_json_rejects_too_many_cells() {
        let rows = vec![Row::Cells(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ])];
        let err = format_table_json(&rows, "table", &["c1".to_string(), "c2".to_string()])
            .expect_err("should reject unnamed column");
        assert!(err.contains("name of the column 3 is required"));
    }

    #[test]
    fn format_list_default_fills_columns() {
        let entries = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
            "6".to_string(),
        ];
        let out = format_list(
            &entries,
            &ListFormatOptions {
                output_width: 4,
                fill_rows: false,
            },
        );
        assert_eq!(out, "1  4\n2  5\n3  6\n");
    }

    #[test]
    fn format_list_fill_rows() {
        let entries = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
            "6".to_string(),
        ];
        let out = format_list(
            &entries,
            &ListFormatOptions {
                output_width: 4,
                fill_rows: true,
            },
        );
        assert_eq!(out, "1  2\n3  4\n5  6\n");
    }

    #[test]
    fn format_list_uses_display_width_for_padding() {
        let entries = vec![
            "你".to_string(),
            "ab".to_string(),
            "cd".to_string(),
            "ef".to_string(),
        ];
        let out = format_list(
            &entries,
            &ListFormatOptions {
                output_width: 6,
                fill_rows: false,
            },
        );
        assert_eq!(out, "你  cd\nab  ef\n");
    }
}
