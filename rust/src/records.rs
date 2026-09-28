//! Shared response records, query parsing, and list rendering.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Serialize)]
pub(crate) struct EmptyRequest {}

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

/// Parse accepted ISO 8601 forms while preserving nanoseconds.
/// Missing time components and timezones default to midnight and UTC.
pub(crate) fn parse_timestamp_value(
    raw: &str,
    label: &str,
) -> Result<Option<OffsetDateTime>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_datetime(raw, label)?))
}

pub(crate) fn parse_timestamp_millis(raw: &str, label: &str) -> Result<String, String> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    let parsed = parse_datetime(raw, label)?;
    parsed
        .replace_nanosecond(u32::from(parsed.millisecond()) * 1_000_000)
        .map_err(|error| format!("failed to parse {label} time: {error}"))?
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format {label} time: {error}"))
}

fn parse_datetime(raw: &str, label: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(raw, &Rfc3339)
        .map_err(|error| error.to_string())
        .or_else(|_| parse_iso8601(raw))
        .map_err(|error| format!("failed to parse {label} time: {error}"))
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
        // Accept offsets written as +HH, +HHMM, or +HH:MM.
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
    // Accept unpadded clock components.
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

/// Default page size for user-facing list requests.
pub(crate) const DEFAULT_LIST_LIMIT: i64 = 50;

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

pub(crate) fn warn_if_has_next_cursor(mut outcome: Outcome, next_cursor: Option<&str>) -> Outcome {
    if outcome.exit_code == 0 {
        if let Some(cursor) = next_cursor.filter(|cursor| !cursor.is_empty()) {
            outcome.stderr.extend_from_slice(
                format!(
                    "More results exist; rerun with --cursor {cursor} to fetch the next page.\n"
                )
                .as_bytes(),
            );
        }
    }
    outcome
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListOutput<'a, T> {
    data: &'a [T],
    #[serde(skip_serializing_if = "NextCursor::is_not_paginated")]
    next_cursor: NextCursor<'a>,
}

pub(crate) enum NextCursor<'a> {
    NotPaginated,
    Page(Option<&'a str>),
}

impl NextCursor<'_> {
    fn is_not_paginated(&self) -> bool {
        matches!(self, Self::NotPaginated)
    }
}

impl Serialize for NextCursor<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::NotPaginated => serializer.serialize_unit(),
            Self::Page(cursor) => cursor.serialize(serializer),
        }
    }
}

pub(crate) fn format_output<T: Record>(records: &[T], format: Format) -> Outcome {
    format_list_output(records, format, NextCursor::NotPaginated)
}

pub(crate) fn format_list_output<T: Record>(
    records: &[T],
    format: Format,
    next_cursor: NextCursor<'_>,
) -> Outcome {
    let mut stdout = Vec::new();
    let result = match format {
        Format::Table => output::render_table(
            &mut stdout,
            T::headers(),
            &records.iter().map(Record::fields).collect::<Vec<_>>(),
        ),
        Format::Json => output::render_json(
            &mut stdout,
            &ListOutput {
                data: records,
                next_cursor,
            },
        ),
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

pub(crate) fn format_record<T: Record>(record: &T, format: Format) -> Outcome {
    if format != Format::Json {
        return format_output(std::slice::from_ref(record), format);
    }
    let mut stdout = Vec::new();
    match output::render_json(&mut stdout, record) {
        Ok(()) => Outcome::success(stdout),
        Err(error) => Outcome::failure(format!("failed to render output: {error}\n")),
    }
}

pub(crate) fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

pub(crate) async fn fetch_list<T, Q>(
    runtime: &Runtime,
    format: Format,
    prefix: &str,
    endpoint: &str,
    query: &Q,
    limit: Option<i64>,
) -> Outcome
where
    T: Record + DeserializeOwned,
    Q: Serialize + ?Sized,
{
    match runtime.client.get::<_, Vec<T>>(endpoint, query).await {
        Ok(records) => {
            let outcome = format_output(&records, format);
            match limit {
                Some(limit) => warn_if_truncated(outcome, records.len(), limit),
                None => outcome,
            }
        }
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

    #[derive(Serialize)]
    struct TestRecord {
        id: &'static str,
    }

    impl Record for TestRecord {
        fn headers() -> &'static [&'static str] {
            &["ID"]
        }

        fn fields(&self) -> Vec<String> {
            vec![self.id.to_owned()]
        }
    }

    #[test]
    fn json_lists_are_enveloped_with_a_cursor() {
        let outcome = format_list_output(
            &[TestRecord { id: "one" }],
            Format::Json,
            NextCursor::Page(Some("next_page")),
        );
        assert_eq!(
            outcome.stdout,
            b"{\"data\":[{\"id\":\"one\"}],\"nextCursor\":\"next_page\"}\n"
        );
    }

    #[test]
    fn json_lists_without_pagination_omit_the_cursor() {
        let outcome = format_output(&[TestRecord { id: "one" }], Format::Json);
        assert_eq!(outcome.stdout, b"{\"data\":[{\"id\":\"one\"}]}\n");
    }

    #[test]
    fn final_paginated_json_list_has_a_null_cursor() {
        let outcome = format_list_output(
            &[TestRecord { id: "one" }],
            Format::Json,
            NextCursor::Page(None),
        );
        assert_eq!(
            outcome.stdout,
            b"{\"data\":[{\"id\":\"one\"}],\"nextCursor\":null}\n"
        );
    }

    #[test]
    fn a_full_page_warns_how_to_continue() {
        let outcome = warn_if_truncated(Outcome::success([]), 50, 50);
        assert_eq!(
            outcome.stderr,
            b"Showing the first 50 results. More may exist; use --offset to page through them.\n"
        );
    }

    #[test]
    fn timestamps_preserve_accepted_iso8601_forms_and_fractional_seconds() {
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
            // Retain acceptance of extra digits, discarding sub-nanosecond precision.
            (
                "2026-09-14T12:34:56.1234567899Z",
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
    fn millisecond_timestamps_keep_fractions_down_to_the_millisecond() {
        for (input, expected) in [
            ("", ""),
            ("2026-09-14", "2026-09-14T00:00:00Z"),
            ("2026-09-14T12:34:56.25Z", "2026-09-14T12:34:56.25Z"),
            ("2026-09-14T12:34:56.123456789", "2026-09-14T12:34:56.123Z"),
            ("2026-09-14T12:34:56.9999Z", "2026-09-14T12:34:56.999Z"),
            ("2026-09-14T12:34:56.5+0545", "2026-09-14T12:34:56.5+05:45"),
            ("2026-09-14T1:2:3.004", "2026-09-14T01:02:03.004Z"),
        ] {
            assert_eq!(
                parse_timestamp_millis(input, "start").unwrap(),
                expected,
                "{input}"
            );
        }
        assert!(parse_timestamp_millis("2026-02-30", "end")
            .unwrap_err()
            .starts_with("failed to parse end time:"));
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
