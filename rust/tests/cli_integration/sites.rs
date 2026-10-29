use super::*;

const EDGE_SITE: &str =
    r#"{"id":"site_one","name":"Garage","type":"edge","retainRecordingsSeconds":604800}"#;
const POLICY: &str = r#"{"revision":4,"createdAt":"2026-01-02T03:04:05Z","policy":{"schemaVersion":1,"rules":[{"pattern":"^tmp/","action":{"type":"ignore"}}]}}"#;

#[test]
#[ignore = "requires loopback sockets"]
fn sites_are_listed_and_fetched() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/sites",
        &format!(
            r#"[{EDGE_SITE},{{"id":"site_two","name":"Lab","type":"self-hosted","url":"https://lab.example.com"}}]"#
        ),
    )]);
    let output = run(&workspace, &server, &["sites", "list", "--format", "csv"]);
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Name,Type,URL,Retain Recordings Seconds\n\
         site_one,Garage,edge,,604800\n\
         site_two,Lab,self-hosted,https://lab.example.com,\n"
    );

    let server = Server::new(vec![Reply::json("GET", "/v1/sites/site%2Fone", EDGE_SITE)]);
    let output = run(
        &workspace,
        &server,
        &["sites", "get", "site/one", "--format", "json"],
    );
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{EDGE_SITE}\n")
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_editing_and_deleting_a_site_send_only_the_fields_given() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json("POST", "/v1/sites", EDGE_SITE)]);
    let output = run(
        &workspace,
        &server,
        &[
            "sites",
            "add",
            "--name",
            "Garage",
            "--type",
            "edge",
            "--retain-recordings-seconds",
            "604800",
        ],
    );
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({"name": "Garage", "type": "edge", "retainRecordingsSeconds": 604_800})
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site created: site_one\n"
    );

    for (flags, expected) in [
        (vec!["--name", "Shed"], serde_json::json!({"name": "Shed"})),
        (
            vec!["--retain-recordings-seconds", "0"],
            serde_json::json!({"retainRecordingsSeconds": 0}),
        ),
        (
            vec!["--url", "https://lab.example.com"],
            serde_json::json!({"url": "https://lab.example.com"}),
        ),
    ] {
        let server = Server::new(vec![Reply::json("PATCH", "/v1/sites/site_one", EDGE_SITE)]);
        let mut args = vec!["sites", "edit", "site_one"];
        args.extend(&flags);
        let output = run(&workspace, &server, &args);
        assert_success(&output);
        assert_eq!(json_body(&server.finish()[0]), expected, "{flags:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Site updated: site_one\n"
        );
    }

    let server = Server::new(vec![Reply::json(
        "DELETE",
        "/v1/sites/site_one",
        r#"{"id":"site_one"}"#,
    )]);
    let output = run(&workspace, &server, &["sites", "delete", "site_one"]);
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site deleted: site_one\n"
    );

    let output = Process::spawn(
        workspace
            .command("http://127.0.0.1:1")
            .args(["sites", "edit", "site_one"]),
    )
    .finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Nothing to update\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn ingest_path_policies_are_printed_and_saved_from_a_file() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("GET", "/v1/sites/site_one/ingest-path-policy", POLICY),
        Reply {
            status: 204,
            ..Reply::json("GET", "/v1/sites/site_two/ingest-path-policy", "")
        },
    ]);
    let output = run(
        &workspace,
        &server,
        &["sites", "ingest-path-policy", "get", "site_one"],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::from_str::<serde_json::Value>(POLICY).unwrap()
    );
    let output = run(
        &workspace,
        &server,
        &["sites", "ingest-path-policy", "get", "site_two"],
    );
    assert_success(&output);
    server.finish();
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site site_two has no saved ingest path policy\n"
    );

    let file = workspace.0.join("policy.json");
    fs::write(&file, r#"{"schemaVersion":1,"rules":[]}"#).unwrap();
    let file = file.to_str().unwrap();
    let server = Server::new(vec![
        Reply::json("POST", "/v1/sites/site_one/ingest-path-policy", POLICY),
        Reply {
            status: 409,
            ..Reply::json(
                "POST",
                "/v1/sites/site_one/ingest-path-policy",
                r#"{"error":"StalePolicyRevision"}"#,
            )
        },
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "sites",
            "ingest-path-policy",
            "set",
            "site_one",
            file,
            "--previous-revision",
            "3",
        ],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Ingest path policy for site site_one is at revision 4\n"
    );
    let output = run(
        &workspace,
        &server,
        &["sites", "ingest-path-policy", "set", "site_one", file],
    );
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Failed to save ingest path policy: StalePolicyRevision\n"
    );
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[0]),
        serde_json::json!({"schemaVersion": 1, "rules": [], "previousRevision": 3})
    );
    assert_eq!(
        json_body(&requests[1]),
        serde_json::json!({"schemaVersion": 1, "rules": []})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn missing_sites_are_named_in_the_error() {
    let workspace = Workspace::new();
    for args in [
        vec!["sites", "get", "site_gone"],
        vec!["sites", "delete", "site_gone"],
        vec!["sites", "edit", "site_gone", "--name", "Shed"],
        vec!["sites", "ingest-path-policy", "get", "site_gone"],
    ] {
        let (method, path) = match args[1] {
            "get" => ("GET", "/v1/sites/site_gone"),
            "delete" => ("DELETE", "/v1/sites/site_gone"),
            "edit" => ("PATCH", "/v1/sites/site_gone"),
            _ => ("GET", "/v1/sites/site_gone/ingest-path-policy"),
        };
        let server = Server::new(vec![Reply {
            status: 404,
            ..Reply::json(method, path, r#"{"error":"Site not found"}"#)
        }]);
        let output = run(&workspace, &server, &args);
        server.finish();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Site not found: site_gone\n",
            "{args:?}"
        );
    }
}
