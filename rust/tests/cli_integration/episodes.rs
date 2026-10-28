use super::*;

const EPISODE: &str = r#"{"id":"ep_one","version":3,"latestVersion":4,"projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{"run":7},"createdAt":"2024-01-02T03:04:07Z"}"#;
const PROPERTIES: &str = r#"[{"key":"note","valueType":"string"},{"key":"speed","valueType":"number"},{"key":"accepted","valueType":"boolean"},{"key":"weather","valueType":"enum","enumValues":["rain","sun"]},{"key":"tags","valueType":"multi-enum","enumValues":["outdoor","night"]}]"#;

fn episode(properties: Option<serde_json::Value>) -> serde_json::Value {
    let mut episode: serde_json::Value = serde_json::from_str(EPISODE).unwrap();
    if let Some(properties) = properties {
        episode["properties"] = properties;
    }
    episode
}

fn member(id: &str, properties: Option<serde_json::Value>) -> String {
    let mut member: serde_json::Value = serde_json::from_str(&dataset_episode(id, false)).unwrap();
    if let Some(properties) = properties {
        member["episode"]["properties"] = properties;
    }
    member.to_string()
}

#[test]
#[ignore = "requires loopback sockets"]
fn reads_request_properties_only_when_selected() {
    for (command, endpoint, nested) in [
        (
            vec!["episodes", "get", "ep_one"],
            "/v1/episodes/ep_one",
            false,
        ),
        (vec!["episodes", "list"], "/v1/episodes", false),
        (
            vec!["datasets", "episodes", "list", "ds_one"],
            "/v1/datasets/ds_one/episodes",
            true,
        ),
        (
            vec!["datasets", "episodes", "list", "ds_one", "--version", "4"],
            "/v1/datasets/ds_one/versions/4/episodes",
            true,
        ),
    ] {
        for (flags, include, properties) in [
            (vec![], None, None),
            (
                vec!["--include-recordings", "--include-properties"],
                Some("recordings,properties"),
                Some(serde_json::json!({"weather":"rain", "custom_key":false})),
            ),
        ] {
            let workspace = Workspace::new();
            let current = episode(properties.clone());
            let body = if nested {
                serde_json::json!({"episodes":[{"addedAt":"", "addedInVersion":4, "episode": current}]})
            } else if command[1] == "get" {
                current
            } else {
                serde_json::json!({"episodes":[current]})
            };
            let server = Server::new(vec![Reply::json("GET", endpoint, &body.to_string())]);
            let mut args = command.clone();
            args.extend(flags);
            args.extend(["--format", "json"]);
            let output = run(&workspace, &server, &args);
            assert_success(&output);
            let requests = server.finish();
            assert_eq!(requests.len(), 1);
            assert_eq!(
                query_pairs(&requests[0]).get("include").map(String::as_str),
                include
            );
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let result = if nested {
                &result["data"][0]["episode"]
            } else if command[1] == "get" {
                &result
            } else {
                &result["data"][0]
            };
            assert_eq!(result["version"], 3);
            assert_eq!(result["latestVersion"], 4);
            assert_eq!(result.get("properties"), properties.as_ref());
        }
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn property_filters_do_not_enable_inclusion_and_draft_reads_keep_the_filter() {
    for draft in [false, true] {
        let workspace = Workspace::new();
        let mut replies = Vec::new();
        if draft {
            replies.push(Reply::json(
                "GET",
                "/v1/datasets/ds_one/versions",
                r#"{"versions":[{"versionNumber":5}]}"#,
            ));
        }
        replies.push(Reply::json(
            "GET",
            if draft {
                "/v1/datasets/ds_one/versions/5/episodes"
            } else {
                "/v1/episodes"
            },
            r#"{"episodes":[]}"#,
        ));
        let server = Server::new(replies);
        let mut args = if draft {
            vec![
                "datasets",
                "episodes",
                "list",
                "ds_one",
                "--version",
                "draft",
            ]
        } else {
            vec!["episodes", "list"]
        };
        args.extend([
            "--properties-query",
            "weather:rain note:\"night run\"",
            "--cursor",
            "next",
            "--format",
            "json",
        ]);
        assert_success(&run(&workspace, &server, &args));
        let requests = server.finish();
        let query = query_pairs(requests.last().unwrap());
        assert_eq!(query["propertiesQuery"], "weather:rain note:\"night run\"");
        assert_eq!(query["cursor"], "next");
        assert!(!query.contains_key("include"));
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn historical_reads_preserve_the_selected_version_and_fail_on_missing_version_fields() {
    for missing in [None, Some("version"), Some("latestVersion")] {
        let workspace = Workspace::new();
        let mut response = episode(None);
        if let Some(field) = missing {
            response.as_object_mut().unwrap().remove(field);
        }
        let server = Server::new(vec![Reply::json(
            "GET",
            "/v1/episodes/ep_one",
            &response.to_string(),
        )]);
        let output = run(
            &workspace,
            &server,
            &[
                "episodes",
                "get",
                "ep_one",
                "--version",
                "3",
                "--format",
                "json",
            ],
        );
        assert_eq!(output.status.success(), missing.is_none());
        assert_eq!(
            query_pairs(&server.finish()[0]),
            expected_pairs(&[("version", "3")])
        );
        if missing.is_some() {
            assert!(output.stdout.is_empty());
        }
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn episode_properties_are_looked_up_and_sent_typed_on_creation() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("GET", "/v1/custom-properties", PROPERTIES),
        Reply::json(
            "POST",
            "/v1/episodes",
            r#"{"episodes":[{"id":"ep_one","created":true}]}"#,
        ),
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "episodes",
            "add",
            "--project-id",
            "prj_default",
            "--recording-id",
            "rec_one",
            "-p",
            "speed:3.5",
            "-p",
            "tags:night",
        ],
    );
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(
        query_pairs(&requests[0]),
        expected_pairs(&[("resourceType", "episode")])
    );
    let body = json_body(&requests[1]);
    assert_eq!(
        body["episodes"][0]["recordings"],
        serde_json::json!(["rec_one"])
    );
    assert_eq!(
        body["episodes"][0]["properties"],
        serde_json::json!({"speed":3.5, "tags":["night"]})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn invalid_property_values_never_reach_the_mutation() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/custom-properties",
        PROPERTIES,
    )]);
    let output = run(
        &workspace,
        &server,
        &["episodes", "edit", "ep_one", "-p", "weather:snow"],
    );
    assert!(!output.status.success());
    assert_eq!(server.finish().len(), 1);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Failed to edit episode: invalid enum value: snow\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn edits_preserve_null_removals_typed_values_and_version_preconditions() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("GET", "/v1/custom-properties", PROPERTIES),
        Reply::json("PATCH", "/v1/episodes/ep_one", EPISODE),
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "episodes",
            "edit",
            "ep_one",
            "--applies-to-version",
            "3",
            "-p",
            "accepted:false",
            "-p",
            "tags:night",
            "--remove-property",
            "note",
            "--start",
            "2024-01-02T03:04:05.123456Z",
            "--add-recording-id",
            "rec_new",
            "--remove-recording-id",
            "rec_old",
        ],
    );
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(
        json_body(&requests[1]),
        serde_json::json!({"appliesToVersion":3, "startTime":"2024-01-02T03:04:05.123Z", "properties":{"accepted":false,"tags":["night"],"note":null}, "recordings":{"add":["rec_new"], "remove":["rec_old"]}})
    );
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Episode updated: ep_one (version 3)\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn removal_and_independent_bounds_do_not_need_a_property_lookup() {
    for (flags, body) in [
        (
            vec!["--remove-property", "weather"],
            serde_json::json!({"properties":{"weather":null}}),
        ),
        (
            vec!["--end", "2024-01-03"],
            serde_json::json!({"endTime":"2024-01-03T00:00:00Z"}),
        ),
        (
            vec!["--add-recording-id", "rec_one"],
            serde_json::json!({"recordings":{"add":["rec_one"]}}),
        ),
    ] {
        let workspace = Workspace::new();
        let server = Server::new(vec![Reply::json("PATCH", "/v1/episodes/ep_one", EPISODE)]);
        let mut args = vec!["episodes", "edit", "ep_one"];
        args.extend(flags);
        assert_success(&run(&workspace, &server, &args));
        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert_eq!(json_body(&requests[0]), body);
    }
}

#[test]
fn invalid_edits_fail_before_http() {
    for args in [
        vec!["episodes", "edit", "ep_one"],
        vec!["episodes", "edit", "ep_one", "--applies-to-version", "3"],
        vec![
            "episodes",
            "edit",
            "ep_one",
            "-p",
            "note:value",
            "--remove-property",
            "note",
        ],
        vec![
            "episodes",
            "edit",
            "ep_one",
            "--add-recording-id",
            "rec_one",
            "--remove-recording-id",
            "rec_one",
        ],
        vec!["episodes", "edit", "ep_one", "--start", "invalid"],
    ] {
        let workspace = Workspace::new();
        let output = Process::spawn(workspace.command("http://127.0.0.1:1").args(&args)).finish();
        assert!(!output.status.success(), "{args:?}");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("request to"),
            "{args:?}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn edit_failures_name_the_episode_or_report_the_api_error() {
    for (status, body, expected) in [
        (
            403,
            r#"{"error":"Episode annotations are not enabled"}"#,
            "Failed to edit episode: Episode annotations are not enabled\n",
        ),
        (
            404,
            r#"{"error":"One or more episodes were not found","ids":["ep_one"]}"#,
            "Episode not found: ep_one\n",
        ),
        (
            404,
            r#"{"error":"One or more recordings not found"}"#,
            "Failed to edit episode: One or more recordings not found\n",
        ),
        (
            409,
            r#"{"error":"One or more episode versions have changed","conflicts":[{"id":"ep_one","currentVersion":4}]}"#,
            "Failed to edit episode: One or more episode versions have changed\n\
             Run foxglove episodes get ep_one to see its latest version.\n",
        ),
    ] {
        let workspace = Workspace::new();
        let server = Server::new(vec![Reply {
            status,
            ..Reply::json("PATCH", "/v1/episodes/ep_one", body)
        }]);
        let output = run(
            &workspace,
            &server,
            &[
                "episodes",
                "edit",
                "ep_one",
                "--remove-property",
                "note",
                "--applies-to-version",
                "3",
            ],
        );
        server.finish();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn comparisons_preserve_properties_on_both_episode_versions() {
    let current = episode(Some(serde_json::json!({"weather":"rain"})));
    let mut previous = episode(Some(serde_json::json!({})));
    previous["version"] = 2.into();
    let body = serde_json::json!({"changes":[{"change":"changed", "addedAt":"", "addedInVersion":1, "episode":current, "previousVersion":2, "previousEpisode":previous}], "addedCount":0, "removedCount":0, "changedCount":1, "nextCursor":"next"});
    for format in ["json", "csv", "table"] {
        let workspace = Workspace::new();
        let server = Server::new(vec![Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/4/compare",
            &body.to_string(),
        )]);
        let output = run(
            &workspace,
            &server,
            &[
                "datasets",
                "versions",
                "compare",
                "ds_one",
                "1",
                "4",
                "--include-recordings",
                "--include-properties",
                "--format",
                format,
            ],
        );
        assert_success(&output);
        assert_eq!(
            query_pairs(&server.finish()[0])["include"],
            "recordings,properties"
        );
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("0 episodes added, 0 removed, 1 changed"));
        if format == "json" {
            let output: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                output["data"][0]["episode"]["properties"],
                serde_json::json!({"weather":"rain"})
            );
            assert_eq!(
                output["data"][0]["previousEpisode"]["properties"],
                serde_json::json!({})
            );
            assert_eq!(output["data"][0]["previousVersion"], 2);
            assert_eq!(output["nextCursor"], "next");
        } else if format == "csv" {
            let mut reader = csv::Reader::from_reader(output.stdout.as_slice());
            let headers = reader.headers().unwrap().clone();
            let row = reader.records().next().unwrap().unwrap();
            let values: BTreeMap<_, _> = headers.iter().zip(row.iter()).collect();
            assert_eq!(values["Version"], "3");
            assert_eq!(values["Previous Version"], "2");
            assert_eq!(values["Custom Properties"], r#"{"weather":"rain"}"#);
            assert_eq!(values["Previous Custom Properties"], "{}");
        } else {
            assert!(String::from_utf8_lossy(&output.stdout).contains("Previous Custom Properties"));
        }
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn download_properties_can_be_toggled_without_downloading_mcap_again() {
    let workspace = Workspace::new();
    for (iteration, flags, properties) in [
        (0, vec![], None),
        (
            1,
            vec!["--include-properties"],
            Some(serde_json::json!({"weather":"rain"})),
        ),
        (2, vec!["--include-properties=false"], None),
        (
            3,
            vec![],
            Some(serde_json::json!({"weather":"unsolicited"})),
        ),
    ] {
        let requested = flags == ["--include-properties"];
        let mut replies = dataset_replies(&[member("ep_one", properties.clone())]);
        if iteration == 0 {
            replies.extend(export_replies(episode_mcap()));
        }
        let server = Server::new(replies);
        assert_success(&download_dataset(&workspace, &server, &flags));
        let requests = server.finish();
        assert_eq!(requests.len(), if iteration == 0 { 5 } else { 3 });
        assert_eq!(
            query_pairs(&requests[2]).get("include").map(String::as_str),
            requested.then_some("properties")
        );
        let manifest = download_manifest(&workspace);
        assert_eq!(
            manifest["episodes"][0].get("properties"),
            if requested { properties.as_ref() } else { None }
        );
        assert_eq!(manifest["formatVersion"], 1);
        assert!(manifest["selection"].get("includeProperties").is_none());
        assert_eq!(
            fs::read(
                workspace
                    .0
                    .join("Highway-merges-v4/episode_0000_ep_one.mcap")
            )
            .unwrap(),
            episode_mcap()
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn forbidden_property_downloads_leave_prior_output_intact() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[member("ep_one", None)]);
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    assert_success(&download_dataset(&workspace, &server, &[]));
    server.finish();
    let root = workspace.0.join("Highway-merges-v4");
    let before = fs::read(root.join("manifest.json")).unwrap();
    let mut replies = dataset_replies(&[]);
    *replies.last_mut().unwrap() = Reply {
        status: 403,
        ..Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/4/episodes",
            r#"{"error":"Episode annotations are not enabled"}"#,
        )
    };
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &["--include-properties"]);
    assert!(!output.status.success());
    assert_eq!(server.finish().len(), 3);
    assert_eq!(fs::read(root.join("manifest.json")).unwrap(), before);
    assert_eq!(
        fs::read(root.join("episode_0000_ep_one.mcap")).unwrap(),
        episode_mcap()
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn property_downloads_page_and_preserve_empty_absent_and_skipped_properties() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[member(
        "ep_one",
        Some(serde_json::json!({"weather":"rain"})),
    )]);
    replies
        .last_mut()
        .unwrap()
        .headers
        .push(("fg-pagination-next-cursor", "next"));
    replies.push(Reply::json("GET", "/v1/datasets/ds_one/versions/4/episodes", &serde_json::json!({"episodes":[serde_json::from_str::<serde_json::Value>(&member("ep_two", Some(serde_json::json!({})))).unwrap(), serde_json::from_str::<serde_json::Value>(&member("ep_three", None)).unwrap()]}).to_string()));
    replies.extend(export_replies(episode_mcap()));
    replies.push(no_streamable_recordings());
    replies.push(Reply {
        status: 403,
        ..Reply::json("POST", "/v1/data/stream", r#"{"error":"forbidden"}"#)
    });
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &["--include-properties"]);
    assert!(!output.status.success());
    let requests = server.finish();
    assert_eq!(query_pairs(&requests[2])["include"], "properties");
    assert_eq!(query_pairs(&requests[3])["include"], "properties");
    assert_eq!(query_pairs(&requests[3])["cursor"], "next");
    let manifest = download_manifest(&workspace);
    assert_eq!(
        manifest["episodes"][0]["properties"],
        serde_json::json!({"weather":"rain"})
    );
    assert_eq!(manifest["episodes"][1]["properties"], serde_json::json!({}));
    assert_eq!(manifest["episodes"][1]["status"], "skipped");
    assert!(manifest["episodes"][2].get("properties").is_none());
    assert_eq!(manifest["episodes"][2]["status"], "failed");
}
