//! Shared response records, query parsing, and list rendering.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime};

use crate::output::{self, Format};
use crate::runtime::Runtime;
use crate::Outcome;

pub(crate) fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct DeviceSummary {
    pub(crate) name: String,
    pub(crate) id: String,
}

pub(crate) trait Record: Serialize {
    fn headers() -> &'static [&'static str];
    fn fields(&self) -> Vec<String>;
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) const fn is_zero(value: &i64) -> bool {
    *value == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) const fn is_false(value: &bool) -> bool {
    !*value
}

/// Parse the ISO 8601 forms accepted by the Go CLI, preserving nanoseconds.
/// Missing time components and timezones default to midnight and UTC.
pub(crate) fn parse_timestamp_value(
    raw: &str,
    label: &str,
) -> Result<Option<OffsetDateTime>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    let parsed = OffsetDateTime::parse(raw, &Rfc3339)
        .map_err(|error| error.to_string())
        .or_else(|_| parse_iso8601(raw))
        .map_err(|error| format!("failed to parse {label} time: {error}"))?;
    Ok(Some(parsed))
}

fn parse_iso8601(raw: &str) -> Result<OffsetDateTime, String> {
    let (date, clock) = raw.split_once('T').unwrap_or((raw, ""));
    let date_format =
        time::format_description::parse("[year]-[month padding:none]-[day padding:none]")
            .map_err(|error| error.to_string())?;
    let date = Date::parse(date.strip_prefix('+').unwrap_or(date), &date_format)
        .map_err(|error| error.to_string())?;
    let (clock, zone) = if let Some(clock) = clock.strip_suffix('Z') {
        (clock, "Z".to_owned())
    } else if let Some(index) = clock.find(['+', '-']) {
        let (clock, zone) = clock.split_at(index);
        // Go accepts offsets written as +HH, +HHMM, or +HH:MM.
        let zone = match zone.len() {
            3 => format!("{zone}:00"),
            5 if zone.is_ascii() => format!("{}:{}", &zone[..3], &zone[3..]),
            _ => zone.to_owned(),
        };
        (clock, zone)
    } else {
        (clock, "Z".to_owned())
    };
    let clock = if clock.is_empty() {
        "00:00:00".to_owned()
    } else {
        match clock.bytes().filter(|byte| *byte == b':').count() {
            0 => format!("{clock}:00:00"),
            1 => format!("{clock}:00"),
            _ => clock.to_owned(),
        }
    };
    // The Go parser also accepts unpadded clock components.
    let clock = clock
        .split(':')
        .map(|component| {
            let (whole, fraction) = component.split_once('.').unwrap_or((component, ""));
            let value = whole.parse::<u8>().map_err(|error| error.to_string())?;
            Ok::<_, String>(if component.contains('.') {
                format!("{value:02}.{fraction}")
            } else {
                format!("{value:02}")
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(":");
    OffsetDateTime::parse(&format!("{date}T{clock}{zone}"), &Rfc3339)
        .map_err(|error| error.to_string())
}

pub(crate) fn parse_timestamp(raw: &str, label: &str) -> Result<String, String> {
    parse_timestamp_value(raw, label)?.map_or_else(
        || Ok(String::new()),
        |parsed| {
            parsed
                .format(&Rfc3339)
                .map_err(|error| format!("failed to format {label} time: {error}"))
        },
    )
}

pub(crate) const DEFAULT_LIST_LIMIT: i64 = 2000;

pub(crate) fn warn_if_truncated(mut outcome: Outcome, count: usize, limit: i64) -> Outcome {
    if outcome.exit_code == 0 && limit > 0 && i64::try_from(count).is_ok_and(|count| count >= limit)
    {
        outcome.stderr.extend_from_slice(
            format!("Showing the first {limit} results. More may exist; use --offset to page through them.\n")
                .as_bytes(),
        );
    }
    outcome
}

pub(crate) fn format_output<T: Record>(records: &[T], format: Format) -> Outcome {
    let mut stdout = Vec::new();
    let result = match format {
        Format::Table => output::render_table(
            &mut stdout,
            T::headers(),
            &records.iter().map(Record::fields).collect::<Vec<_>>(),
        ),
        Format::Json => output::render_json(&mut stdout, records),
        Format::Csv => output::render_csv(
            &mut stdout,
            T::headers(),
            &records.iter().map(Record::fields).collect::<Vec<_>>(),
        ),
    };
    match result {
        Ok(()) => Outcome::success(stdout),
        Err(error) => Outcome::failure(format!("failed to render output: {error}\n")),
    }
}

pub(crate) async fn fetch_list<T, Q>(
    runtime: &Runtime,
    format: Format,
    prefix: &str,
    endpoint: &str,
    query: &Q,
) -> Outcome
where
    T: Record + DeserializeOwned,
    Q: Serialize + ?Sized,
{
    match runtime.client.get::<_, Vec<T>>(endpoint, query).await {
        Ok(records) => format_output(&records, format),
        Err(error) => Outcome::failure(format!("{prefix}: {error}\n")),
    }
}

pub(crate) fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

pub(crate) trait ProjectFallback {
    fn or_project(self, project_id: &str) -> String;
}

impl ProjectFallback for Option<String> {
    fn or_project(self, project_id: &str) -> String {
        self.unwrap_or_else(|| project_id.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_preserve_go_iso8601_forms_and_fractional_seconds() {
        for (input, expected) in [
            ("", ""),
            ("2026-09-14", "2026-09-14T00:00:00Z"),
            ("2026-09-14T", "2026-09-14T00:00:00Z"),
            ("2026-09-14T12", "2026-09-14T12:00:00Z"),
            ("2026-09-14T12:34", "2026-09-14T12:34:00Z"),
            ("2026-09-14T12:34:56", "2026-09-14T12:34:56Z"),
            (
                "2026-09-14T12:34:56.123456789",
                "2026-09-14T12:34:56.123456789Z",
            ),
            ("2026-09-14T12Z", "2026-09-14T12:00:00Z"),
            ("2026-09-14T12:34+05", "2026-09-14T12:34:00+05:00"),
            (
                "2026-09-14T12:34:56.123+0545",
                "2026-09-14T12:34:56.123+05:45",
            ),
            ("2026-09-14T12-06:30", "2026-09-14T12:00:00-06:30"),
            ("+2026-9-14T12:34:56Z", "2026-09-14T12:34:56Z"),
            ("2026-09-14T1:2:3.123", "2026-09-14T01:02:03.123Z"),
        ] {
            assert_eq!(
                parse_timestamp(input, "start").unwrap(),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn timestamps_reject_invalid_dates_times_and_offsets() {
        for input in [
            "not a date",
            "2026-02-30",
            "2026-09-14T24:00:00",
            "2026-09-14T12:60",
            "2026-09-14T12:00:00+1",
            "2026-09-14T12:00:00+05:60",
            "2026-09-14T12:00:00Ztrailing",
        ] {
            assert!(
                parse_timestamp(input, "end")
                    .unwrap_err()
                    .starts_with("failed to parse end time:"),
                "{input}"
            );
        }
    }
}
