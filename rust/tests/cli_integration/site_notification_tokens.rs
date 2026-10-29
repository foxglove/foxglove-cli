use super::*;

const ENDPOINT: &str = "/v1/site-inbox-notification-tokens";

#[test]
#[ignore = "requires loopback sockets"]
fn site_notification_tokens_are_listed_created_once_and_deleted() {
    let workspace = Workspace::new();
    let token =
        r#"{"id":"sint_1","orgId":"org_1","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z"}"#;
    let server = Server::new(vec![Reply::json("GET", ENDPOINT, &format!("[{token}]"))]);
    let output = run(
        &workspace,
        &server,
        &[
            "site-notification-tokens",
            "list",
            "--site-id",
            "site_1",
            "--format",
            "csv",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("siteId", "site_1")])
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Site ID,Created At\nsint_1,site_1,2026-01-02T03:04:05Z\n"
    );

    let created = r#"{"id":"sint_2","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z","token":"fox_snt_fake"}"#;
    let server = Server::new(vec![Reply::json("POST", ENDPOINT, created)]);
    let output = run(
        &workspace,
        &server,
        &[
            "site-notification-tokens",
            "add",
            "--site-id",
            " site_1 ",
            "--format",
            "csv",
        ],
    );
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({"siteId": "site_1"})
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Site ID,Created At,Token\nsint_2,site_1,2026-01-02T03:04:05Z,fox_snt_fake\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site notification token created. Save the token now; it is not shown again.\n"
    );

    let server = Server::new(vec![Reply::json(
        "DELETE",
        "/v1/site-inbox-notification-tokens/sint_2",
        r#"{"id":"sint_2"}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &["site-notification-tokens", "delete", "sint_2"],
    );
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site notification token deleted\n"
    );

    let server = Server::new(vec![]);
    let output = run(
        &workspace,
        &server,
        &["site-notification-tokens", "add", "--site-id", " "],
    );
    assert!(!output.status.success());
    server.finish();
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_site_notification_token_without_a_value_fails_with_its_id() {
    let workspace = Workspace::new();
    for created in [
        r#"{"id":"sint_3","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z"}"#,
        r#"{"id":"sint_3","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z","token":null}"#,
    ] {
        let server = Server::new(vec![Reply::json("POST", ENDPOINT, created)]);
        let output = run(
            &workspace,
            &server,
            &["site-notification-tokens", "add", "--site-id", "site_1"],
        );
        server.finish();
        assert!(!output.status.success(), "{created}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "", "{created}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Site notification token sint_3 was created, but the response did not include its value. Delete it with `foxglove site-notification-tokens delete sint_3` and try again.\n",
            "{created}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_missing_site_notification_token_is_named() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply {
        status: 404,
        ..Reply::json(
            "DELETE",
            "/v1/site-inbox-notification-tokens/one",
            r#"{"error":"Not Found"}"#,
        )
    }]);
    let output = run(
        &workspace,
        &server,
        &["site-notification-tokens", "delete", "one"],
    );
    server.finish();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Site notification token not found: one\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn site_notification_token_identifier_paths_are_escaped() {
    const KEY: &str = "drive#1?/\\% snow☃";
    let cases: &[(&[&str], &str, &str, &str)] = &[(
        &["site-notification-tokens", "delete", KEY],
        "DELETE",
        "/v1/site-inbox-notification-tokens/drive%231%3F%2F%5C%25%20snow%E2%98%83",
        "{}",
    )];
    assert_each_request_reaches(cases);
}
