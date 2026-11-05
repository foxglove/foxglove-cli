use super::*;

const AUTOMATION: &str = r#"{"id":"auto_1","projectId":"prj_1","name":"Triage","actionPrompt":"Summarize new recordings","isOwner":true,"permissions":{"canEdit":true,"canRun":true,"canDelete":true},"createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z"}"#;

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
