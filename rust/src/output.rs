//! Shared output-format parsing and deterministic renderers.

use comfy_table::{ContentArrangement, ContentLineStyle, LineStyle, Table, TableStyle};
use serde::Serialize;
use std::io::{self, Write};

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

/// Render a table, wrapping cells so the whole table fits the terminal.
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
    width: u16,
) -> io::Result<()> {
    if rows.is_empty() {
        return writer.write_all(b"No records found\n");
    }
    validate_rows(headers, rows)?;
    // The Go CLI's columns without its outer pipes: a `|` between cells, a rule
    // under the header, and a rule between records so a row that wraps onto a
    // second line is still one visible record.
    let style = TableStyle::new()
        .header_lines(ContentLineStyle::none().junction('|'))
        .content_lines(ContentLineStyle::none().junction('|'))
        .header_separator(LineStyle::none().fill('-').junction('|'))
        .row_separator(LineStyle::none().fill('-').junction('|'));
    let mut table = Table::new();
    table
        .load_style(style)
        .set_width(width)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers.iter().copied())
        .add_rows(
            rows.iter()
                .map(|row| row.iter().map(|cell| escape_cell(cell))),
        );
    writeln!(writer, "{table}")
}

/// A cell keeps its own delimiters out of the table it is printed in.
fn escape_cell(cell: &str) -> String {
    cell.replace('|', "\\|").replace(['\r', '\n'], "\\n")
}

/// comfy-table only detects the width on a real terminal, so a piped stdout
/// would otherwise arrange for unlimited width. `stty` reads the terminal on
/// stdin, so `list | head` still fits the window the reader is looking at.
fn terminal_width() -> u16 {
    if let Some(columns) = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|columns| *columns > 0)
    {
        return columns;
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
        .unwrap_or(80)
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
            80,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            " ID         | Name  \n\
             ------------|-------\n\
             \u{20}dev_1      | Robot \n\
             ------------|-------\n\
             \u{20}dev_longer | A     \n"
        );
    }

    #[test]
    fn a_table_too_wide_for_the_terminal_wraps_inside_its_columns() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["ID", "Notes"],
            &[vec!["dev_1".into(), "a".repeat(60)]],
            40,
        )
        .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        for line in rendered.lines() {
            assert!(line.chars().count() <= 40, "{line:?}");
        }
        // Nothing is dropped: every character of the long cell is still there.
        let joined: String = rendered.lines().flat_map(str::chars).collect();
        assert!(joined.matches('a').count() == 60, "{rendered}");
    }

    #[test]
    fn table_escapes_cell_delimiters_and_newlines() {
        let mut output = Vec::new();
        super::render_table_at(
            &mut output,
            &["Value"],
            &[vec!["left|right\nnext".into()]],
            80,
        )
        .unwrap();
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("left\\|right\\nnext"),);
    }
}
