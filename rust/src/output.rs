//! Shared output-format parsing and deterministic renderers.

use serde::Serialize;
use std::io::{self, Write};

/// Supported list-command output formats.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    #[default]
    Table,
    Json,
    Csv,
}

impl Format {
    /// Resolve the `--format` value.
    ///
    /// # Errors
    ///
    /// Returns an unknown-format message when the value is unsupported.
    pub fn resolve(format: Option<&str>) -> Result<Self, String> {
        match format.unwrap_or("table") {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            value => Err(format!("Command failed. unknown output format: {value}\n")),
        }
    }
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

/// Render a simple, pipe-delimited table.
///
/// # Errors
///
/// Returns invalid-row or output-write errors.
pub fn render_table(
    writer: &mut dyn Write,
    headers: &[&str],
    rows: &[Vec<String>],
) -> io::Result<()> {
    if rows.is_empty() {
        return writer.write_all(b"No records found\n");
    }
    validate_rows(headers, rows)?;
    writeln!(writer, "{}", headers.join(" | "))?;
    writeln!(
        writer,
        "{}",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    )?;
    for row in rows {
        writeln!(
            writer,
            "{}",
            row.iter()
                .map(|cell| cell.replace('|', "\\|").replace(['\r', '\n'], "\\n"))
                .collect::<Vec<_>>()
                .join(" | ")
        )?;
    }
    Ok(())
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
    use super::{render_csv, render_json, render_table, Format};

    #[test]
    fn parses_requested_format() {
        assert_eq!(Format::resolve(Some("csv")), Ok(Format::Csv));
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
    fn json_uses_serde_json_output() {
        let mut output = Vec::new();
        render_json(&mut output, &vec![serde_json::json!({"id": "one"})]).unwrap();
        assert_eq!(output, b"[{\"id\":\"one\"}]\n");
    }

    #[test]
    fn table_is_pipe_delimited() {
        let mut output = Vec::new();
        render_table(
            &mut output,
            &["ID", "Name"],
            &[vec!["dev_1".into(), "Robot".into()]],
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "ID | Name\n--- | ---\ndev_1 | Robot\n"
        );
    }

    #[test]
    fn table_escapes_cell_delimiters_and_newlines() {
        let mut output = Vec::new();
        render_table(&mut output, &["Value"], &[vec!["left|right\nnext".into()]]).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Value\n---\nleft\\|right\\nnext\n"
        );
    }
}
