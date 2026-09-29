//! Shared output-format parsing and deterministic renderers.

use comfy_table::{ColumnConstraint, ContentArrangement, LineStyle, Table, TableStyle};
use serde::Serialize;
use std::io::{self, IsTerminal, Write};

/// Supported list-command output formats.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, clap::ValueEnum)]
pub enum Format {
    #[default]
    Table,
    Json,
    Csv,
}

/// Render a JSON value using `serde_json`'s normal compact representation.
///
/// # Errors
///
/// Returns any serialization or output-write error.
pub fn render_json(writer: &mut dyn Write, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

/// Render RFC 4180-style CSV, including the header for an empty result.
///
/// # Errors
///
/// Returns any output-write error.
pub fn render_csv(
    writer: &mut dyn Write,
    headers: &[&str],
    rows: &[Vec<String>],
) -> io::Result<()> {
    let mut csv = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(&mut *writer);
    csv.write_record(headers).map_err(io::Error::other)?;
    for row in rows {
        csv.write_record(row).map_err(io::Error::other)?;
    }
    csv.flush().map_err(io::Error::other)
}

/// Render a table. When stdout is a terminal or `COLUMNS` is set, cells other
/// than IDs wrap to fit that width. ID columns never wrap, so a table with many
/// IDs can be wider than that width. Otherwise rows are not wrapped.
///
/// # Errors
///
/// Returns invalid-row or output-write errors.
pub fn render_table(
    writer: &mut dyn Write,
    headers: &[&str],
    rows: &[Vec<String>],
) -> io::Result<()> {
    render_table_at(writer, headers, rows, terminal_width())
}

fn render_table_at(
    writer: &mut dyn Write,
    headers: &[&str],
    rows: &[Vec<String>],
    width: Option<u16>,
) -> io::Result<()> {
    if rows.is_empty() {
        return writer.write_all(b"No records found\n");
    }
    validate_rows(headers, rows)?;
    let style = TableStyle::new().header_separator(LineStyle::none().fill('-').junction('-'));
    let mut table = Table::new();
    table
        .load_style(style)
        .set_header(headers.iter().copied())
        .add_rows(
            rows.iter()
                .map(|row| row.iter().map(|cell| escape_cell(cell))),
        );
    if let Some(width) = width {
        table
            .set_width(width)
            .set_content_arrangement(ContentArrangement::Dynamic);
        for (column, header) in table.column_iter_mut().zip(headers) {
            if is_id_header(header) {
                column.set_constraint(ColumnConstraint::ContentWidth);
            }
        }
    }
    writeln!(writer, "{table}")
}

fn is_id_header(header: &str) -> bool {
    header == "ID" || header.ends_with(" ID")
}

fn escape_cell(cell: &str) -> String {
    cell.replace(['\r', '\n'], "\\n")
}

fn terminal_width() -> Option<u16> {
    if let Some(columns) = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|columns| *columns > 0)
    {
        return Some(columns);
    }
    if !io::stdout().is_terminal() {
        return None;
    }
    std::process::Command::new("stty")
        .arg("size")
        .stdin(std::process::Stdio::inherit())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()?
                .split_whitespace()
                .nth(1)?
                .parse::<u16>()
                .ok()
        })
        .filter(|columns| *columns > 0)
        .or(Some(80))
}

fn validate_rows(headers: &[&str], rows: &[Vec<String>]) -> io::Result<()> {
    if rows.iter().any(|row| row.len() != headers.len()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "table row does not match header width",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_csv, render_json};

    #[test]
    fn empty_csv_contains_its_header() {
        let mut output = Vec::new();
        render_csv(&mut output, &["ID", "Name"], &[]).unwrap();
        assert_eq!(output, b"ID,Name\n");
    }

    #[test]
    fn csv_escapes_commas_quotes_and_newlines() {
        let mut output = Vec::new();
        render_csv(&mut output, &["Value"], &[vec!["a,\"b\"\nc".to_owned()]]).unwrap();
        assert_eq!(output, b"Value\n\"a,\"\"b\"\"\nc\"\n");
    }

    #[test]
    fn json_uses_serde_json_output() {
        let mut output = Vec::new();
        render_json(&mut output, &vec![serde_json::json!({"id": "one"})]).unwrap();
        assert_eq!(output, b"[{\"id\":\"one\"}]\n");
    }

    #[test]
    fn a_table_that_fits_keeps_every_cell_on_one_line() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["ID", "Name"],
            &[
                vec!["dev_1".into(), "Robot".into()],
                vec!["dev_longer".into(), "A".into()],
            ],
            Some(80),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            " ID           Name  \n\
             --------------------\n\
             \u{20}dev_1        Robot \n\
             \u{20}dev_longer   A     \n"
        );
    }

    #[test]
    fn a_table_too_wide_for_the_terminal_wraps_inside_its_columns() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["ID", "Notes"],
            &[vec!["dev_1".into(), "a".repeat(60)]],
            Some(40),
        )
        .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        for line in rendered.lines() {
            assert!(line.chars().count() <= 40, "{line:?}");
        }
        let joined: String = rendered.lines().flat_map(str::chars).collect();
        assert!(joined.matches('a').count() == 60, "{rendered}");
    }

    #[test]
    fn id_columns_never_wrap_in_a_narrow_table() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["ID", "Name", "Device ID"],
            &[vec![
                "rec_0edpo2ngbqJWmewz".into(),
                "a long recording name ".repeat(4),
                "dev_0eXGTrKQ5BVPyZmG".into(),
            ]],
            Some(60),
        )
        .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        let lines: Vec<_> = rendered.lines().collect();
        assert!(lines[2].starts_with(" rec_0edpo2ngbqJWmewz "), "{rendered}");
        assert!(lines[2].ends_with(" dev_0eXGTrKQ5BVPyZmG "), "{rendered}");
        assert!(lines.len() > 3, "{rendered}");
        for line in lines {
            assert!(line.chars().count() <= 60, "{line:?}");
        }
    }

    #[test]
    fn a_table_without_a_width_is_not_wrapped() {
        let mut output = Vec::new();
        let notes = "word ".repeat(40);
        super::render_table_at(
            &mut output,
            &["ID", "Notes"],
            &[vec!["dev_1".into(), notes.clone()]],
            None,
        )
        .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert_eq!(rendered.lines().count(), 3, "{rendered}");
        assert!(rendered.contains(notes.trim_end()), "{rendered}");
    }

    #[test]
    fn table_escapes_cell_newlines_and_keeps_pipes_verbatim() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["Value"],
            &[vec!["left|right\nnext".into()]],
            Some(80),
        )
        .unwrap();
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("left|right\\nnext"),);
    }
}
