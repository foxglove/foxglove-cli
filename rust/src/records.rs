//! Shared response records, query parsing, and list rendering.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::output::{self, Format};
use crate::runtime::Runtime;
use crate::Outcome;

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

pub(crate) fn parse_timestamp(raw: &str, label: &str) -> Result<String, String> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    let parsed = OffsetDateTime::parse(raw, &Rfc3339).or_else(|_| {
        let format = time::format_description::parse("[year]-[month]-[day]")
            .map_err(|error| error.to_string())?;
        let date = Date::parse(raw, &format).map_err(|error| error.to_string())?;
        Ok::<_, String>(
            PrimitiveDateTime::new(date, time::Time::MIDNIGHT).assume_offset(UtcOffset::UTC),
        )
    });
    let parsed = parsed.map_err(|error| format!("failed to parse {label} time: {error}"))?;
    parsed
        .replace_nanosecond(0)
        .map_err(|error| format!("failed to parse {label} time: {error}"))?
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format {label} time: {error}"))
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

impl ProjectFallback for String {
    fn or_project(self, project_id: &str) -> String {
        if self.is_empty() {
            project_id.to_owned()
        } else {
            self
        }
    }
}
