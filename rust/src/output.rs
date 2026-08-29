//! Shared output-format parsing and deterministic renderers.

use std::io::{self, Write};

use serde::Serialize;

/// Supported list-command output formats.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    #[default]
    Table,
    Json,
    Csv,
}

impl Format {
    /// Resolve the `--format` value and deprecated `--json` alias.
    ///
    /// # Errors
    ///
    /// Returns the compatible conflict or unknown-format message.
    pub fn resolve(format: Option<&str>, json: bool) -> Result<Self, String> {
        if json && format.is_some_and(|value| value != "json") {
            return Err("Command failed. Output format conflict: --json, --format\n".to_owned());
        }
        match format.unwrap_or(if json { "json" } else { "table" }) {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            value => Err(format!("Command failed. unknown output format: {value}\n")),
        }
    }
}

/// Render a JSON value with the Go CLI's four-space indentation and final newline.
///
/// # Errors
///
/// Returns any serialization or output-write error.
pub fn render_json(writer: &mut dyn Write, value: &impl Serialize) -> io::Result<()> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut *writer, formatter);
    value.serialize(&mut serializer).map_err(io::Error::other)?;
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
    write_csv_row(writer, headers.iter().copied())?;
    for row in rows {
        write_csv_row(writer, row.iter().map(String::as_str))?;
    }
    Ok(())
}

/// Render the Go CLI table layout for a known terminal width.
///
/// # Errors
///
/// Returns invalid-row or output-write errors.
pub fn render_table(
    writer: &mut dyn Write,
    terminal_width: usize,
    headers: &[&str],
    rows: &[Vec<String>],
) -> io::Result<()> {
    if rows.is_empty() {
        return writer.write_all(b"No records found\n");
    }
    validate_rows(headers, rows)?;
    let widths = cell_widths(headers, rows);
    let table_width = headers.len() + 1 + widths.iter().sum::<usize>();
    if terminal_width < table_width {
        render_hamburger(writer, terminal_width, headers, rows)
    } else {
        render_hotdog(writer, headers, rows, &widths)
    }
}

fn write_csv_row<'a>(
    writer: &mut dyn Write,
    fields: impl IntoIterator<Item = &'a str>,
) -> io::Result<()> {
    let mut first = true;
    for field in fields {
        if !first {
            writer.write_all(b",")?;
        }
        first = false;
        if field
            .bytes()
            .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'))
        {
            writer.write_all(b"\"")?;
            for part in field.split_inclusive('"') {
                writer.write_all(part.as_bytes())?;
                if part.ends_with('"') {
                    writer.write_all(b"\"")?;
                }
            }
            writer.write_all(b"\"")?;
        } else {
            writer.write_all(field.as_bytes())?;
        }
    }
    writer.write_all(b"\n")
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

fn cell_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths = headers
        .iter()
        .map(|header| header.len() + 4)
        .collect::<Vec<_>>();
    for row in rows {
        for (index, column) in row.iter().enumerate() {
            widths[index] = widths[index].max(column.len() + 2);
        }
    }
    for (width, header) in widths.iter_mut().zip(headers) {
        if (*width - header.len()) % 2 == 1 {
            *width += 1;
        }
    }
    widths
}

fn render_hotdog(
    writer: &mut dyn Write,
    headers: &[&str],
    rows: &[Vec<String>],
    widths: &[usize],
) -> io::Result<()> {
    writer.write_all(b"|")?;
    for (header, width) in headers.iter().zip(widths) {
        let padding = (width - header.len()) / 2;
        write!(
            writer,
            "{}{}{}|",
            " ".repeat(padding),
            header,
            " ".repeat(padding)
        )?;
    }
    writer.write_all(b"\n|")?;
    for width in widths {
        write!(writer, "{}|", "-".repeat(*width))?;
    }
    writer.write_all(b"\n")?;
    for row in rows {
        writer.write_all(b"|")?;
        for (column, width) in row.iter().zip(widths) {
            write!(writer, " {column}{}|", " ".repeat(width - column.len() - 1))?;
        }
        writer.write_all(b"\n")?;
    }
    Ok(())
}

fn render_hamburger(
    writer: &mut dyn Write,
    terminal_width: usize,
    headers: &[&str],
    rows: &[Vec<String>],
) -> io::Result<()> {
    let longest_record_header = format!("-[ RECORD {} ]", rows.len() + 1);
    let header_width = headers
        .iter()
        .map(|header| header.len())
        .max()
        .unwrap_or_default()
        .max(longest_record_header.len());
    let record_width = rows
        .iter()
        .flatten()
        .map(String::len)
        .max()
        .unwrap_or_default();
    let right_extent = (record_width + 15).min(terminal_width.saturating_sub(header_width + 1));
    let right_dashes = "-".repeat(right_extent);
    for (index, row) in rows.iter().enumerate() {
        let record_header = format!("-[ RECORD {} ]", index + 1);
        writeln!(
            writer,
            "{}{}+{}",
            record_header,
            "-".repeat(header_width - record_header.len()),
            right_dashes
        )?;
        for (header, column) in headers.iter().zip(row) {
            writeln!(
                writer,
                "{header:<header_width$}| {column:<value_width$}",
                value_width = right_extent.saturating_sub(1)
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_csv, render_json, render_table, Format};

    #[test]
    fn format_alias_conflicts_are_rejected() {
        assert_eq!(
            Format::resolve(Some("csv"), true),
            Err("Command failed. Output format conflict: --json, --format\n".into())
        );
    }

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
    fn json_uses_four_spaces() {
        let mut output = Vec::new();
        render_json(&mut output, &vec![serde_json::json!({"id": "one"})]).unwrap();
        assert_eq!(output, b"[\n    {\n        \"id\": \"one\"\n    }\n]\n");
    }

    #[test]
    fn table_matches_the_wide_go_layout() {
        let mut output = Vec::new();
        render_table(
            &mut output,
            80,
            &["ID", "Name"],
            &[vec!["dev_1".into(), "Robot".into()]],
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "|   ID   |  Name  |\n|--------|--------|\n| dev_1  | Robot  |\n"
        );
    }
}
