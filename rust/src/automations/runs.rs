//! Automation run commands.

use serde::{Deserialize, Serialize};

use super::trigger::{AutomationSchedule, AutomationTrigger};
use super::{automation_not_found, is_not_found, required_project_id, Created};
use crate::api::encode_path_segment;
use crate::cli::{AutomationRunAddArgs, AutomationRunGetArgs, AutomationRunListArgs};
use crate::output::Format;
use crate::records::{
    format_output, format_record, is_zero, parse_timestamp, warn_if_truncated, Record,
    DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

/// A run as the create and get endpoints return it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationRun {
    id: String,
    automation_id: String,
    action_prompt: String,
    run_prompt: String,
    status: String,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
}

/// The recording, event, session, or device whose event started a run.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TriggerSource {
    kind: String,
    id: String,
    project_id: String,
    available: bool,
}

impl TriggerSource {
    fn summary(&self) -> String {
        let mut summary = format!("{} {}", self.kind, self.id);
        if !self.available {
            summary.push_str(" (unavailable)");
        }
        summary
    }
}

/// Token counts in disjoint buckets; their sum is the run's total.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct TokenUsage {
    #[serde(rename = "inputTokens")]
    input: u64,
    #[serde(rename = "outputTokens")]
    output: u64,
    #[serde(rename = "cacheReadTokens")]
    cache_read: u64,
    #[serde(rename = "cacheWriteTokens")]
    cache_write: u64,
}

impl TokenUsage {
    fn total(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
    }
}

fn total_tokens(usage: Option<&TokenUsage>) -> String {
    usage
        .map(|usage| usage.total().to_string())
        .unwrap_or_default()
}

fn source_cell(source: Option<&TriggerSource>) -> String {
    source.map(TriggerSource::summary).unwrap_or_default()
}

/// A run as the list endpoint returns it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationRunSummary {
    id: String,
    automation_id: String,
    automation_name: String,
    status: String,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
    trigger_event_type: Option<String>,
    trigger_source: Option<TriggerSource>,
    trigger_schedule: Option<AutomationSchedule>,
    token_usage: Option<TokenUsage>,
}

const RUN_HEADERS: [&str; 11] = [
    "ID",
    "Automation ID",
    "Automation Name",
    "Status",
    "Trigger Event Type",
    "Trigger Source",
    "Created At",
    "Started At",
    "Completed At",
    "Total Tokens",
    "Error Message",
];

impl Record for AutomationRunSummary {
    fn headers() -> &'static [&'static str] {
        &RUN_HEADERS
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.automation_id.clone(),
            self.automation_name.clone(),
            self.status.clone(),
            self.trigger_event_type.clone().unwrap_or_default(),
            source_cell(self.trigger_source.as_ref()),
            self.created_at.clone(),
            self.started_at.clone().unwrap_or_default(),
            self.completed_at.clone().unwrap_or_default(),
            total_tokens(self.token_usage.as_ref()),
            self.error_message.clone().unwrap_or_default(),
        ]
    }
}

/// A run with its prompts, final answer, and what started it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutomationRunDetail {
    #[serde(flatten)]
    run: AutomationRun,
    automation_name: String,
    final_answer: Option<String>,
    trigger: Option<AutomationTrigger>,
    trigger_source: Option<TriggerSource>,
    token_usage: Option<TokenUsage>,
}

impl Record for AutomationRunDetail {
    fn headers() -> &'static [&'static str] {
        const HEADERS: [&str; 13] = [
            RUN_HEADERS[0],
            RUN_HEADERS[1],
            RUN_HEADERS[2],
            RUN_HEADERS[3],
            RUN_HEADERS[4],
            RUN_HEADERS[5],
            RUN_HEADERS[6],
            RUN_HEADERS[7],
            RUN_HEADERS[8],
            RUN_HEADERS[9],
            RUN_HEADERS[10],
            "Run Prompt",
            "Final Answer",
        ];
        &HEADERS
    }

    fn fields(&self) -> Vec<String> {
        let run = &self.run;
        vec![
            run.id.clone(),
            run.automation_id.clone(),
            self.automation_name.clone(),
            run.status.clone(),
            self.trigger
                .as_ref()
                .map(|trigger| trigger.event_type.clone())
                .unwrap_or_default(),
            source_cell(self.trigger_source.as_ref()),
            run.created_at.clone(),
            run.started_at.clone().unwrap_or_default(),
            run.completed_at.clone().unwrap_or_default(),
            total_tokens(self.token_usage.as_ref()),
            run.error_message.clone().unwrap_or_default(),
            run.run_prompt.clone(),
            self.final_answer.clone().unwrap_or_default(),
        ]
    }
}

#[derive(Deserialize)]
struct AutomationRunListResponse {
    runs: Vec<AutomationRunSummary>,
}

fn run_endpoint(id: &str) -> String {
    format!("/v1/automations/runs/{}", encode_path_segment(id))
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateRunRequest<'a> {
    automation_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_prompt: Option<&'a str>,
}

pub(crate) async fn add_run(runtime: &Runtime, args: &AutomationRunAddArgs) -> Outcome {
    let request = CreateRunRequest {
        automation_id: &args.automation_id,
        run_prompt: args.run_prompt.as_deref(),
    };
    match runtime
        .client
        .post_created::<_, Created>("/v1/automations/runs", &request)
        .await
    {
        Ok(created) => Outcome::notice(format!(
            "Automation run created: {id}\nRun foxglove automations runs get {id} to follow its progress.\n",
            id = created.id
        )),
        Err(error) if is_not_found(&error) => automation_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to start automation run: {error}\n")),
    }
}

pub(crate) async fn get_run(
    runtime: &Runtime,
    args: &AutomationRunGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, AutomationRunDetail>(&run_endpoint(&args.run_id), &())
        .await
    {
        Ok(run) => format_record(&run, format),
        Err(error) if is_not_found(&error) => {
            Outcome::failure(format!("Automation run not found: {}\n", args.run_id))
        }
        Err(error) => Outcome::failure(format!("Failed to get automation run: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunListQuery<'a> {
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    automation_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    status: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    created_after: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    created_before: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_order: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_member_id: Option<&'a str>,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
}

impl<'a> RunListQuery<'a> {
    fn new(args: &'a AutomationRunListArgs, project_id: String) -> Result<Self, String> {
        Ok(Self {
            project_id,
            automation_id: args.automation_id.join(","),
            status: args.status.join(","),
            created_after: parse_timestamp(
                args.created_after.as_deref().unwrap_or_default(),
                "created-after",
            )?,
            created_before: parse_timestamp(
                args.created_before.as_deref().unwrap_or_default(),
                "created-before",
            )?,
            sort_by: args.sort_by.as_deref(),
            sort_order: args.sort_order.as_deref(),
            owner_member_id: args.owner_member_id.as_deref(),
            limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
            offset: args.offset.unwrap_or_default(),
        })
    }
}

pub(crate) async fn list_runs(
    runtime: &Runtime,
    args: &AutomationRunListArgs,
    format: Format,
) -> Outcome {
    let project_id =
        match required_project_id(runtime, args.project_id.as_ref(), "listing automation runs") {
            Ok(project_id) => project_id,
            Err(outcome) => return outcome,
        };
    let query = match RunListQuery::new(args, project_id) {
        Ok(query) => query,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .get::<_, AutomationRunListResponse>("/v1/automations/runs", &query)
        .await
    {
        Ok(response) => warn_if_truncated(
            format_output(&response.runs, format),
            response.runs.len(),
            query.limit,
        ),
        Err(error) => Outcome::failure(format!("Failed to list automation runs: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AutomationRunDetail, AutomationRunSummary, CreateRunRequest};
    use crate::records::Record;

    #[test]
    fn listed_runs_keep_nullable_fields_and_summarize_their_source_and_usage() {
        let manual = json!({
            "id": "run_1",
            "automationId": "auto_1",
            "automationName": "Triage",
            "status": "pending",
            "createdAt": "2026-01-02T00:00:00Z",
            "updatedAt": "2026-01-02T00:00:00Z",
            "startedAt": null,
            "completedAt": null,
            "errorMessage": null,
            "triggerEventType": null,
            "triggerSource": null,
            "triggerSchedule": null,
            "tokenUsage": null,
        });
        let run: AutomationRunSummary = serde_json::from_value(manual.clone()).unwrap();
        assert_eq!(serde_json::to_value(&run).unwrap(), manual);
        assert_eq!(
            run.fields()[3..],
            ["pending", "", "", "2026-01-02T00:00:00Z", "", "", "", ""]
        );

        let mut triggered = manual;
        triggered["status"] = json!("timed_out");
        triggered["triggerEventType"] = json!("recording.imported");
        triggered["triggerSource"] = json!(
            {"kind": "recording", "id": "rec_1", "projectId": "prj_1", "available": false}
        );
        triggered["tokenUsage"] = json!(
            {"inputTokens": 10, "outputTokens": 5, "cacheReadTokens": 100, "cacheWriteTokens": 1}
        );
        let run: AutomationRunSummary = serde_json::from_value(triggered.clone()).unwrap();
        assert_eq!(serde_json::to_value(&run).unwrap(), triggered);
        let fields = run.fields();
        assert_eq!(
            fields[3..6],
            [
                "timed_out",
                "recording.imported",
                "recording rec_1 (unavailable)"
            ]
        );
        assert_eq!(fields[9], "116");
    }

    #[test]
    fn run_details_include_the_run_and_its_trigger() {
        let detail = json!({
            "id": "run_1",
            "automationId": "auto_1",
            "actionPrompt": "Summarize",
            "runPrompt": "Look at rec_1",
            "status": "succeeded",
            "createdAt": "2026-01-02T00:00:00Z",
            "updatedAt": "2026-01-02T00:05:00Z",
            "startedAt": "2026-01-02T00:00:01Z",
            "completedAt": "2026-01-02T00:05:00Z",
            "errorMessage": null,
            "automationName": "Triage",
            "finalAnswer": "All good",
            "trigger": {
                "id": "trg_1",
                "automationId": "auto_1",
                "eventType": "clock.updated",
                "enabled": true,
                "configurationVersion": 1,
                "configuration": {"schedule": {"cadence": "hourly", "anchor": "2026-01-01T00:00:00Z"}},
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-01T00:00:00Z",
                "deletedAt": "2026-01-03T00:00:00Z",
            },
            "triggerSource": null,
            "tokenUsage": null,
        });
        let run: AutomationRunDetail = serde_json::from_value(detail.clone()).unwrap();
        assert_eq!(serde_json::to_value(&run).unwrap(), detail);
        assert_eq!(AutomationRunDetail::headers().len(), run.fields().len());
        assert_eq!(run.fields()[4], "clock.updated");
        assert_eq!(run.fields()[11..], ["Look at rec_1", "All good"]);
    }

    #[test]
    fn run_requests_omit_an_absent_prompt() {
        for (run_prompt, expected) in [
            (None, json!({"automationId": "auto_1"})),
            (
                Some("Check rec_1"),
                json!({"automationId": "auto_1", "runPrompt": "Check rec_1"}),
            ),
        ] {
            let request = CreateRunRequest {
                automation_id: "auto_1",
                run_prompt,
            };
            assert_eq!(serde_json::to_value(&request).unwrap(), expected);
        }
    }
}
