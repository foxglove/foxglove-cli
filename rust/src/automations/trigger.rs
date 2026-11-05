//! Automation trigger commands.

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::UtcOffset;

use super::{automation_endpoint, is_not_found};
use crate::cli::{AutomationGetArgs, AutomationIdArgs, AutomationTriggerSetArgs};
use crate::output::Format;
use crate::records::{format_record, parse_timestamp_value, Record};
use crate::runtime::Runtime;
use crate::Outcome;

/// A UTC calendar recurrence starting at a whole-hour anchor.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct AutomationSchedule {
    pub(super) cadence: String,
    pub(super) anchor: String,
}

impl AutomationSchedule {
    pub(super) fn summary(&self) -> String {
        format!("{} from {}", self.cadence, self.anchor)
    }
}

/// Settings for a trigger's event type.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TriggerConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schedule: Option<AutomationSchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event_type_id: Option<String>,
}

impl TriggerConfiguration {
    fn is_empty(&self) -> bool {
        self.schedule.is_none() && self.event_type_id.is_none()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AutomationTrigger {
    id: String,
    automation_id: String,
    pub(super) event_type: String,
    enabled: bool,
    configuration_version: i64,
    configuration: TriggerConfiguration,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

impl Record for AutomationTrigger {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Automation ID",
            "Event Type",
            "Enabled",
            "Event Type ID",
            "Schedule",
            "Configuration Version",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.automation_id.clone(),
            self.event_type.clone(),
            self.enabled.to_string(),
            self.configuration.event_type_id.clone().unwrap_or_default(),
            self.configuration
                .schedule
                .as_ref()
                .map(AutomationSchedule::summary)
                .unwrap_or_default(),
            self.configuration_version.to_string(),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

fn trigger_endpoint(automation_id: &str) -> String {
    format!("{}/trigger", automation_endpoint(automation_id))
}

fn trigger_not_found(automation_id: &str) -> Outcome {
    Outcome::failure(format!(
        "Automation {automation_id} not found, or it has no trigger\n"
    ))
}

pub(crate) async fn get_trigger(
    runtime: &Runtime,
    args: &AutomationGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, AutomationTrigger>(&trigger_endpoint(&args.automation_id), &())
        .await
    {
        Ok(trigger) => format_record(&trigger, format),
        Err(error) if is_not_found(&error) => trigger_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to get automation trigger: {error}\n")),
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetTriggerRequest<'a> {
    event_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "TriggerConfiguration::is_empty")]
    configuration: TriggerConfiguration,
}

impl<'a> SetTriggerRequest<'a> {
    fn new(args: &'a AutomationTriggerSetArgs) -> Result<Self, String> {
        let schedule = match (&args.schedule_cadence, &args.schedule_anchor) {
            (Some(cadence), Some(anchor)) => Some(AutomationSchedule {
                cadence: cadence.clone(),
                anchor: utc_anchor(anchor)?,
            }),
            _ => None,
        };
        Ok(Self {
            event_type: &args.event_type,
            enabled: args.enabled,
            configuration: TriggerConfiguration {
                schedule,
                event_type_id: args.event_type_id.clone(),
            },
        })
    }
}

/// Normalize a schedule anchor to the UTC timestamp the API expects.
fn utc_anchor(raw: &str) -> Result<String, String> {
    parse_timestamp_value(raw, "schedule anchor")?
        .ok_or_else(|| "--schedule-anchor cannot be empty".to_owned())?
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format schedule anchor time: {error}"))
}

pub(crate) async fn set_trigger(runtime: &Runtime, args: &AutomationTriggerSetArgs) -> Outcome {
    let request = match SetTriggerRequest::new(args) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .put_created::<_, AutomationTrigger>(&trigger_endpoint(&args.automation_id), &request)
        .await
    {
        Ok(trigger) => Outcome::notice(format!(
            "Trigger set for automation {}: {} ({})\n",
            args.automation_id,
            trigger.event_type,
            if trigger.enabled {
                "enabled"
            } else {
                "disabled"
            }
        )),
        Err(error) if is_not_found(&error) => super::automation_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to set automation trigger: {error}\n")),
    }
}

pub(crate) async fn delete_trigger(runtime: &Runtime, args: &AutomationIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&trigger_endpoint(&args.automation_id))
        .await
    {
        Ok(()) => Outcome::notice(format!(
            "Trigger deleted for automation {}\n",
            args.automation_id
        )),
        Err(error) if is_not_found(&error) => trigger_not_found(&args.automation_id),
        Err(error) => Outcome::failure(format!("Failed to delete automation trigger: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AutomationTrigger, AutomationTriggerSetArgs, SetTriggerRequest};
    use crate::records::Record;

    #[test]
    fn triggers_keep_their_configuration_and_tolerate_new_event_types() {
        let scheduled = json!({
            "id": "trg_1",
            "automationId": "auto_1",
            "eventType": "clock.updated",
            "enabled": true,
            "configurationVersion": 1,
            "configuration": {
                "schedule": {"cadence": "daily", "anchor": "2026-01-02T09:00:00Z"},
            },
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "deletedAt": null,
        });
        let trigger: AutomationTrigger = serde_json::from_value(scheduled.clone()).unwrap();
        assert_eq!(serde_json::to_value(&trigger).unwrap(), scheduled);
        assert_eq!(
            trigger.fields()[4..6],
            ["", "daily from 2026-01-02T09:00:00Z"]
        );

        let mut unconfigured = scheduled;
        unconfigured["eventType"] = json!("device.updated");
        unconfigured["configuration"] = json!({});
        let trigger: AutomationTrigger = serde_json::from_value(unconfigured.clone()).unwrap();
        assert_eq!(serde_json::to_value(&trigger).unwrap(), unconfigured);
        assert_eq!(trigger.fields()[2..6], ["device.updated", "true", "", ""]);
    }

    fn set_args(
        event_type: &str,
        enabled: Option<bool>,
        event_type_id: Option<&str>,
        schedule: Option<(&str, &str)>,
    ) -> AutomationTriggerSetArgs {
        AutomationTriggerSetArgs {
            automation_id: "auto_1".to_owned(),
            enabled,
            event_type: event_type.to_owned(),
            event_type_id: event_type_id.map(str::to_owned),
            schedule_anchor: schedule.map(|(_, anchor)| anchor.to_owned()),
            schedule_cadence: schedule.map(|(cadence, _)| cadence.to_owned()),
        }
    }

    #[test]
    fn set_requests_omit_unset_fields_and_normalize_the_anchor_to_utc() {
        for (args, expected) in [
            (
                set_args("recording.imported", None, None, None),
                json!({"eventType": "recording.imported"}),
            ),
            (
                set_args("event.created", Some(false), Some("evt_type_1"), None),
                json!({
                    "eventType": "event.created",
                    "enabled": false,
                    "configuration": {"eventTypeId": "evt_type_1"},
                }),
            ),
            (
                set_args(
                    "clock.updated",
                    Some(true),
                    None,
                    Some(("weekly", "2026-01-05T10:00+01:00")),
                ),
                json!({
                    "eventType": "clock.updated",
                    "enabled": true,
                    "configuration": {
                        "schedule": {"cadence": "weekly", "anchor": "2026-01-05T09:00:00Z"},
                    },
                }),
            ),
        ] {
            let request = SetTriggerRequest::new(&args).unwrap();
            assert_eq!(serde_json::to_value(&request).unwrap(), expected);
        }
        let error = SetTriggerRequest::new(&set_args(
            "clock.updated",
            None,
            None,
            Some(("daily", "tomorrow")),
        ))
        .unwrap_err();
        assert!(
            error.starts_with("failed to parse schedule anchor time:"),
            "{error}"
        );
    }
}
