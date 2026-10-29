use super::*;

const LAYOUT: &str = r#"{"id":"lay_one","name":"Overview","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","savedAt":"2024-01-02T03:04:04Z","folderName":"Ops","permission":"ORG_WRITE","data":{"layout":"3D!a"}}"#;

#[test]
#[ignore = "requires loopback sockets"]
fn layouts_are_listed_without_their_data_unless_requested() {
    let workspace = Workspace::new();
    let list = format!("[{LAYOUT}]");
    let server = Server::new(vec![Reply::json("GET", "/v1/layouts", &list)]);
    let output = run(&workspace, &server, &["layouts", "list", "--format", "csv"]);
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("includeData", "false")])
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Name,Folder,Permission,Created At,Updated At,Saved At\n\
         lay_one,Overview,Ops,ORG_WRITE,2024-01-02T03:04:05Z,2024-01-02T03:04:06Z,2024-01-02T03:04:04Z\n"
    );

    let server = Server::new(vec![Reply::json("GET", "/v1/layouts", &list)]);
    let output = run(
        &workspace,
        &server,
        &[
            "layouts",
            "list",
            "--include-data",
            "--updated-since",
            "2024-01-01T00:00:00Z",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[
            ("includeData", "true"),
            ("updatedSince", "2024-01-01T00:00:00Z")
        ])
    );
    let output: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        output["data"][0],
        serde_json::from_str::<serde_json::Value>(LAYOUT).unwrap()
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn getting_a_layout_includes_its_data_on_request() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json("GET", "/v1/layouts/lay_one", LAYOUT)]);
    let output = run(
        &workspace,
        &server,
        &[
            "layouts",
            "get",
            "lay_one",
            "--include-data",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("includeData", "true")])
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::from_str::<serde_json::Value>(LAYOUT).unwrap()
    );

    let server = Server::new(vec![Reply {
        status: 404,
        ..Reply::json("GET", "/v1/layouts/lay_gone", r#"{"error":"Not Found"}"#)
    }]);
    let output = run(&workspace, &server, &["layouts", "get", "lay_gone"]);
    server.finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Layout not found: lay_gone\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_a_layout_sends_the_data_file_and_settings() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("layout.json"), r#"{"layout":"3D!a"}"#).unwrap();
    let server = Server::new(vec![Reply::json("POST", "/v1/layouts", LAYOUT)]);
    let output = run(
        &workspace,
        &server,
        &[
            "layouts",
            "add",
            "--name",
            "Overview",
            "--permission",
            "ORG_WRITE",
            "--folder-name",
            "Ops",
            "--id",
            "lay_one",
            "--saved-at",
            "2024-01-02T03:04:04Z",
            "--data",
            "layout.json",
        ],
    );
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({
            "id": "lay_one",
            "name": "Overview",
            "savedAt": "2024-01-02T03:04:04Z",
            "folderName": "Ops",
            "permission": "ORG_WRITE",
            "data": {"layout": "3D!a"},
        })
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Layout created: lay_one\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn invalid_layout_input_is_rejected_before_a_request() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("array.json"), "[]").unwrap();
    fs::write(workspace.0.join("layout.json"), "{}").unwrap();
    for (args, expected) in [
        (
            vec![
                "layouts",
                "add",
                "--name",
                "Overview",
                "--permission",
                "ORG_READ",
                "--data",
                "array.json",
            ],
            "array.json must contain a JSON object of layout data\n",
        ),
        (
            vec![
                "layouts",
                "add",
                "--name",
                "Overview",
                "--permission",
                "ORG_READ",
                "--folder-name",
                "a/b",
                "--data",
                "layout.json",
            ],
            "--folder-name cannot contain a forward slash\n",
        ),
        (vec!["layouts", "edit", "lay_one"], "Nothing to update\n"),
    ] {
        let output = Process::spawn(workspace.command("http://127.0.0.1:1").args(&args)).finish();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected,
            "{args:?}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn editing_a_layout_sends_only_the_fields_given() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("layout.json"), r#"{"layout":"Plot!b"}"#).unwrap();
    for (flags, expected) in [
        (
            vec!["--name", "Renamed", "--permission", "ORG_READ"],
            serde_json::json!({"name": "Renamed", "permission": "ORG_READ"}),
        ),
        (
            vec!["--folder-name", ""],
            serde_json::json!({"folderName": ""}),
        ),
        (
            vec!["--data", "layout.json"],
            serde_json::json!({"data": {"layout": "Plot!b"}}),
        ),
    ] {
        let server = Server::new(vec![Reply::json("PATCH", "/v1/layouts/lay_one", LAYOUT)]);
        let mut args = vec!["layouts", "edit", "lay_one"];
        args.extend(&flags);
        let output = run(&workspace, &server, &args);
        assert_success(&output);
        assert_eq!(json_body(&server.finish()[0]), expected, "{flags:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Layout updated: lay_one\n"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn deleting_a_layout_says_how_to_restore_it() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "DELETE",
        "/v1/layouts/lay_one",
        r#"{"success":true}"#,
    )]);
    let output = run(&workspace, &server, &["layouts", "delete", "lay_one"]);
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Layout deleted: lay_one\nRestore it with foxglove layouts restore lay_one\n"
    );

    let server = Server::new(vec![Reply::json(
        "POST",
        "/v1/layouts/lay_one/restore",
        r#"{"success":true}"#,
    )]);
    let output = run(&workspace, &server, &["layouts", "restore", "lay_one"]);
    assert_success(&output);
    assert_eq!(json_body(&server.finish()[0]), serde_json::json!({}));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Layout restored: lay_one\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn layout_identifier_paths_are_escaped() {
    const KEY: &str = "drive#1?/\\% snow☃";
    const PATH: &str = "/v1/layouts/drive%231%3F%2F%5C%25%20snow%E2%98%83";
    assert_each_request_reaches(&[
        (&["layouts", "get", KEY], "GET", PATH, LAYOUT),
        (
            &["layouts", "edit", KEY, "--name", "renamed"],
            "PATCH",
            PATH,
            LAYOUT,
        ),
        (&["layouts", "delete", KEY], "DELETE", PATH, "{}"),
        (
            &["layouts", "restore", KEY],
            "POST",
            "/v1/layouts/drive%231%3F%2F%5C%25%20snow%E2%98%83/restore",
            "{}",
        ),
    ]);
}
