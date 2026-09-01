//! Shared infrastructure for read-only commands.

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::api::{ApiError, FoxgloveClient};
use crate::config::Config;
use crate::output::{self, Format};
use crate::Outcome;

pub(crate) const DEFAULT_CLIENT_ID: &str = "d51173be08ed4cf7a734aed9ac30afd0";
pub(crate) const DEFAULT_BASE_URL: &str = "https://api.foxglove.dev";
pub(crate) const USER_AGENT: &str = "foxglove-cli/v1.0.33";

#[derive(Debug)]
pub(crate) struct Runtime {
    pub(crate) client: FoxgloveClient,
    pub(crate) project_id: String,
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

pub(crate) fn runtime(matches: &ArgMatches) -> Result<Runtime, String> {
    let config = Config::load_default().map_err(|error| error.clone())?;
    let project_id = config.get_string("default_project_id").unwrap_or_default();
    let client_id = client_id(matches);
    let base_url = config
        .get_string("base_url")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    let token = config.get_string("bearer_token").unwrap_or_default();
    let client = FoxgloveClient::new(&base_url, client_id, token, USER_AGENT)
        .map_err(|error| error.to_string())?;
    Ok(Runtime { client, project_id })
}

pub(crate) fn client_id(matches: &ArgMatches) -> String {
    matches
        .get_one::<String>("client-id")
        .cloned()
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_owned())
}

pub(crate) fn resolve_format(matches: &ArgMatches) -> Result<Format, String> {
    Format::resolve(
        matches
            .try_get_many::<String>("format")
            .ok()
            .flatten()
            .and_then(|mut values| values.next_back().cloned())
            .filter(|value| !value.is_empty())
            .as_deref(),
        matches
            .try_get_one::<bool>("json")
            .ok()
            .flatten()
            .copied()
            .unwrap_or(false),
    )
}

pub(crate) fn last_value(matches: &ArgMatches, id: &str) -> Option<String> {
    matches
        .get_many::<String>(id)
        .and_then(|mut values| values.next_back().cloned())
        .or_else(|| matches.get_one::<String>(id).cloned())
}

pub(crate) fn value(matches: &ArgMatches, id: &str) -> String {
    last_value(matches, id).unwrap_or_default()
}

pub(crate) fn parse_i64(matches: &ArgMatches, id: &str, default: i64) -> Result<i64, String> {
    let raw = value(matches, id);
    if raw.is_empty() {
        return Ok(default);
    }
    raw.parse::<i64>()
        .map_err(|error| format!("invalid value for --{id}: {error}"))
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

pub(crate) fn query() -> Vec<(String, String)> {
    Vec::new()
}

pub(crate) fn add(
    query: &mut Vec<(String, String)>,
    key: &str,
    value: impl Display,
    include: bool,
) {
    if include {
        query.push((key.to_owned(), value.to_string()));
    }
}

pub(crate) fn add_str(query: &mut Vec<(String, String)>, key: &str, value: &str) {
    add(query, key, value, !value.is_empty());
}

pub(crate) fn sort_query(query: &mut [(String, String)]) {
    query.sort_by(|left, right| left.0.cmp(&right.0));
}

pub(crate) fn format_output<T: Record>(records: &[T], format: Format) -> Outcome {
    let mut stdout = Vec::new();
    let result = match format {
        Format::Table => output::render_table(
            &mut stdout,
            80,
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

pub(crate) fn finish_list<T, F, Fut>(
    runtime: &Runtime,
    format: Format,
    prefix: &str,
    operation: F,
) -> Outcome
where
    T: Record,
    F: FnOnce(FoxgloveClient) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<T>, ApiError>>,
{
    match block_on(operation(runtime.client.clone())) {
        Ok(records) => format_output(&records, format),
        Err(error) => Outcome::failure(format!("{prefix}: {error}\n")),
    }
}

pub(crate) fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build Tokio runtime")
        .block_on(future)
}

pub(crate) fn positional(matches: &ArgMatches, index: usize) -> String {
    matches
        .try_get_many::<String>("positionals")
        .ok()
        .flatten()
        .and_then(|mut values| values.nth(index).cloned())
        .unwrap_or_default()
}

pub(crate) fn session_key_error(matches: &ArgMatches, project_id: &str) -> Option<String> {
    if !value(matches, "session-key").is_empty() && project_id.is_empty() {
        Some("--project-id is required when using --session-key\n".to_owned())
    } else {
        None
    }
}

pub(crate) fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn human_readable_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut divisor = 1024_i64;
    let mut exponent = 0_usize;
    while bytes / divisor >= 1024 {
        divisor *= 1024;
        exponent += 1;
    }
    let suffix = "KMGTPE".as_bytes().get(exponent).copied().unwrap_or(b'E') as char;
    format!("{:.1} {suffix}iB", bytes as f64 / divisor as f64)
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
