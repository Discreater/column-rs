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
                widths[idx] = widths[idx].max(cell.chars().count());
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
                        let pad = widths[idx].saturating_sub(cell.chars().count());
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

#[cfg(test)]
mod tests {
    use super::{Row, TableFormatOptions, format_table, parse_rows};

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
}
