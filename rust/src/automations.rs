//! Automation commands.

mod trigger;

pub(crate) use trigger::{delete_trigger, get_trigger, set_trigger};

use serde::{Deserialize, Serialize};

use crate::api::{encode_path_segment, ApiError};
use crate::cli::{
    AutomationAddArgs, AutomationEditArgs, AutomationGetArgs, AutomationIdArgs, AutomationListArgs,
};
use crate::output::Format;
use crate::records::{
    format_output, format_record, is_zero, warn_if_truncated, ProjectFallback, Record,
    DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationPermissions {
    can_edit: bool,
    can_run: bool,
    can_delete: bool,
}

/// An automation's saved definition, as the get, create, and edit endpoints
/// return it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Automation {
    id: String,
    project_id: String,
    name: String,
    action_prompt: String,
    is_owner: bool,
    permissions: AutomationPermissions,
    created_at: String,
    updated_at: String,
}

impl Record for Automation {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Project ID",
            "Action Prompt",
            "Is Owner",
            "Can Edit",
            "Can Run",
            "Can Delete",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.project_id.clone(),
            self.action_prompt.clone(),
            self.is_owner.to_string(),
            self.permissions.can_edit.to_string(),
            self.permissions.can_run.to_string(),
            self.permissions.can_delete.to_string(),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

/// An automation as the list endpoint returns it, with a summary of its runs
/// and trigger.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationListItem {
    #[serde(flatten)]
    automation: Automation,
    lifetime_run_count: i64,
    trigger_enabled: Option<bool>,
    last_run_at: Option<String>,
    last_run_status: Option<String>,
}

impl Record for AutomationListItem {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Project ID",
            "Trigger Enabled",
            "Lifetime Run Count",
            "Last Run At",
            "Last Run Status",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.automation.id.clone(),
            self.automation.name.clone(),
            self.automation.project_id.clone(),
            optional_cell(self.trigger_enabled),
            self.lifetime_run_count.to_string(),
            self.last_run_at.clone().unwrap_or_default(),
            self.last_run_status.clone().unwrap_or_default(),
            self.automation.created_at.clone(),
            self.automation.updated_at.clone(),
        ]
    }
}

#[derive(Deserialize)]
struct AutomationListResponse {
    automations: Vec<AutomationListItem>,
}

/// The part of a create response that the create command prints. Decoding only
/// the ID keeps a create from failing after the server accepted it.
#[derive(Debug, Deserialize)]
struct Created {
    id: String,
}

fn optional_cell(value: Option<impl ToString>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn automation_endpoint(id: &str) -> String {
    format!("/v1/automations/{}", encode_path_segment(id))
}

/// The automation endpoints answer 404 only when the automation, or for trigger
/// endpoints its trigger, does not exist.
fn is_not_found(error: &ApiError) -> bool {
    matches!(error, ApiError::NotFound { .. })
}

fn automation_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Automation not found: {id}\n"))
}

/// Resolve a required project ID from the flag or the configured default.
fn required_project_id(
    runtime: &Runtime,
    project_id: Option<&String>,
    action: &str,
) -> Result<String, Outcome> {
    let project_id = project_id.cloned().or_project(&runtime.project_id);
    if project_id.is_empty() {
        return Err(Outcome::failure(format!(
            "--project-id is required when {action}\n"
        )));
    }
    Ok(project_id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationListQuery {
    project_id: String,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
}

pub(crate) async fn list_automations(
    runtime: &Runtime,
    args: &AutomationListArgs,
    format: Format,
) -> Outcome {
    let project_id =
        match required_project_id(runtime, args.project_id.as_ref(), "listing automations") {
            Ok(project_id) => project_id,
            Err(outcome) => return outcome,
        };
    let query = AutomationListQuery {
        project_id,
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        offset: args.offset.unwrap_or_default(),
    };
    match runtime
        .client
        .get::<_, AutomationListResponse>("/v1/automations", &query)
        .await
    {
        Ok(response) => warn_if_truncated(
            format_output(&response.automations, format),
            response.automations.len(),
            query.limit,
        ),
        Err(error) => Outcome::failure(format!("Failed to list automations: {error}\n")),
    }
}

pub(crate) async fn get_automation(
    runtime: &Runtime,
    args: &AutomationGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, Automation>(&automation_endpoint(&args.automation_id), &())
        .await
    {
        Ok(automation) => format_record(&automation, format),
        Err(error) if is_not_found(&error) => automation_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to get automation: {error}\n")),
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAutomationRequest<'a> {
    project_id: String,
    name: &'a str,
    action_prompt: &'a str,
}

pub(crate) async fn add_automation(runtime: &Runtime, args: &AutomationAddArgs) -> Outcome {
    let project_id =
        match required_project_id(runtime, args.project_id.as_ref(), "creating an automation") {
            Ok(project_id) => project_id,
            Err(outcome) => return outcome,
        };
    let request = CreateAutomationRequest {
        project_id,
        name: &args.name,
        action_prompt: &args.action_prompt,
    };
    match runtime
        .client
        .post_created::<_, Created>("/v1/automations", &request)
        .await
    {
        Ok(created) => Outcome::notice(format!("Automation created: {}\n", created.id)),
        Err(error) => Outcome::failure(format!("Failed to create automation: {error}\n")),
    }
}

/// A partial update: omitted fields keep their saved values.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditAutomationRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_prompt: Option<&'a str>,
}

impl<'a> EditAutomationRequest<'a> {
    fn new(args: &'a AutomationEditArgs) -> Option<Self> {
        let request = Self {
            name: args.name.as_deref(),
            action_prompt: args.action_prompt.as_deref(),
        };
        (request.name.is_some() || request.action_prompt.is_some()).then_some(request)
    }
}

pub(crate) async fn edit_automation(runtime: &Runtime, args: &AutomationEditArgs) -> Outcome {
    let Some(request) = EditAutomationRequest::new(args) else {
        return Outcome::failure("Nothing to update\n");
    };
    match runtime
        .client
        .patch::<_, _, Automation>(&automation_endpoint(&args.automation_id), &(), &request)
        .await
    {
        Ok(automation) => Outcome::notice(format!("Automation updated: {}\n", automation.id)),
        Err(error) if is_not_found(&error) => automation_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to edit automation: {error}\n")),
    }
}

pub(crate) async fn delete_automation(runtime: &Runtime, args: &AutomationIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&automation_endpoint(&args.automation_id))
        .await
    {
        Ok(()) => Outcome::notice(format!("Automation deleted: {}\n", args.automation_id)),
        Err(error) if is_not_found(&error) => automation_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to delete automation: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Automation, AutomationEditArgs, AutomationListItem, AutomationListResponse,
        CreateAutomationRequest, EditAutomationRequest,
    };
    use crate::records::Record;

    fn automation_json() -> serde_json::Value {
        json!({
            "id": "auto_1",
            "projectId": "prj_1",
            "name": "Triage",
            "actionPrompt": "Summarize new recordings",
            "isOwner": true,
            "permissions": {"canEdit": true, "canRun": true, "canDelete": false},
            "createdAt": "2026-01-02T03:04:05Z",
            "updatedAt": "2026-01-03T03:04:05Z",
        })
    }

    #[test]
    fn automations_keep_their_public_fields_and_drop_others() {
        let mut wire = automation_json();
        wire["owner"] = json!({"id": "usr_1"});
        let automation: Automation = serde_json::from_value(wire).unwrap();
        assert_eq!(
            serde_json::to_value(&automation).unwrap(),
            automation_json()
        );
        assert_eq!(
            automation.fields(),
            [
                "auto_1",
                "Triage",
                "prj_1",
                "Summarize new recordings",
                "true",
                "true",
                "true",
                "false",
                "2026-01-02T03:04:05Z",
                "2026-01-03T03:04:05Z",
            ]
        );
    }

    #[test]
    fn listed_automations_keep_null_run_summaries_and_unknown_statuses() {
        let mut never_run = automation_json();
        never_run["lifetimeRunCount"] = json!(0);
        never_run["triggerEnabled"] = json!(null);
        never_run["lastRunAt"] = json!(null);
        never_run["lastRunStatus"] = json!(null);
        let mut ran = automation_json();
        ran["lifetimeRunCount"] = json!(3);
        ran["triggerEnabled"] = json!(false);
        ran["lastRunAt"] = json!("2026-01-04T00:00:00Z");
        ran["lastRunStatus"] = json!("cancelled");
        let response: AutomationListResponse =
            serde_json::from_value(json!({"automations": [never_run.clone(), ran]})).unwrap();
        let [never_run_item, ran_item]: [AutomationListItem; 2] =
            response.automations.try_into().unwrap();
        assert_eq!(serde_json::to_value(&never_run_item).unwrap(), never_run);
        assert_eq!(
            never_run_item.fields()[3..7],
            ["", "0", "", ""].map(str::to_owned)
        );
        assert_eq!(
            ran_item.fields()[3..7],
            ["false", "3", "2026-01-04T00:00:00Z", "cancelled"].map(str::to_owned)
        );
    }

    #[test]
    fn create_and_edit_requests_send_only_given_fields() {
        let request = CreateAutomationRequest {
            project_id: "prj_1".to_owned(),
            name: "Triage",
            action_prompt: "Summarize",
        };
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({"projectId": "prj_1", "name": "Triage", "actionPrompt": "Summarize"})
        );

        let edit = |name: Option<&str>, action_prompt: Option<&str>| AutomationEditArgs {
            automation_id: "auto_1".to_owned(),
            name: name.map(str::to_owned),
            action_prompt: action_prompt.map(str::to_owned),
        };
        assert_eq!(EditAutomationRequest::new(&edit(None, None)), None);
        for (args, expected) in [
            (edit(Some("Renamed"), None), json!({"name": "Renamed"})),
            (
                edit(None, Some("New prompt")),
                json!({"actionPrompt": "New prompt"}),
            ),
        ] {
            let request = EditAutomationRequest::new(&args).unwrap();
            assert_eq!(serde_json::to_value(&request).unwrap(), expected);
        }
    }
}
