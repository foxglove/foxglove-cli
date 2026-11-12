use super::*;

const AUTOMATION: &str = r#"{"id":"auto_1","projectId":"prj_1","name":"Triage","actionPrompt":"Summarize new recordings","isOwner":true,"permissions":{"canEdit":true,"canRun":true,"canDelete":true},"createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z"}"#;
const TRIGGER: &str = r#"{"id":"trg_1","automationId":"auto_1","eventType":"clock.updated","enabled":false,"configurationVersion":1,"configuration":{"schedule":{"cadence":"daily","anchor":"2026-01-05T09:00:00Z"}},"createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z","deletedAt":null}"#;
const RUN: &str = r#"{"id":"run_1","automationId":"auto_1","actionPrompt":"Summarize new recordings","runPrompt":"Check rec_1","status":"pending","createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z","startedAt":null,"completedAt":null,"errorMessage":null}"#;

fn created(method: &'static str, path: &'static str, body: &str) -> Reply {
    Reply {
        status: 201,
        ..Reply::json(method, path, body)
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn automations_are_listed_with_their_project_and_page() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/automations",
        &format!(
            r#"{{"automations":[{},"lifetimeRunCount":2,"triggerEnabled":null,"lastRunAt":"2026-01-03T00:00:00Z","lastRunStatus":"succeeded"}}]}}"#,
            AUTOMATION.strip_suffix('}').unwrap()
        ),
    )]);
    let output = Process::spawn(
        workspace
            .command(&server.url)
            .env("FOXGLOVE_DEFAULT_PROJECT_ID", "prj_1")
            .args([
                "automations",
                "list",
                "--limit",
                "1",
                "--offset",
                "3",
                "--format",
                "csv",
            ]),
    )
    .finish();
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(
        query_pairs(&requests[0]),
        expected_pairs(&[("projectId", "prj_1"), ("limit", "1"), ("offset", "3")])
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Name,Project ID,Trigger Enabled,Lifetime Run Count,Last Run At,Last Run Status,Created At,Updated At\n\
         auto_1,Triage,prj_1,,2,2026-01-03T00:00:00Z,succeeded,2026-01-02T03:04:05Z,2026-01-02T03:04:05Z\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Showing the first 1 results. More may exist; use --offset to page through them.\n"
    );

    let output = Process::spawn(
        workspace
            .command("http://127.0.0.1:1")
            .args(["automations", "list"]),
    )
    .finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "--project-id is required when listing automations\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn automations_are_created_edited_and_deleted() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        // A create needs only the new ID; other fields may be missing or null.
        created("POST", "/v1/automations", r#"{"id":"auto_1","name":null}"#),
        Reply::json("PATCH", "/v1/automations/auto%2F1", AUTOMATION),
        Reply::json("DELETE", "/v1/automations/auto_1", r#"{"success":true}"#),
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "automations",
            "add",
            "--project-id",
            "prj_1",
            "--name",
            "Triage",
            "--action-prompt",
            "Summarize new recordings",
        ],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation created: auto_1\n"
    );
    let output = run(
        &workspace,
        &server,
        &["automations", "edit", "auto/1", "--name", "Renamed"],
    );
    assert_success(&output);
    let output = run(&workspace, &server, &["automations", "delete", "auto_1"]);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation deleted: auto_1\n"
    );
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[0]),
        serde_json::json!({"projectId": "prj_1", "name": "Triage", "actionPrompt": "Summarize new recordings"})
    );
    assert_eq!(
        json_body(&requests[1]),
        serde_json::json!({"name": "Renamed"})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn triggers_are_replaced_with_put_and_read_back() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        created("PUT", "/v1/automations/auto_1/trigger", TRIGGER),
        Reply::json("GET", "/v1/automations/auto_1/trigger", TRIGGER),
        Reply {
            status: 404,
            ..Reply::json(
                "GET",
                "/v1/automations/auto_2/trigger",
                r#"{"error":"Not found"}"#,
            )
        },
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "automations",
            "trigger",
            "set",
            "auto_1",
            "--event-type",
            "clock.updated",
            "--enabled=false",
            "--schedule-cadence",
            "daily",
            "--schedule-anchor",
            "2026-01-05T09:00",
        ],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Trigger set for automation auto_1: clock.updated (disabled)\n"
    );
    let output = run(
        &workspace,
        &server,
        &[
            "automations",
            "trigger",
            "get",
            "auto_1",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::from_str::<serde_json::Value>(TRIGGER).unwrap()
    );
    let output = run(
        &workspace,
        &server,
        &["automations", "trigger", "get", "auto_2"],
    );
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation auto_2 not found, or it has no trigger\n"
    );
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[0]),
        serde_json::json!({
            "eventType": "clock.updated",
            "enabled": false,
            "configuration": {"schedule": {"cadence": "daily", "anchor": "2026-01-05T09:00:00Z"}},
        })
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn runs_are_started_and_listed_with_their_filters() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        created("POST", "/v1/automations/runs", RUN),
        // A create needs only the new ID; other fields may be missing or null.
        created(
            "POST",
            "/v1/automations/runs",
            r#"{"id":"run_2","runPrompt":null}"#,
        ),
        Reply::json("GET", "/v1/automations/runs", r#"{"runs":[]}"#),
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "automations",
            "runs",
            "add",
            "auto_1",
            "--run-prompt",
            "Check rec_1",
        ],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation run created: run_1\nRun foxglove automations runs get run_1 to follow its progress.\n"
    );
    let output = run(
        &workspace,
        &server,
        &["automations", "runs", "add", "auto_1"],
    );
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("Automation run created: run_2\n"));
    let output = run(
        &workspace,
        &server,
        &[
            "automations",
            "runs",
            "list",
            "--project-id",
            "prj_1",
            "--automation-id",
            "auto_1,auto_2",
            "--automation-id",
            "auto_3",
            "--status",
            "failed",
            "--created-after",
            "2026-01-02",
            "--created-before",
            "2026-01-03T12:00:00Z",
            "--sort-by",
            "createdAt",
            "--sort-order",
            "asc",
            "--owner-member-id",
            "me",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "{\"data\":[]}\n");
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[0]),
        serde_json::json!({"automationId": "auto_1", "runPrompt": "Check rec_1"})
    );
    assert_eq!(
        json_body(&requests[1]),
        serde_json::json!({"automationId": "auto_1"})
    );
    assert_eq!(
        query_pairs(&requests[2]),
        expected_pairs(&[
            ("projectId", "prj_1"),
            ("automationId", "auto_1,auto_2,auto_3"),
            ("status", "failed"),
            ("createdAfter", "2026-01-02T00:00:00Z"),
            ("createdBefore", "2026-01-03T12:00:00Z"),
            ("sortBy", "createdAt"),
            ("sortOrder", "asc"),
            ("ownerMemberId", "me"),
            ("limit", "50"),
        ])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn runs_are_deleted_by_id() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json(
            "DELETE",
            "/v1/automations/runs/run%2F1",
            r#"{"success":true}"#,
        ),
        Reply {
            status: 404,
            ..Reply::json(
                "DELETE",
                "/v1/automations/runs/run_2",
                r#"{"error":"Not found"}"#,
            )
        },
    ]);
    let output = run(
        &workspace,
        &server,
        &["automations", "runs", "delete", "run/1"],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation run deleted: run/1\n"
    );
    let output = run(
        &workspace,
        &server,
        &["automations", "runs", "delete", "run_2"],
    );
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Automation run not found: run_2\n"
    );
    server.finish();
}
