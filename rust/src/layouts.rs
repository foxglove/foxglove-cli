//! Layout commands.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{LayoutAddArgs, LayoutEditArgs, LayoutGetArgs, LayoutIdArgs, LayoutListArgs};
use crate::output::Format;
use crate::records::{
    fetch_list, format_record, null_to_default, parse_timestamp, EmptyRequest, Record,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Layout {
    id: String,
    name: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(
        rename = "savedAt",
        skip_serializing_if = "String::is_empty",
        default,
        deserialize_with = "null_to_default"
    )]
    saved_at: String,
    #[serde(
        rename = "folderName",
        skip_serializing_if = "String::is_empty",
        default,
        deserialize_with = "null_to_default"
    )]
    folder_name: String,
    permission: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data: Option<Value>,
}

impl Record for Layout {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Folder",
            "Permission",
            "Created At",
            "Updated At",
            "Saved At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.folder_name.clone(),
            self.permission.clone(),
            self.created_at.clone(),
            self.updated_at.clone(),
            self.saved_at.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LayoutQuery {
    include_data: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    updated_since: String,
}

fn layout_endpoint(id: &str) -> String {
    format!("/v1/layouts/{}", encode_path_segment(id))
}

fn layout_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Layout not found: {id}\n"))
}

/// Read the layout data object that `add` and `edit` send as `data`.
async fn read_layout_data(path: &Path) -> Result<Value, String> {
    let raw = tokio::fs::read(path)
        .await
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let data: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("Invalid layout JSON in {}: {error}", path.display()))?;
    if !data.is_object() {
        return Err(format!(
            "{} must contain a JSON object of layout data",
            path.display()
        ));
    }
    Ok(data)
}

fn check_folder_name(folder_name: Option<&str>) -> Result<(), String> {
    if folder_name.is_some_and(|name| name.contains('/')) {
        return Err("--folder-name cannot contain a forward slash".to_owned());
    }
    Ok(())
}

pub(crate) async fn list_layouts(
    runtime: &Runtime,
    args: &LayoutListArgs,
    format: Format,
) -> Outcome {
    let updated_since = match parse_timestamp(
        args.updated_since.as_deref().unwrap_or_default(),
        "updated since",
    ) {
        Ok(value) => value,
        Err(error) => {
            return Outcome::failure(format!(
                "Failed to parse value of --updated-since: {error}\n"
            ))
        }
    };
    let query = LayoutQuery {
        include_data: args.include_data,
        updated_since,
    };
    fetch_list::<Layout, _>(
        runtime,
        format,
        "Failed to list layouts",
        "/v1/layouts",
        &query,
        None,
    )
    .await
}

pub(crate) async fn get_layout(runtime: &Runtime, args: &LayoutGetArgs, format: Format) -> Outcome {
    let query = LayoutQuery {
        include_data: args.include_data,
        updated_since: String::new(),
    };
    match runtime
        .client
        .get::<_, Layout>(&layout_endpoint(&args.layout_id), &query)
        .await
    {
        Ok(layout) => format_record(&layout, format),
        Err(error) if error.is_not_found_for("layout") => layout_not_found(&args.layout_id),
        Err(error) => Outcome::failure(format!("Failed to get layout: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateLayoutRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    saved_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder_name: Option<String>,
    permission: String,
    data: Value,
}

pub(crate) async fn add_layout(runtime: &Runtime, args: &LayoutAddArgs) -> Outcome {
    if let Err(error) = check_folder_name(args.folder_name.as_deref()) {
        return Outcome::failure(format!("{error}\n"));
    }
    let saved_at = match args
        .saved_at
        .as_deref()
        .map(|raw| parse_timestamp(raw, "saved at"))
        .transpose()
    {
        Ok(value) => value,
        Err(error) => {
            return Outcome::failure(format!("Failed to parse value of --saved-at: {error}\n"))
        }
    };
    let data = match read_layout_data(&args.data).await {
        Ok(data) => data,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let request = CreateLayoutRequest {
        id: args.id.clone(),
        name: args.name.clone(),
        saved_at,
        folder_name: args.folder_name.clone(),
        permission: args.permission.clone(),
        data,
    };
    match runtime
        .client
        .post::<_, Layout>("/v1/layouts", &request)
        .await
    {
        Ok(layout) => Outcome::notice(format!("Layout created: {}\n", layout.id)),
        Err(error) => Outcome::failure(format!("Failed to create layout: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditLayoutRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub(crate) async fn edit_layout(runtime: &Runtime, args: &LayoutEditArgs) -> Outcome {
    if args.name.is_none()
        && args.folder_name.is_none()
        && args.permission.is_none()
        && args.data.is_none()
    {
        return Outcome::failure("Nothing to update\n");
    }
    if let Err(error) = check_folder_name(args.folder_name.as_deref()) {
        return Outcome::failure(format!("{error}\n"));
    }
    let data = match &args.data {
        Some(path) => match read_layout_data(path).await {
            Ok(data) => Some(data),
            Err(error) => return Outcome::failure(format!("{error}\n")),
        },
        None => None,
    };
    let request = EditLayoutRequest {
        name: args.name.clone(),
        folder_name: args.folder_name.clone(),
        permission: args.permission.clone(),
        data,
    };
    match runtime
        .client
        .patch::<_, _, Value>(&layout_endpoint(&args.layout_id), &(), &request)
        .await
    {
        Ok(_) => Outcome::notice(format!("Layout updated: {}\n", args.layout_id)),
        Err(error) if error.is_not_found_for("layout") => layout_not_found(&args.layout_id),
        Err(error) => Outcome::failure(format!("Failed to edit layout: {error}\n")),
    }
}

pub(crate) async fn delete_layout(runtime: &Runtime, args: &LayoutIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&layout_endpoint(&args.layout_id))
        .await
    {
        Ok(()) => Outcome::notice(format!(
            "Layout deleted: {id}\nRestore it with foxglove layouts restore {id}\n",
            id = args.layout_id
        )),
        Err(error) if error.is_not_found_for("layout") => layout_not_found(&args.layout_id),
        Err(error) => Outcome::failure(format!("Failed to delete layout: {error}\n")),
    }
}

pub(crate) async fn restore_layout(runtime: &Runtime, args: &LayoutIdArgs) -> Outcome {
    match runtime
        .client
        .post::<_, Value>(
            &format!("{}/restore", layout_endpoint(&args.layout_id)),
            &EmptyRequest {},
        )
        .await
    {
        Ok(_) => Outcome::notice(format!("Layout restored: {}\n", args.layout_id)),
        Err(error) if error.is_not_found_for("layout") => layout_not_found(&args.layout_id),
        Err(error) => Outcome::failure(format!("Failed to restore layout: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Layout, Record};

    #[test]
    fn layout_data_stays_out_of_table_fields_and_optional_fields_are_omitted() {
        let layout: Layout = serde_json::from_value(serde_json::json!({
            "id": "lay_1",
            "name": "Overview",
            "createdAt": "2024-01-02T03:04:05Z",
            "updatedAt": "2024-01-02T03:04:06Z",
            "permission": "ORG_READ",
        }))
        .unwrap();
        assert_eq!(
            layout.fields(),
            [
                "lay_1",
                "Overview",
                "",
                "ORG_READ",
                "2024-01-02T03:04:05Z",
                "2024-01-02T03:04:06Z",
                ""
            ]
        );
        let json = serde_json::to_value(&layout).unwrap();
        for key in ["savedAt", "folderName", "data"] {
            assert!(json.get(key).is_none(), "{key}: {json}");
        }

        let layout: Layout = serde_json::from_value(serde_json::json!({
            "id": "lay_1",
            "name": "Overview",
            "createdAt": "2024-01-02T03:04:05Z",
            "updatedAt": "2024-01-02T03:04:06Z",
            "savedAt": null,
            "folderName": null,
            "permission": "ORG_READ",
        }))
        .unwrap();
        assert_eq!(layout.fields()[2], "");
        assert_eq!(layout.fields()[6], "");
    }
}
