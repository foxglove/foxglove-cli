use super::*;

const PROPERTY: &str = r#"{"id":"cp_one","key":"weather","label":"Weather","resourceType":"device","valueType":"enum","enumValues":["rain","sun"],"values":["rain","sun"],"hasAssociatedData":true}"#;

#[test]
#[ignore = "requires loopback sockets"]
fn custom_properties_are_listed_by_resource_type() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("GET", "/v1/custom-properties", &format!("[{PROPERTY}]")),
        Reply::json(
            "GET",
            "/v1/custom-properties",
            r#"[{"id":"cp_two","key":"note","label":"Note","resourceType":"event","valueType":"string","hasAssociatedData":false}]"#,
        ),
    ]);
    let csv = run(
        &workspace,
        &server,
        &[
            "custom-properties",
            "list",
            "--resource-type",
            "device",
            "--format",
            "csv",
        ],
    );
    let json = run(
        &workspace,
        &server,
        &["custom-properties", "list", "--format", "json"],
    );
    let requests = server.finish();
    assert_success(&csv);
    assert_success(&json);
    assert_eq!(
        query_pairs(&requests[0]),
        expected_pairs(&[("resourceType", "device")])
    );
    assert_eq!(query_pairs(&requests[1]), expected_pairs(&[]));
    assert_eq!(
        String::from_utf8_lossy(&csv.stdout),
        "ID,Key,Label,Resource Type,Value Type,Enum Values,Has Associated Data\n\
         cp_one,weather,Weather,device,enum,\"rain, sun\",true\n"
    );
    let listed: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        listed["data"],
        serde_json::json!([{"id": "cp_two", "key": "note", "label": "Note", "resourceType": "event", "valueType": "string", "hasAssociatedData": false}])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_custom_property_is_fetched_by_its_encoded_id() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/custom-properties/cp%2Fone",
        PROPERTY,
    )]);
    let output = run(
        &workspace,
        &server,
        &["custom-properties", "get", "cp/one", "--format", "json"],
    );
    server.finish();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"id": "cp_one", "key": "weather", "label": "Weather", "resourceType": "device", "valueType": "enum", "enumValues": ["rain", "sun"], "hasAssociatedData": true})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn custom_properties_are_created_edited_and_deleted() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("POST", "/v1/custom-properties", PROPERTY),
        Reply::json(
            "POST",
            "/v1/custom-properties",
            r#"{"id":"cp_two","key":"note","label":"Note","resourceType":"event","valueType":"string","hasAssociatedData":false}"#,
        ),
        Reply::json("PATCH", "/v1/custom-properties/cp_one", PROPERTY),
        Reply::json("PATCH", "/v1/custom-properties/cp_one", PROPERTY),
        Reply::json(
            "DELETE",
            "/v1/custom-properties/cp_one",
            r#"{"id":"cp_one"}"#,
        ),
    ]);
    let commands: [(&[&str], &str); 5] = [
        (
            &[
                "custom-properties",
                "add",
                "--key",
                "weather",
                "--label",
                "Weather",
                "--resource-type",
                "device",
                "--value-type",
                "enum",
                "--enum-value",
                "rain",
                "--enum-value",
                "sun",
            ],
            "Custom property created: cp_one\n",
        ),
        (
            &[
                "custom-properties",
                "add",
                "--key",
                "note",
                "--label",
                "Note",
                "--resource-type",
                "event",
                "--value-type",
                "string",
            ],
            "Custom property created: cp_two\n",
        ),
        (
            &["custom-properties", "edit", "cp_one", "--label", "Sky"],
            "Custom property updated: cp_one\n",
        ),
        (
            &[
                "custom-properties",
                "edit",
                "cp_one",
                "--enum-value",
                "sun",
                "--enum-value",
                "rain",
                "--enum-value",
                "snow",
            ],
            "Custom property updated: cp_one\n",
        ),
        (
            &["custom-properties", "delete", "cp_one"],
            "Custom property deleted: cp_one\n",
        ),
    ];
    for (args, notice) in commands {
        let output = run(&workspace, &server, args);
        assert_success(&output);
        assert_eq!(String::from_utf8_lossy(&output.stderr), notice, "{args:?}");
    }
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[0]),
        serde_json::json!({"key": "weather", "label": "Weather", "resourceType": "device", "valueType": "enum", "enumValues": ["rain", "sun"]})
    );
    assert_eq!(
        json_body(&requests[1]),
        serde_json::json!({"key": "note", "label": "Note", "resourceType": "event", "valueType": "string"})
    );
    assert_eq!(json_body(&requests[2]), serde_json::json!({"label": "Sky"}));
    assert_eq!(
        json_body(&requests[3]),
        serde_json::json!({"enumValues": ["sun", "rain", "snow"]})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn custom_property_changes_are_checked_before_a_request() {
    let workspace = Workspace::new();
    let server = Server::new(Vec::new());
    for (args, error) in [
        (
            vec!["--value-type", "string", "--enum-value", "rain"],
            "--enum-value can only be used with --value-type enum or multi-enum, not string\n",
        ),
        (
            vec!["--value-type", "enum"],
            "--value-type enum requires at least one --enum-value\n",
        ),
    ] {
        let mut command = vec![
            "custom-properties",
            "add",
            "--key",
            "weather",
            "--label",
            "Weather",
            "--resource-type",
            "device",
        ];
        command.extend(args);
        let output = run(&workspace, &server, &command);
        assert!(!output.status.success(), "{command:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            error,
            "{command:?}"
        );
    }
    let output = run(
        &workspace,
        &server,
        &["custom-properties", "edit", "cp_one"],
    );
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Nothing to update\n"
    );
    assert!(server.finish().is_empty());
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_missing_custom_property_is_named_in_the_error() {
    let workspace = Workspace::new();
    for (args, method) in [
        (vec!["custom-properties", "get", "cp_gone"], "GET"),
        (
            vec!["custom-properties", "edit", "cp_gone", "--label", "Sky"],
            "PATCH",
        ),
        (vec!["custom-properties", "delete", "cp_gone"], "DELETE"),
    ] {
        let server = Server::new(vec![Reply {
            status: 404,
            ..Reply::json(
                method,
                "/v1/custom-properties/cp_gone",
                r#"{"error":"Not Found"}"#,
            )
        }]);
        let output = run(&workspace, &server, &args);
        server.finish();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Custom property not found: cp_gone\n",
            "{args:?}"
        );
    }
}
