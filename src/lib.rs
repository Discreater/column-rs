pub fn parse_rows(input: &str) -> Vec<Vec<String>> {
    input
        .lines()
        .filter_map(|line| {
            let cols: Vec<String> = line.split_whitespace().map(ToString::to_string).collect();
            if cols.is_empty() { None } else { Some(cols) }
        })
        .collect()
}

pub fn format_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let max_cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0usize; max_cols];
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count());
        }
    }

    let mut output = String::new();
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            output.push_str(cell);
            if idx < row.len() - 1 {
                let pad = widths[idx].saturating_sub(cell.chars().count()) + 2;
                for _ in 0..pad {
                    output.push(' ');
                }
            }
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{format_table, parse_rows};

    #[test]
    fn parse_rows_skips_blank_lines() {
        let rows = parse_rows("a b\n\n c   d \n");
        assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn format_table_aligns_columns() {
        let rows = vec![
            vec!["name".to_string(), "age".to_string()],
            vec!["alice".to_string(), "8".to_string()],
            vec!["bob".to_string(), "12".to_string()],
        ];

        let out = format_table(&rows);
        assert_eq!(out, "name   age\nalice  8\nbob    12\n");
    }
}
