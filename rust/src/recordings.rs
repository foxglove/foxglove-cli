//! Recording commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::output::Format;
use crate::read_helpers::{
    add, add_str, compact_json, finish_list, human_readable_bytes, parse_i64, parse_timestamp,
    query, session_key_error, sort_query, value, DeviceSummary, ProjectFallback, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Site {
    name: String,
    id: String,
}

fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MetadataRecord {
    name: String,
    metadata: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Recording {
    id: String,
    path: String,
    size: i64,
    #[serde(rename = "messageCount")]
    message_count: i64,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "importedAt")]
    imported_at: String,
    start: String,
    end: String,
    #[serde(rename = "importStatus")]
    import_status: String,
    #[serde(default, deserialize_with = "null_to_default")]
    site: Site,
    #[serde(rename = "edgeSite")]
    #[serde(default, deserialize_with = "null_to_default")]
    edge_site: Site,
    #[serde(default, deserialize_with = "null_to_default")]
    device: DeviceSummary,
    #[serde(default)]
    metadata: Option<Vec<MetadataRecord>>,
    #[serde(default, deserialize_with = "null_to_default")]
    key: String,
    #[serde(rename = "projectId")]
    project_id: String,
}
impl Record for Recording {
    fn headers() -> &'static [&'static str] {
        &[
            "Recording ID",
            "Path",
            "Size",
            "Message Count",
            "Created At",
            "Imported At",
            "Start",
            "End",
            "Import Status",
            "Site ID",
            "Site Name",
            "Edge Site ID",
            "Edge Site Name",
            "Device ID",
            "Device Name",
            "Metadata",
            "Key",
            "Project ID",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.path.clone(),
            human_readable_bytes(self.size),
            self.message_count.to_string(),
            self.created_at.clone(),
            self.imported_at.clone(),
            self.start.clone(),
            self.end.clone(),
            self.import_status.clone(),
            self.site.id.clone(),
            self.site.name.clone(),
            self.edge_site.id.clone(),
            self.edge_site.name.clone(),
            self.device.id.clone(),
            self.device.name.clone(),
            compact_json(&serde_json::to_value(&self.metadata).unwrap_or(Value::Null)),
            self.key.clone(),
            self.project_id.clone(),
        ]
    }
}
pub(crate) fn list_recordings(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    let start = match parse_timestamp(&value(matches, "start"), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&value(matches, "end"), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let mut query = query();
    add_str(&mut query, "device.id", &value(matches, "device-id"));
    add_str(&mut query, "device.name", &value(matches, "device-name"));
    add_str(&mut query, "edgeSite.id", &value(matches, "edge-site-id"));
    add_str(&mut query, "end", &end);
    add_str(&mut query, "importStatus", &value(matches, "import-status"));
    let limit = match parse_i64(matches, "limit", 2000) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let offset = match parse_i64(matches, "offset", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "limit", limit, limit != 0);
    add(&mut query, "offset", offset, offset != 0);
    add_str(&mut query, "path", &value(matches, "path"));
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &value(matches, "session-key"));
    add_str(&mut query, "site.id", &value(matches, "site-id"));
    add_str(&mut query, "sortBy", &value(matches, "sort-by"));
    add_str(&mut query, "sortOrder", &value(matches, "sort-order"));
    add_str(&mut query, "start", &start);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list recordings",
        move |client| async move {
            client
                .get::<_, Vec<Recording>>("/v1/recordings", &query)
                .await
        },
    )
}

pub(crate) fn delete_recording(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let id = crate::read_helpers::positional(matches, 0);
    match crate::read_helpers::block_on(runtime.client.delete(&format!("/v1/recordings/{id}"))) {
        Ok(()) => Outcome::default(),
        Err(error) if error.is_not_found() => Outcome {
            stderr: b"Not found. The resource may have already been deleted.\n".to_vec(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to delete recording: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::Recording;

    #[test]
    fn recording_accepts_nullable_api_reference_fields_like_go() {
        let recording: Recording = serde_json::from_str(
            r#"{"id":"rec","path":"fixture.mcap","size":1,"messageCount":0,"createdAt":"2024-01-01T00:00:00Z","importedAt":"2024-01-01T00:00:00Z","start":"2024-01-01T00:00:00Z","end":"2024-01-01T00:00:00Z","importStatus":"complete","site":{"id":"site","name":"Site"},"edgeSite":null,"device":null,"metadata":null,"key":null,"projectId":"prj"}"#,
        )
        .unwrap();
        assert!(recording.edge_site.id.is_empty());
        assert!(recording.device.id.is_empty());
        assert!(recording.key.is_empty());
    }
}
