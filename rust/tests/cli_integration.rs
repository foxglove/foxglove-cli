mod support;

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::process::Output;

use foxglove_rust::format::{Channel, McapWriter, Message, RecordSink, Schema};
use support::{assert_success, Process, Reply, Server, Workspace};

#[test]
#[ignore = "requires loopback sockets"]
fn explicit_empty_project_overrides_configured_default() {
    let workspace = Workspace::new();
    for (flags, expected) in [
        (vec![], Some("prj_default")),
        (vec!["--project-id="], None),
        (vec!["--project-id", ""], None),
        (vec!["--project-id", "prj_explicit"], Some("prj_explicit")),
    ] {
        let server = Server::new(vec![Reply::json("GET", "/v1/recordings", "[]")]);
        let output = Process::spawn(
            workspace
                .command(&server.url)
                .env("DEFAULT_PROJECT_ID", "prj_default")
                .args(["recordings", "list", "--format", "json"])
                .args(&flags),
        )
        .finish();
        assert_success(&output);
        let requests = server.finish();
        let target = requests[0].split_whitespace().nth(1).unwrap();
        let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
        let project = url
            .query_pairs()
            .find(|(name, _)| name == "projectId")
            .map(|(_, value)| value.into_owned());
        assert_eq!(project.as_deref(), expected, "{flags:?}");
    }
}

#[test]
fn clearing_project_default_requires_project_for_session_key() {
    let workspace = Workspace::new();
    let output = Process::spawn(
        workspace
            .command("http://127.0.0.1:1")
            .env("DEFAULT_PROJECT_ID", "prj_default")
            .args([
                "recordings",
                "list",
                "--project-id=",
                "--session-key",
                "session_key",
            ]),
    )
    .finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "--project-id is required when using --session-key\n",
    );
}

fn message(channel_id: u16, time: u64, data: Vec<u8>) -> Message {
    Message {
        channel_id,
        sequence: 0,
        log_time: time,
        publish_time: time,
        data,
    }
}

fn recording(messages: &[Message]) -> Vec<u8> {
    let mut writer = McapWriter::new(Cursor::new(Vec::new())).unwrap();
    writer
        .schema(&Schema {
            id: 1,
            name: "example/Message".into(),
            encoding: "ros1msg".into(),
            data: b"uint8 value\n".to_vec(),
        })
        .unwrap();
    for (id, schema_id, topic, encoding) in [(1, 1, "/typed", "ros1"), (2, 0, "/untyped", "json")] {
        writer
            .channel(&Channel {
                id,
                schema_id,
                topic: topic.into(),
                message_encoding: encoding.into(),
                metadata: BTreeMap::new(),
            })
            .unwrap();
    }
    for message in messages {
        writer.message(message).unwrap();
    }
    writer.finish_into().unwrap().into_inner()
}

fn export_replies(body: Vec<u8>) -> Vec<Reply> {
    vec![
        Reply::json(
            "POST",
            "/v1/data/stream",
            r#"{"link":"{BASE_URL}/download"}"#,
        ),
        Reply {
            body,
            ..Reply::json("GET", "/download", "")
        },
    ]
}

#[test]
#[ignore = "requires loopback sockets"]
fn repeated_event_query_fields_reach_the_api() {
    let workspace = Workspace::new();
    for fields in [
        vec!["metadata"],
        vec!["properties"],
        vec!["metadata", "properties"],
    ] {
        let server = Server::new(vec![Reply::json("GET", "/v1/events", "[]")]);
        let mut command = workspace.command(&server.url);
        command.args([
            "events",
            "list",
            "--format",
            "json",
            "--query",
            "robot & camera",
            "--device-name",
            "Robot A",
            "--limit",
            "3",
            "--offset",
            "1",
        ]);
        for field in &fields {
            command.args(["--query-field", field]);
        }
        let output = Process::spawn(&mut command).finish();
        assert_success(&output);
        assert_eq!(output.stdout, b"[]\n");
        let requests = server.finish();
        let target = requests[0].split_whitespace().nth(1).unwrap();
        let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
        let mut expected = vec![
            ("query", "robot & camera"),
            ("device.name", "Robot A"),
            ("limit", "3"),
            ("offset", "1"),
            ("sortOrder", "asc"),
        ];
        expected.extend(fields.iter().map(|value| ("queryFields", *value)));
        expected.sort_unstable();
        let mut actual: Vec<_> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        actual.sort_unstable();
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn large_mcap_export_preserves_server_bytes() {
    let workspace = Workspace::new();
    let mut data = vec![b' '; 64 * 1024 * 1024];
    data[..11].copy_from_slice(b"{\"value\":1}");
    let payload = recording(&[message(2, 1, data)]);
    let server = Server::new(export_replies(payload.clone()));
    fs::write(workspace.0.join("output.mcap"), b"existing destination").unwrap();
    let output = Process::spawn(workspace.command(&server.url).args([
        "data",
        "export",
        "--recording-id",
        "rec",
        "--output-file",
        "output.mcap",
    ]))
    .finish();
    assert_success(&output);
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(workspace.0.join("output.mcap")).unwrap(), payload);
    assert_eq!(server.finish().len(), 2);
}

#[test]
#[ignore = "requires loopback sockets"]
fn invalid_record_length_preserves_destination() {
    let workspace = Workspace::new();
    let mut payload = mcap::MAGIC.to_vec();
    payload.push(5);
    payload.extend_from_slice(&u64::MAX.to_le_bytes());
    let server = Server::new(export_replies(payload));
    fs::write(workspace.0.join("output.mcap"), b"existing destination").unwrap();
    let output = Process::spawn(workspace.command(&server.url).args([
        "data",
        "export",
        "--recording-id",
        "rec",
        "--output-file",
        "output.mcap",
    ]))
    .finish();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(workspace.0.join("output.mcap")).unwrap(),
        b"existing destination"
    );
    assert_eq!(
        fs::read_dir(&workspace.0).unwrap().count(),
        1,
        "staging was not cleaned"
    );
    assert_eq!(server.finish().len(), 2);
}

#[derive(Default)]
struct Records {
    channels: BTreeMap<u16, Channel>,
    schemas: BTreeMap<u16, Schema>,
    messages: Vec<Message>,
}

impl RecordSink for Records {
    fn schema(&mut self, schema: Schema) -> Result<(), foxglove_rust::format::Error> {
        self.schemas.insert(schema.id, schema);
        Ok(())
    }
    fn channel(&mut self, channel: Channel) -> Result<(), foxglove_rust::format::Error> {
        self.channels.insert(channel.id, channel);
        Ok(())
    }
    fn message(&mut self, message: Message) -> Result<(), foxglove_rust::format::Error> {
        self.messages.push(message);
        Ok(())
    }
    fn metadata(
        &mut self,
        _: String,
        _: BTreeMap<String, String>,
    ) -> Result<(), foxglove_rust::format::Error> {
        Ok(())
    }
    fn attachment(
        &mut self,
        _: foxglove_rust::format::Attachment,
    ) -> Result<(), foxglove_rust::format::Error> {
        Ok(())
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn recovery_preserves_messages_and_schemaless_channels() {
    let workspace = Workspace::new();
    let messages = vec![
        message(1, 1, vec![1]),
        message(2, 2, b"{}".to_vec()),
        message(1, 3, vec![2]),
        message(2, 4, b"{\"value\":2}".to_vec()),
    ];
    let mut partial = recording(&messages[..2]);
    partial.truncate(partial.len() - 4);
    let mut replies = export_replies(partial);
    replies.extend(export_replies(recording(&messages[1..])));
    let server = Server::new(replies);
    let output = Process::spawn(workspace.command(&server.url).args([
        "data",
        "export",
        "--recording-id",
        "rec",
        "--output-file",
        "output.mcap",
    ]))
    .finish();
    assert_success(&output);
    let mut records = Records::default();
    foxglove_rust::format::read_mcap(
        &mut fs::File::open(workspace.0.join("output.mcap")).unwrap(),
        &mut records,
    )
    .unwrap();
    assert_eq!(records.messages.len(), messages.len());
    for (got, want) in records.messages.iter().zip(messages) {
        assert_eq!(
            (got.log_time, got.publish_time, &got.data),
            (want.log_time, want.publish_time, &want.data)
        );
        let channel = &records.channels[&got.channel_id];
        if want.channel_id == 2 {
            assert_eq!(channel.topic, "/untyped");
            assert_eq!(channel.schema_id, 0);
        } else {
            assert_eq!(channel.topic, "/typed");
            let schema = &records.schemas[&channel.schema_id];
            assert_eq!(schema.name, "example/Message");
            assert_eq!(schema.data, b"uint8 value\n");
        }
    }
    assert_eq!(server.finish().len(), 4);
}

#[cfg(all(unix, feature = "compat-test"))]
#[test]
#[ignore = "requires loopback sockets and Unix signal delivery"]
fn ctrl_c_preserves_credentials_and_exports_during_response_bodies() {
    use std::time::Duration;
    for (endpoint, login) in [("/v1/data/stream", false), ("/v1/signin", true)] {
        let workspace = Workspace::new();
        let mut replies = Vec::new();
        if login {
            replies.push(Reply::json("POST", "/v1/auth/device-code", r#"{"deviceCode":"fixture","userCode":"1234","verificationUriComplete":"https://example.invalid"}"#));
            replies.push(Reply::json(
                "POST",
                "/v1/auth/token",
                r#"{"idToken":"fixture"}"#,
            ));
        }
        replies.push(Reply {
            stall: true,
            ..Reply::json("POST", endpoint, "{")
        });
        let count = replies.len();
        let server = Server::new(replies);
        let config = b"bearer_token: original-fixture-token\n";
        fs::write(workspace.0.join(".foxgloverc"), config).unwrap();
        fs::write(workspace.0.join("output.mcap"), b"original destination").unwrap();
        let mut command = workspace.command(&server.url);
        if login {
            command.args(["auth", "login", "--base-url", &server.url]);
        } else {
            command.args([
                "data",
                "export",
                "--recording-id",
                "rec",
                "--output-file",
                "output.mcap",
            ]);
        }
        let child = Process::spawn(&mut command);
        for _ in 0..count {
            server
                .requests
                .recv_timeout(Duration::from_secs(10))
                .unwrap();
        }
        child.interrupt();
        let output = child.finish();
        assert_eq!(
            output.status.code(),
            Some(130),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(workspace.0.join(".foxgloverc")).unwrap(), config);
        assert_eq!(
            fs::read(workspace.0.join("output.mcap")).unwrap(),
            b"original destination"
        );
        assert_eq!(
            fs::read_dir(&workspace.0).unwrap().count(),
            2,
            "staging was not cleaned"
        );
        server.finish();
    }
}

type RequestCase<'a> = (&'a [&'a str], &'static str, &'static str, &'static str);

fn assert_each_request_reaches(cases: &[RequestCase<'_>]) {
    let workspace = Workspace::new();
    for &(args, method, path, body) in cases {
        let server = Server::new(vec![Reply::json(method, path, body)]);
        let output = Process::spawn(workspace.command(&server.url).args(args)).finish();
        assert_success(&output);
        assert_eq!(server.finish().len(), 1, "{args:?}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn identifier_paths_are_escaped_for_every_command() {
    const KEY: &str = "drive#1?/\\% snow☃";
    const SESSION_PATH: &str = "/v1/sessions/drive%231%3F%2F%5C%25%20snow%E2%98%83";
    const SESSION: &str = r#"{"id":"session","createdAt":"","updatedAt":""}"#;
    let cases: &[(&[&str], &str, &str, &str)] = &[
        (&["sessions", "get", KEY], "GET", SESSION_PATH, SESSION),
        (
            &["sessions", "recordings", "list", KEY],
            "GET",
            SESSION_PATH,
            SESSION,
        ),
        (&["sessions", "delete", KEY], "DELETE", SESSION_PATH, "{}"),
        (
            &["sessions", "recordings", "add", KEY, "rec"],
            "PATCH",
            SESSION_PATH,
            "{}",
        ),
        (
            &["sessions", "recordings", "remove", KEY, "rec"],
            "PATCH",
            SESSION_PATH,
            "{}",
        ),
        (
            &["devices", "edit", KEY, "--name", "renamed"],
            "PATCH",
            "/v1/devices/drive%231%3F%2F%5C%25%20snow%E2%98%83",
            r#"{"id":"device","name":"renamed"}"#,
        ),
        (
            &["recordings", "delete", KEY],
            "DELETE",
            "/v1/recordings/drive%231%3F%2F%5C%25%20snow%E2%98%83",
            "{}",
        ),
        (
            &["data", "import", "unused.mcap", "--edge-recording-id", KEY],
            "POST",
            "/v1/recordings/drive%231%3F%2F%5C%25%20snow%E2%98%83/import",
            r#"{"id":"recording"}"#,
        ),
        (
            &["extensions", "unpublish", KEY],
            "DELETE",
            "/v1/extensions/drive%231%3F%2F%5C%25%20snow%E2%98%83",
            "{}",
        ),
        (
            &["attachments", "download", KEY],
            "GET",
            "/v1/recording-attachments/drive%231%3F%2F%5C%25%20snow%E2%98%83/download",
            "attachment",
        ),
        (
            &["datasets", "episodes", "list", KEY],
            "GET",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/episodes",
            r#"{"episodes":[]}"#,
        ),
    ];
    assert_each_request_reaches(cases);
}

#[test]
#[ignore = "requires loopback sockets"]
fn dataset_and_episode_identifier_paths_are_escaped() {
    const KEY: &str = "drive#1?/\\% snow☃";
    const DATASET_PATH: &str = "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83";
    const EPISODE_PATH: &str = "/v1/episodes/drive%231%3F%2F%5C%25%20snow%E2%98%83";
    let cases: &[RequestCase] = &[
        (
            &["datasets", "get", KEY],
            "GET",
            DATASET_PATH,
            r#"{"id":"ds","projectId":"","name":"","createdAt":"","updatedAt":""}"#,
        ),
        (
            &["datasets", "edit", KEY, "--name", "renamed"],
            "PATCH",
            DATASET_PATH,
            "{}",
        ),
        (&["datasets", "delete", KEY], "DELETE", DATASET_PATH, "{}"),
        (
            &["datasets", "commit", KEY],
            "POST",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/commit",
            r#"{"committed":{"versionNumber":1}}"#,
        ),
        (
            &["datasets", "discard", KEY],
            "POST",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/discard",
            "{}",
        ),
        (
            &["datasets", "episodes", "add", KEY, "ep"],
            "PATCH",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/episodes",
            "{}",
        ),
        (
            &["datasets", "episodes", "remove", KEY, "ep"],
            "PATCH",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/episodes",
            "{}",
        ),
        (
            &["datasets", "episodes", "list", KEY, "--version", "2"],
            "GET",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/versions/2/episodes",
            r#"{"episodes":[]}"#,
        ),
        (
            &["datasets", "versions", "list", KEY],
            "GET",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/versions",
            r#"{"versions":[]}"#,
        ),
        (
            &["datasets", "versions", "get", KEY, "2"],
            "GET",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/versions/2",
            r#"{"versionNumber":2}"#,
        ),
        (
            &["datasets", "versions", "compare", KEY, "1", "2"],
            "GET",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/versions/2/compare",
            r#"{"changes":[],"addedCount":0,"removedCount":0,"nextCursor":null}"#,
        ),
        (
            &["datasets", "versions", "restore", KEY, "1"],
            "POST",
            "/v1/datasets/drive%231%3F%2F%5C%25%20snow%E2%98%83/versions/1/restore",
            "{}",
        ),
        (
            &["episodes", "get", KEY],
            "GET",
            EPISODE_PATH,
            r#"{"id":"ep","projectId":"","startTime":"","endTime":"","createdAt":""}"#,
        ),
        (&["episodes", "delete", KEY], "DELETE", EPISODE_PATH, "{}"),
    ];
    assert_each_request_reaches(cases);
}

#[test]
fn ambiguous_session_keys_are_rejected_before_sending_a_request() {
    let workspace = Workspace::new();
    for key in ["", ".", ".."] {
        let output = Process::spawn(
            workspace
                .command("http://127.0.0.1:1")
                .args(["sessions", "delete", key]),
        )
        .finish();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("API path segments must not be"));
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn empty_environment_overrides_use_saved_configuration() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json("GET", "/v1/recordings", "[]")]);
    fs::write(
        workspace.0.join(".foxgloverc"),
        format!(
            "base_url: {}\nbearer_token: saved-token\ndefault_project_id: saved-project\n",
            server.url,
        ),
    )
    .unwrap();
    let output = Process::spawn(
        workspace
            .command("")
            .env("BEARER_TOKEN", "")
            .env("DEFAULT_PROJECT_ID", "")
            .args(["recordings", "list", "--format", "json"]),
    )
    .finish();
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("projectId=saved-project"));
    assert!(requests[0].contains("authorization: Bearer saved-token\r\n"));
}

fn query_pairs(request: &str) -> BTreeMap<String, String> {
    let target = request.split_whitespace().nth(1).unwrap();
    reqwest::Url::parse(&format!("http://localhost{target}"))
        .unwrap()
        .query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}

fn expected_pairs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

#[test]
#[ignore = "requires loopback sockets"]
fn dataset_list_filters_reach_the_api() {
    const DATASETS: &str = r#"[{"id":"ds_one","projectId":"prj_explicit","name":"Highway","description":"Merges","episodeCount":2,"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"}]"#;
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json("GET", "/v1/datasets", DATASETS)]);
    let output = Process::spawn(workspace.command(&server.url).args([
        "datasets",
        "list",
        "--project-id",
        "prj_explicit",
        "--limit",
        "10",
        "--offset",
        "5",
        "--sort-by",
        "name",
        "--sort-order",
        "desc",
        "--format",
        "csv",
    ]))
    .finish();
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Name,Project ID,Description,Episode Count,Created At,Updated At\n\
         ds_one,Highway,prj_explicit,Merges,2,2024-01-02T03:04:05Z,2024-01-02T03:04:06Z\n"
    );
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[
            ("limit", "10"),
            ("offset", "5"),
            ("projectId", "prj_explicit"),
            ("sortBy", "name"),
            ("sortOrder", "desc"),
        ])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn episode_filters_reach_the_api_and_the_response_envelope_is_unwrapped() {
    const EPISODES: &str = r#"{"episodes":[{"id":"ep_one","projectId":"prj_explicit","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{"run":7},"recordings":[{"id":"rec_one","path":"one.mcap","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","available":false}],"createdAt":"2024-01-02T03:04:07Z"}]}"#;
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json("GET", "/v1/episodes", EPISODES)]);
    let output = Process::spawn(workspace.command(&server.url).args([
        "episodes",
        "list",
        "--project-id",
        "prj_explicit",
        "--start",
        "2024-01-02",
        "--end",
        "2024-01-03",
        "--recording-id",
        "rec_one",
        "--include-recordings",
        "--limit",
        "10",
        "--offset",
        "5",
        "--sort-by",
        "startTime",
        "--sort-order",
        "desc",
        "--format",
        "csv",
    ]))
    .finish();
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ID,Project ID,Start Time,End Time,Recordings,Metadata,Created At\n\
         ep_one,prj_explicit,2024-01-02T03:04:05Z,2024-01-02T03:04:06Z,rec_one,\"{\"\"run\"\":7}\",2024-01-02T03:04:07Z\n"
    );
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[
            ("end", "2024-01-03T00:00:00Z"),
            ("include", "recordings"),
            ("limit", "10"),
            ("offset", "5"),
            ("projectId", "prj_explicit"),
            ("recordingId", "rec_one"),
            ("sortBy", "startTime"),
            ("sortOrder", "desc"),
            ("start", "2024-01-02T00:00:00Z"),
        ])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn dataset_episode_membership_is_rendered_alongside_the_episode() {
    const EPISODES: &str = r#"{"episodes":[{"addedAt":"2024-01-02T03:04:08Z","addedInVersion":3,"episode":{"id":"ep_one","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{},"createdAt":"2024-01-02T03:04:07Z"}}]}"#;
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/episodes",
        EPISODES,
    )]);
    let output = Process::spawn(workspace.command(&server.url).args([
        "datasets",
        "episodes",
        "list",
        "ds_one",
        "--sort-by",
        "addedAt",
        "--format",
        "csv",
    ]))
    .finish();
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Episode ID,Project ID,Start Time,End Time,Recordings,Metadata,Added At,Added In Version,Created At\n\
         ep_one,prj_default,2024-01-02T03:04:05Z,2024-01-02T03:04:06Z,,{},2024-01-02T03:04:08Z,3,2024-01-02T03:04:07Z\n"
    );
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("limit", "2000"), ("sortBy", "addedAt"),])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_full_page_reports_that_more_results_may_exist() {
    fn page(count: usize) -> String {
        let episodes = (0..count)
            .map(|index| {
                format!(
                    r#"{{"id":"ep_{index}","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{{}},"createdAt":"2024-01-02T03:04:07Z"}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"episodes":[{episodes}]}}"#)
    }

    let workspace = Workspace::new();
    for (returned, requested, expected) in [
        (
            3,
            "3",
            "Showing the first 3 results. More may exist; use --offset to page through them.\n",
        ),
        (2, "3", ""),
        (0, "0", ""),
    ] {
        let body = page(returned);
        let server = Server::new(vec![Reply {
            body: body.into_bytes(),
            ..Reply::json("GET", "/v1/episodes", "")
        }]);
        let output = Process::spawn(
            workspace
                .command(&server.url)
                .args(["episodes", "list", "--limit", requested]),
        )
        .finish();
        assert_success(&output);
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected,
            "{returned} of {requested}"
        );
        assert_eq!(
            query_pairs(&server.finish()[0])
                .get("limit")
                .map(String::as_str),
            Some(requested)
        );
    }
}

const DOWNLOAD_DATASET: &str = r#"{"id":"ds_one","projectId":"prj_default","name":"Highway merges","createdAt":"","updatedAt":""}"#;

fn dataset_episode(id: &str, missing_recordings: bool) -> String {
    format!(
        r#"{{"addedAt":"2024-01-02T03:04:08Z","addedInVersion":4,"hasMissingRecordings":{missing_recordings},"episode":{{"id":"{id}","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{{}},"createdAt":"2024-01-02T03:04:07Z"}}}}"#
    )
}

fn dataset_replies(episodes: &[String]) -> Vec<Reply> {
    vec![
        Reply::json("GET", "/v1/datasets/ds_one", DOWNLOAD_DATASET),
        Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions",
            r#"{"versions":[{"versionNumber":5},{"versionNumber":4,"committedAt":"2024-01-02T00:00:00Z"},{"versionNumber":3,"committedAt":"2024-01-01T00:00:00Z"}]}"#,
        ),
        Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/4/episodes",
            &format!(r#"{{"episodes":[{}]}}"#, episodes.join(",")),
        ),
    ]
}

fn download_dataset(workspace: &Workspace, server: &Server, args: &[&str]) -> Output {
    Process::spawn(
        workspace
            .command(&server.url)
            .args(["datasets", "download", "ds_one"])
            .args(args),
    )
    .finish()
}

fn download_manifest(workspace: &Workspace) -> serde_json::Value {
    let path = workspace.0.join("Highway-merges-v4").join("manifest.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn no_streamable_recordings() -> Reply {
    Reply {
        status: 404,
        ..Reply::json(
            "POST",
            "/v1/data/stream",
            r#"{"error":"Episode has no recordings available for streaming","code":"NoStreamableRecordings"}"#,
        )
    }
}

fn episode_mcap() -> Vec<u8> {
    static EPISODE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    EPISODE
        .get_or_init(|| recording(&[message(1, 1, vec![1])]))
        .clone()
}

#[test]
#[ignore = "requires loopback sockets"]
fn downloading_a_dataset_version_writes_episodes_and_a_manifest() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_two", true),
    ]);
    replies.extend(export_replies(episode_mcap()));
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &["--topics", "/a, /b"]);
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(
        query_pairs(&requests[1]),
        expected_pairs(&[("sortOrder", "desc")])
    );

    let root = workspace.0.join("Highway-merges-v4");
    for file in ["episode_0000_ep_one.mcap", "episode_0001_ep_two.mcap"] {
        assert_eq!(fs::read(root.join(file)).unwrap(), episode_mcap());
    }
    let manifest = download_manifest(&workspace);
    assert_eq!(manifest["version"]["versionNumber"], 4);
    assert_eq!(
        manifest["selection"]["topics"],
        serde_json::json!(["/a", "/b"])
    );
    assert_eq!(
        manifest["selection"]["digest"],
        "9fb275812cf27c6fc9747e249fef9132e3ccd5edf1aeb68b22b8452797db8b0c"
    );
    assert_eq!(
        manifest["episodes"][0]["file"],
        "Highway-merges-v4/episode_0000_ep_one.mcap"
    );
    assert_eq!(manifest["episodes"][0]["byteSize"], episode_mcap().len());
    assert_eq!(
        manifest["episodes"][0]["episodeHasMissingRecordings"],
        false
    );
    assert_eq!(manifest["episodes"][1]["episodeHasMissingRecordings"], true);

    let body = requests[3].split("\r\n\r\n").nth(1).unwrap();
    let stream: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(stream["episodeId"], "ep_one");
    assert_eq!(stream["topics"], serde_json::json!(["/a", "/b"]));
    assert_eq!(stream["start"], "2024-01-02T03:04:05Z");
    assert_eq!(stream["end"], "2024-01-02T03:04:06Z");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "Downloaded 2 of 2 episodes to Highway-merges-v4 (0 failed, 0 skipped)\n1 downloaded episode has missing recordings"
        ),
        "{stderr}"
    );

    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_two", true),
    ]);
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &["--topics", "/a, /b", "--strict"]);
    server.finish();
    assert_eq!(output.status.code(), Some(1));

    let workspace = Workspace::new();
    let episodes = [
        dataset_episode("ep_one", false),
        dataset_episode("ep_gone", false),
    ];
    let mut replies = dataset_replies(&episodes);
    replies.extend(export_replies(episode_mcap()));
    replies.push(no_streamable_recordings());
    let server = Server::new(replies);
    assert_success(&download_dataset(&workspace, &server, &[]));
    server.finish();
    let mut replies = dataset_replies(&episodes);
    replies.push(no_streamable_recordings());
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &["--strict"]);
    server.finish();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
#[ignore = "requires loopback sockets"]
fn episodes_that_cannot_be_downloaded_are_recorded_in_the_manifest() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_gone", true),
        dataset_episode("ep_truncated", false),
    ]);
    replies.extend(export_replies(episode_mcap()));
    replies.push(no_streamable_recordings());
    for _ in 0..2 {
        replies.extend(export_replies(b"partial".to_vec()));
    }
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &[]);
    assert!(!output.status.success());
    server.finish();

    let root = workspace.0.join("Highway-merges-v4");
    assert!(!root.join("episode_0002_ep_truncated.mcap").exists());
    let manifest = download_manifest(&workspace);
    let statuses: Vec<_> = manifest["episodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|episode| episode["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["downloaded", "skipped", "failed"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Downloaded 1 of 3 episodes to Highway-merges-v4 (1 failed, 1 skipped)"),
        "{stderr}"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_rerun_reuses_the_episodes_an_earlier_run_downloaded() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_two", true),
    ]);
    replies.extend(export_replies(episode_mcap()));
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    assert_success(&download_dataset(&workspace, &server, &[]));
    server.finish();

    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", true),
        dataset_episode("ep_two", true),
    ]);
    replies.push(no_streamable_recordings());
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &[]);
    assert_success(&output);
    server.finish();
    let size = episode_mcap().len();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("Episode 1 of 2: {size} bytes already downloaded")),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "Episode 2 of 2: {size} bytes kept from an earlier run (skipped: "
        )),
        "{stderr}"
    );
    let manifest = download_manifest(&workspace);
    for (index, missing_recordings) in [(0, false), (1, true)] {
        let episode = &manifest["episodes"][index];
        assert_eq!(episode["status"], "downloaded");
        assert_eq!(episode["episodeHasMissingRecordings"], missing_recordings);
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_failed_refresh_keeps_the_earlier_file_and_fails_the_run() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[dataset_episode("ep_one", true)]);
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    assert_success(&download_dataset(&workspace, &server, &[]));
    server.finish();

    let mut replies = dataset_replies(&[dataset_episode("ep_one", true)]);
    replies.push(Reply {
        status: 403,
        ..Reply::json("POST", "/v1/data/stream", r#"{"error":"Forbidden"}"#)
    });
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &[]);
    server.finish();
    assert!(!output.status.success());
    let size = episode_mcap().len();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "Episode 1 of 1: {size} bytes kept from an earlier run (failed: "
        )) && stderr.contains("(1 failed, 0 skipped)"),
        "{stderr}"
    );
    let root = workspace.0.join("Highway-merges-v4");
    assert_eq!(
        fs::read(root.join("episode_0000_ep_one.mcap")).unwrap(),
        episode_mcap()
    );
    let episode = &download_manifest(&workspace)["episodes"][0];
    assert_eq!(episode["status"], "failed");
    assert!(episode["reason"].is_string(), "{episode}");
    assert_eq!(episode["byteSize"], size);
    assert_eq!(
        episode["file"],
        "Highway-merges-v4/episode_0000_ep_one.mcap"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_killed_run_has_recorded_the_episodes_it_finished() {
    let workspace = Workspace::new();
    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_two", false),
    ]);
    replies.extend(export_replies(episode_mcap()));
    replies.extend(export_replies(episode_mcap()));
    replies.last_mut().unwrap().stall = true;
    let requests_before_kill = replies.len();
    let server = Server::new(replies);
    let process = Process::spawn(
        workspace
            .command(&server.url)
            .args(["datasets", "download", "ds_one"]),
    );
    for _ in 0..requests_before_kill {
        server
            .requests
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
    }
    drop(process);
    server.finish();

    let manifest = download_manifest(&workspace);
    let episodes = manifest["episodes"].as_array().unwrap();
    assert_eq!(episodes.len(), 1, "{manifest}");
    assert_eq!(episodes[0]["id"], "ep_one");
    assert_eq!(episodes[0]["status"], "downloaded");

    let mut replies = dataset_replies(&[
        dataset_episode("ep_one", false),
        dataset_episode("ep_two", false),
    ]);
    replies.extend(export_replies(episode_mcap()));
    let server = Server::new(replies);
    let output = download_dataset(&workspace, &server, &[]);
    assert_success(&output);
    assert_eq!(server.finish().len(), 5);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Episode 1 of 2: ") && stderr.contains("bytes already downloaded"),
        "{stderr}"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn an_uncommitted_version_is_not_downloaded() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json("GET", "/v1/datasets/ds_one", DOWNLOAD_DATASET),
        Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/5",
            r#"{"versionNumber":5}"#,
        ),
    ]);
    let output = download_dataset(&workspace, &server, &["--version", "5"]);
    server.finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Version 5 is not a committed version of this dataset\n"
    );
}

#[test]
fn half_open_episode_time_ranges_are_rejected_before_sending_a_request() {
    let workspace = Workspace::new();
    for args in [
        vec!["episodes", "list", "--start", "2024-01-02"],
        vec!["episodes", "list", "--end", "2024-01-03"],
        vec![
            "datasets",
            "episodes",
            "list",
            "ds_one",
            "--start",
            "2024-01-02",
        ],
    ] {
        let output = Process::spawn(workspace.command("http://127.0.0.1:1").args(&args)).finish();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "both --start and --end must be specified, or neither\n",
            "{args:?}"
        );
    }
}

fn json_body(request: &str) -> serde_json::Value {
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}

fn run(workspace: &Workspace, server: &Server, args: &[&str]) -> Output {
    Process::spawn(workspace.command(&server.url).args(args)).finish()
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_a_dataset_sends_its_episodes_and_says_how_to_commit_them() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "POST",
        "/v1/datasets",
        r#"{"id":"ds_new","projectId":"prj_explicit","name":"Highway","createdAt":"","updatedAt":"","added":1,"removed":0,"alreadyPresent":1}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets",
            "add",
            "--name",
            "Highway",
            "--description",
            "Merges",
            "--project-id",
            "prj_explicit",
            "--episode-id",
            "ep_one",
            "--episode-id",
            "ep_two",
        ],
    );
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({
            "projectId": "prj_explicit",
            "name": "Highway",
            "description": "Merges",
            "episodeIds": ["ep_one", "ep_two"],
        })
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Dataset created: ds_new\n\
         Added 1 episode (1 already present)\n\
         Run foxglove datasets commit ds_new to commit the draft as a new version.\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_an_empty_dataset_sends_only_its_name_and_project() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "POST",
        "/v1/datasets",
        r#"{"id":"ds_new","projectId":"prj_default","name":"Highway","createdAt":"","updatedAt":"","added":0,"removed":0,"alreadyPresent":0}"#,
    )]);
    let output = Process::spawn(
        workspace
            .command(&server.url)
            .env("DEFAULT_PROJECT_ID", "prj_default")
            .args(["datasets", "add", "--name", "Highway", "--description", ""]),
    )
    .finish();
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({"projectId": "prj_default", "name": "Highway"})
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Dataset created: ds_new\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_in_an_unknown_project_names_the_project() {
    let workspace = Workspace::new();
    for (mut args, path) in [
        (vec!["datasets", "add", "--name", "Highway"], "/v1/datasets"),
        (
            vec!["episodes", "add", "--recording-id", "rec_one"],
            "/v1/episodes",
        ),
    ] {
        let server = Server::new(vec![Reply {
            status: 404,
            ..Reply::json("POST", path, r#"{"error":"Project not found"}"#)
        }]);
        args.extend(["--project-id", "prj_gone"]);
        let output = run(&workspace, &server, &args);
        server.finish();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Project not found: prj_gone\n",
            "{args:?}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn editing_a_dataset_sends_only_the_fields_given_and_an_empty_description_clears_it() {
    let workspace = Workspace::new();
    for (flags, expected) in [
        (
            vec!["--name", "Renamed"],
            serde_json::json!({"name": "Renamed"}),
        ),
        (
            vec!["--description", ""],
            serde_json::json!({"description": null}),
        ),
    ] {
        let server = Server::new(vec![Reply::json("PATCH", "/v1/datasets/ds_one", "{}")]);
        let mut args = vec!["datasets", "edit", "ds_one"];
        args.extend(&flags);
        let output = run(&workspace, &server, &args);
        assert_success(&output);
        assert_eq!(json_body(&server.finish()[0]), expected, "{flags:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Dataset updated: ds_one\n"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn getting_a_dataset_renders_one_record() {
    const DATASET: &str = r#"{"id":"ds_one","projectId":"prj_default","name":"Highway","description":"Merges","episodeCount":2,"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"}"#;
    let workspace = Workspace::new();
    for (format, expected) in [
        (
            "json",
            "{\"id\":\"ds_one\",\"projectId\":\"prj_default\",\"name\":\"Highway\",\"description\":\"Merges\",\"episodeCount\":2,\"createdAt\":\"2024-01-02T03:04:05Z\",\"updatedAt\":\"2024-01-02T03:04:06Z\"}\n",
        ),
        (
            "csv",
            "ID,Name,Project ID,Description,Episode Count,Created At,Updated At\n\
             ds_one,Highway,prj_default,Merges,2,2024-01-02T03:04:05Z,2024-01-02T03:04:06Z\n",
        ),
    ] {
        let server = Server::new(vec![Reply::json("GET", "/v1/datasets/ds_one", DATASET)]);
        let output = run(
            &workspace,
            &server,
            &["datasets", "get", "ds_one", "--format", format],
        );
        assert_success(&output);
        server.finish();
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{format}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn deleting_what_is_already_gone_is_not_an_error() {
    let workspace = Workspace::new();
    for (command, path, noun) in [
        ("datasets", "/v1/datasets/one", "Dataset"),
        ("episodes", "/v1/episodes/one", "Episode"),
    ] {
        let server = Server::new(vec![Reply {
            status: 404,
            ..Reply::json("DELETE", path, r#"{"error":"Not Found"}"#)
        }]);
        let output = run(&workspace, &server, &[command, "delete", "one"]);
        assert_success(&output);
        server.finish();
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "Not found. The resource may have already been deleted.\n{noun} deleted: one\n"
            )
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn missing_datasets_episodes_and_versions_are_named_in_the_error() {
    let workspace = Workspace::new();
    let cases: &[(&[&str], &str, &str, &str)] = &[
        (
            &["datasets", "get", "ds_one"],
            "GET",
            "/v1/datasets/ds_one",
            "Dataset not found: ds_one\n",
        ),
        (
            &["datasets", "commit", "ds_one"],
            "POST",
            "/v1/datasets/ds_one/commit",
            "Dataset not found: ds_one\n",
        ),
        (
            &["episodes", "get", "ep_one"],
            "GET",
            "/v1/episodes/ep_one",
            "Episode not found: ep_one\n",
        ),
        (
            &["datasets", "versions", "get", "ds_one", "9"],
            "GET",
            "/v1/datasets/ds_one/versions/9",
            "Version 9 of dataset ds_one not found\n",
        ),
        (
            &["datasets", "versions", "compare", "ds_one", "2", "9"],
            "GET",
            "/v1/datasets/ds_one/versions/9/compare",
            "Version 2 or 9 of dataset ds_one not found\n",
        ),
        (
            &["datasets", "versions", "restore", "ds_one", "9"],
            "POST",
            "/v1/datasets/ds_one/versions/9/restore",
            "Committed version 9 of dataset ds_one not found\n",
        ),
    ];
    for &(args, method, path, expected) in cases {
        let server = Server::new(vec![Reply {
            status: 404,
            ..Reply::json(method, path, r#"{"error":"Not Found"}"#)
        }]);
        let output = run(&workspace, &server, args);
        server.finish();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            expected,
            "{args:?}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn episode_changes_report_what_the_api_applied() {
    let workspace = Workspace::new();
    for (args, response, body, stderr) in [
        (
            vec!["add", "ds_one", "ep_one", "ep_two", "ep_two"],
            r#"{"added":1,"removed":0,"alreadyPresent":2}"#,
            serde_json::json!({"add": ["ep_one", "ep_two", "ep_two"]}),
            "Added 1 episode (2 already present)\n\
             Run foxglove datasets commit ds_one to commit the draft as a new version.\n",
        ),
        (
            vec!["remove", "ds_one", "ep_one", "ep_gone", "ep_one"],
            r#"{"added":0,"removed":1,"alreadyPresent":0}"#,
            serde_json::json!({"remove": ["ep_one", "ep_gone", "ep_one"]}),
            "Removed 1 episode (1 not in the dataset)\n\
             Run foxglove datasets commit ds_one to commit the draft as a new version.\n",
        ),
        (
            vec!["remove", "ds_one", "ep_gone"],
            r#"{"added":0,"removed":0,"alreadyPresent":0}"#,
            serde_json::json!({"remove": ["ep_gone"]}),
            "Removed 0 episodes (1 not in the dataset)\n",
        ),
    ] {
        let server = Server::new(vec![Reply::json(
            "PATCH",
            "/v1/datasets/ds_one/episodes",
            response,
        )]);
        let mut command = vec!["datasets", "episodes"];
        command.extend(&args);
        let output = run(&workspace, &server, &command);
        assert_success(&output);
        assert_eq!(json_body(&server.finish()[0]), body, "{args:?}");
        assert_eq!(String::from_utf8_lossy(&output.stderr), stderr, "{args:?}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_version_and_the_missing_recordings_filter_reach_the_dataset_episode_list() {
    let workspace = Workspace::new();
    for (flag, value) in [
        ("--has-missing-recordings", "true"),
        ("--has-missing-recordings=false", "false"),
    ] {
        let server = Server::new(vec![Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/3/episodes",
            &format!(
                r#"{{"episodes":[{}]}}"#,
                dataset_episode("ep_one", value == "true")
            ),
        )]);
        let output = run(
            &workspace,
            &server,
            &[
                "datasets",
                "episodes",
                "list",
                "ds_one",
                "--version",
                "3",
                flag,
                "--format",
                "json",
            ],
        );
        assert_success(&output);
        assert_eq!(
            query_pairs(&server.finish()[0]),
            expected_pairs(&[("hasMissingRecordings", value), ("limit", "2000")])
        );
        let episodes: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            episodes[0]["hasMissingRecordings"],
            value == "true",
            "{flag}"
        );
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn the_draft_version_is_looked_up_before_listing_its_episodes() {
    let workspace = Workspace::new();
    let server = Server::new(vec![
        Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions",
            r#"{"versions":[{"versionNumber":5,"createdAt":"2024-01-03T00:00:00Z"}]}"#,
        ),
        Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/5/episodes",
            &format!(r#"{{"episodes":[{}]}}"#, dataset_episode("ep_one", false)),
        ),
    ]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets",
            "episodes",
            "list",
            "ds_one",
            "--version",
            "draft",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(
        query_pairs(&requests[0]),
        expected_pairs(&[("limit", "1"), ("sortOrder", "desc")])
    );
    let episodes: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(episodes[0]["episode"]["id"], "ep_one");
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_dataset_without_a_draft_says_so() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions",
        r#"{"versions":[{"versionNumber":4,"committedAt":"2024-01-02T00:00:00Z"}]}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets",
            "episodes",
            "list",
            "ds_one",
            "--version",
            "draft",
        ],
    );
    assert_eq!(server.finish().len(), 1);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Dataset ds_one has no draft\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_missing_version_is_named_in_the_error() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply {
        status: 404,
        ..Reply::json(
            "GET",
            "/v1/datasets/ds_one/versions/9/episodes",
            r#"{"error":"Version not found"}"#,
        )
    }]);
    let output = run(
        &workspace,
        &server,
        &["datasets", "episodes", "list", "ds_one", "--version", "9"],
    );
    server.finish();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Version 9 of dataset ds_one not found\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn the_episode_list_sends_the_missing_recordings_filter_and_passes_the_field_through() {
    fn episodes(missing_recordings: &str) -> String {
        format!(
            r#"{{"episodes":[{{"id":"ep_one","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{{}},{missing_recordings}"createdAt":"2024-01-02T03:04:07Z"}}]}}"#
        )
    }
    let workspace = Workspace::new();
    for (flags, supplied, query, expected) in [
        (
            vec!["--has-missing-recordings"],
            "",
            Some("true"),
            serde_json::Value::Null,
        ),
        (
            vec!["--has-missing-recordings=false"],
            "",
            Some("false"),
            serde_json::Value::Null,
        ),
        (
            vec!["--include-recordings"],
            r#""hasMissingRecordings":true,"#,
            None,
            serde_json::json!(true),
        ),
        (vec![], "", None, serde_json::Value::Null),
    ] {
        let server = Server::new(vec![Reply::json(
            "GET",
            "/v1/episodes",
            &episodes(supplied),
        )]);
        let mut args = vec!["episodes", "list", "--format", "json"];
        args.extend(&flags);
        let output = run(&workspace, &server, &args);
        assert_success(&output);
        assert_eq!(
            query_pairs(&server.finish()[0])
                .get("hasMissingRecordings")
                .map(String::as_str),
            query,
            "{flags:?}"
        );
        let episodes: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(episodes[0]["hasMissingRecordings"], expected, "{flags:?}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn versions_are_listed_with_the_draft_marked() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions",
        r#"{"versions":[{"versionNumber":1,"committedAt":"2024-01-02T00:00:00Z","createdAt":"2024-01-01T00:00:00Z","episodeCount":2,"addedEpisodeCount":2,"removedEpisodeCount":0},{"versionNumber":2,"createdAt":"2024-01-02T00:00:00Z","episodeCount":3,"addedEpisodeCount":1,"removedEpisodeCount":0}]}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets",
            "versions",
            "list",
            "ds_one",
            "--sort-order",
            "asc",
            "--limit",
            "2",
            "--offset",
            "1",
            "--format",
            "csv",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("limit", "2"), ("offset", "1"), ("sortOrder", "asc")])
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Showing the first 2 results. More may exist; use --offset to page through them.\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Version,Status,Committed At,Episode Count,Added,Removed,Created At\n\
         1,committed,2024-01-02T00:00:00Z,2,2,0,2024-01-01T00:00:00Z\n\
         2,draft,,3,1,0,2024-01-02T00:00:00Z\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn versions_are_listed_a_full_page_at_a_time_by_default() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions",
        r#"{"versions":[]}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &["datasets", "versions", "list", "ds_one"],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("limit", "2000")])
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_version_reports_whether_its_recordings_are_missing() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions/1",
        r#"{"versionNumber":1,"committedAt":"2024-01-02T00:00:00Z","createdAt":"2024-01-01T00:00:00Z","episodeCount":2,"addedEpisodeCount":2,"removedEpisodeCount":0,"hasMissingRecordings":true}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets", "versions", "get", "ds_one", "1", "--format", "csv",
        ],
    );
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Version,Status,Committed At,Episode Count,Added,Removed,Created At,Missing Recordings\n\
         1,committed,2024-01-02T00:00:00Z,2,2,0,2024-01-01T00:00:00Z,true\n"
    );
}

fn change(side: &str, id: &str) -> String {
    format!(
        r#"{{"change":"{side}","addedAt":"2024-01-02T03:04:08Z","addedInVersion":1,"episode":{{"id":"{id}","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{{}},"createdAt":"2024-01-02T03:04:07Z"}}}}"#
    )
}

#[test]
#[ignore = "requires loopback sockets"]
fn comparing_versions_fetches_one_page_and_says_how_to_get_the_next() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions/3/compare",
        &format!(
            r#"{{"changes":[{},{}],"addedCount":2,"removedCount":1,"nextCursor":"page2"}}"#,
            change("added", "ep_one"),
            change("removed", "ep_two")
        ),
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
            "3",
            "--include-recordings",
            "--format",
            "csv",
        ],
    );
    assert_success(&output);
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        query_pairs(&requests[0]),
        expected_pairs(&[
            ("include", "recordings"),
            ("limit", "2000"),
            ("version", "1")
        ])
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let changes: Vec<_> = stdout
        .lines()
        .map(|line| line.split(',').take(2).collect::<Vec<_>>().join(","))
        .collect();
    assert_eq!(
        changes,
        ["Change,Episode ID", "added,ep_one", "removed,ep_two"]
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "2 episodes added, 1 removed\n\
         More changes exist; rerun with --cursor page2 to fetch the next page.\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn comparing_versions_sends_the_given_cursor_and_limit() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions/3/compare",
        &format!(
            r#"{{"changes":[{}],"addedCount":2,"removedCount":1,"nextCursor":null}}"#,
            change("added", "ep_three")
        ),
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "datasets", "versions", "compare", "ds_one", "1", "3", "--cursor", "page2", "--limit",
            "2",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("cursor", "page2"), ("limit", "2"), ("version", "1")])
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "2 episodes added, 1 removed\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn an_empty_compare_page_offers_no_next_page() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/datasets/ds_one/versions/2/compare",
        r#"{"changes":[],"addedCount":0,"removedCount":0,"nextCursor":"again"}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &["datasets", "versions", "compare", "ds_one", "1", "2"],
    );
    assert_success(&output);
    server.finish();
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "0 episodes added, 0 removed\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn restoring_a_version_reports_the_changes_it_staged() {
    let workspace = Workspace::new();
    for (flags, query, response, stderr) in [
        (
            vec!["--force"],
            vec![("force", "true")],
            r#"{"added":2,"removed":1,"discardedAdds":1,"discardedRemoves":0}"#,
            "Discarded 1 pending addition\n\
             Restored version 3 into the draft: 2 episodes added, 1 removed\n\
             Run foxglove datasets commit ds_one to commit the draft as a new version.\n",
        ),
        (
            vec![],
            vec![],
            r#"{"added":0,"removed":0,"discardedAdds":0,"discardedRemoves":0}"#,
            "The draft matches version 3\n",
        ),
        (
            vec!["--force"],
            vec![("force", "true")],
            r#"{"added":0,"removed":0,"discardedAdds":2,"discardedRemoves":0}"#,
            "Discarded 2 pending additions\nThe draft matches version 3\n",
        ),
    ] {
        let server = Server::new(vec![Reply::json(
            "POST",
            "/v1/datasets/ds_one/versions/3/restore",
            response,
        )]);
        let mut args = vec!["datasets", "versions", "restore", "ds_one", "3"];
        args.extend(&flags);
        let output = run(&workspace, &server, &args);
        assert_success(&output);
        let request = &server.finish()[0];
        assert_eq!(query_pairs(request), expected_pairs(&query), "{flags:?}");
        assert_eq!(json_body(request), serde_json::json!({}));
        assert_eq!(String::from_utf8_lossy(&output.stderr), stderr, "{flags:?}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn a_restore_over_pending_changes_fails_without_force() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply {
        status: 409,
        ..Reply::json(
            "POST",
            "/v1/datasets/ds_one/versions/3/restore",
            r#"{"error":"Editable version has staged changes; pass force to overwrite"}"#,
        )
    }]);
    let output = run(
        &workspace,
        &server,
        &["datasets", "versions", "restore", "ds_one", "3"],
    );
    server.finish();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Dataset ds_one has pending changes. Commit them first, or pass --force to discard them.\n"
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn committing_and_discarding_report_the_counts() {
    let workspace = Workspace::new();
    for (command, path, response, stderr) in [
        (
            "commit",
            "/v1/datasets/ds_one/commit",
            r#"{"committed":{"versionNumber":4,"committedAt":"2024-01-02T00:00:00Z","createdAt":"2024-01-01T00:00:00Z","episodeCount":12,"addedEpisodeCount":3,"removedEpisodeCount":1},"editableVersionNumber":5}"#,
            "Committed version 4 with 12 episodes (3 added, 1 removed)\n",
        ),
        (
            "discard",
            "/v1/datasets/ds_one/discard",
            r#"{"discardedAdds":2,"discardedRemoves":1}"#,
            "Discarded 2 pending additions and 1 pending removal\n",
        ),
        (
            "discard",
            "/v1/datasets/ds_one/discard",
            r#"{"discardedAdds":0,"discardedRemoves":3}"#,
            "Discarded 3 pending removals\n",
        ),
        (
            "discard",
            "/v1/datasets/ds_one/discard",
            r#"{"discardedAdds":0,"discardedRemoves":0}"#,
            "No pending changes to discard\n",
        ),
    ] {
        let server = Server::new(vec![Reply::json("POST", path, response)]);
        let output = run(&workspace, &server, &["datasets", command, "ds_one"]);
        assert_success(&output);
        assert_eq!(json_body(&server.finish()[0]), serde_json::json!({}));
        assert_eq!(String::from_utf8_lossy(&output.stderr), stderr, "{command}");
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn creating_an_episode_keeps_its_window_to_the_millisecond() {
    let workspace = Workspace::new();
    for (created, stderr) in [
        (true, "Episode created: ep_new\n"),
        (
            false,
            "Episode already exists: ep_new (its metadata is unchanged)\n",
        ),
    ] {
        let server = Server::new(vec![Reply::json(
            "POST",
            "/v1/episodes",
            &format!(r#"{{"episodes":[{{"id":"ep_new","created":{created}}}]}}"#),
        )]);
        let output = run(
            &workspace,
            &server,
            &[
                "episodes",
                "add",
                "--project-id",
                "prj_explicit",
                "--recording-id",
                "rec_one",
                "--recording-id",
                "rec_two",
                "--start",
                "2024-01-02T03:04:05.250Z",
                "--end",
                "2024-01-02T03:04:06.1239Z",
                "--metadata",
                r#"{"run":7}"#,
            ],
        );
        assert_success(&output);
        assert_eq!(
            json_body(&server.finish()[0]),
            serde_json::json!({
                "projectId": "prj_explicit",
                "episodes": [{
                    "recordings": ["rec_one", "rec_two"],
                    "startTime": "2024-01-02T03:04:05.25Z",
                    "endTime": "2024-01-02T03:04:06.123Z",
                    "metadata": {"run": 7},
                }],
            })
        );
        assert_eq!(String::from_utf8_lossy(&output.stderr), stderr);
    }
}

#[test]
#[ignore = "requires loopback sockets"]
fn an_episode_window_left_out_is_inferred_by_the_api() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "POST",
        "/v1/episodes",
        r#"{"episodes":[{"id":"ep_new","created":true}]}"#,
    )]);
    let output = Process::spawn(
        workspace
            .command(&server.url)
            .env("DEFAULT_PROJECT_ID", "prj_default")
            .args(["episodes", "add", "--recording-id", "rec_one"]),
    )
    .finish();
    assert_success(&output);
    assert_eq!(
        json_body(&server.finish()[0]),
        serde_json::json!({"projectId": "prj_default", "episodes": [{"recordings": ["rec_one"]}]})
    );
}

#[test]
#[ignore = "requires loopback sockets"]
fn getting_an_episode_can_include_its_recordings() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply::json(
        "GET",
        "/v1/episodes/ep_one",
        r#"{"id":"ep_one","projectId":"prj_default","startTime":"2024-01-02T03:04:05Z","endTime":"2024-01-02T03:04:06Z","metadata":{},"recordings":[{"id":"rec_one","path":"one.mcap","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","available":false}],"hasMissingRecordings":true,"createdAt":"2024-01-02T03:04:07Z"}"#,
    )]);
    let output = run(
        &workspace,
        &server,
        &[
            "episodes",
            "get",
            "ep_one",
            "--include-recordings",
            "--format",
            "json",
        ],
    );
    assert_success(&output);
    assert_eq!(
        query_pairs(&server.finish()[0]),
        expected_pairs(&[("include", "recordings")])
    );
    let episode: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(episode["id"], "ep_one");
    assert_eq!(episode["recordings"][0]["available"], false);
    assert_eq!(episode["hasMissingRecordings"], true);
}

#[test]
#[ignore = "requires loopback sockets"]
fn an_episode_in_a_dataset_is_not_deleted() {
    let workspace = Workspace::new();
    let server = Server::new(vec![Reply {
        status: 409,
        ..Reply::json(
            "DELETE",
            "/v1/episodes/ep_one",
            r#"{"error":"Cannot delete an episode that belongs to a dataset"}"#,
        )
    }]);
    let output = run(&workspace, &server, &["episodes", "delete", "ep_one"]);
    server.finish();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Failed to delete episode: Cannot delete an episode that belongs to a dataset\n"
    );
}

#[test]
fn dataset_and_episode_writes_are_validated_before_sending_a_request() {
    let workspace = Workspace::new();
    for (args, expected) in [
        (
            vec!["datasets", "add", "--name", "Highway"],
            "--project-id is required when creating a dataset\n",
        ),
        (
            vec!["datasets", "add", "--name", " ", "--project-id", "prj"],
            "--name cannot be empty\n",
        ),
        (vec!["datasets", "edit", "ds_one"], "Nothing to update\n"),
        (
            vec!["datasets", "edit", "ds_one", "--name", ""],
            "--name cannot be empty\n",
        ),
        (
            vec!["episodes", "add", "--recording-id", "rec_one"],
            "--project-id is required when creating an episode\n",
        ),
        (
            vec![
                "episodes",
                "add",
                "--recording-id",
                "rec_one",
                "--start",
                "2024-01-02",
            ],
            "both --start and --end must be specified, or neither\n",
        ),
        (
            vec![
                "episodes",
                "add",
                "--recording-id",
                "rec_one",
                "--metadata",
                "[1]",
            ],
            "--metadata must be a JSON object: [1]\n",
        ),
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
