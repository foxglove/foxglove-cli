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
            &["recordings", "transfer", KEY],
            "POST",
            "/v1/recordings/drive%231%3F%2F%5C%25%20snow%E2%98%83/import",
            r#"{"id":"recording","importStatus":"pending"}"#,
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
    let workspace = Workspace::new();
    for &(args, method, path, body) in cases {
        let server = Server::new(vec![Reply::json(method, path, body)]);
        let output = Process::spawn(workspace.command(&server.url).args(args)).finish();
        assert_success(&output);
        assert_eq!(server.finish().len(), 1, "{args:?}");
    }
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

#[test]
fn v2_command_paths_replace_data_without_aliases() {
    let workspace = Workspace::new();
    let help = Process::spawn(workspace.command("http://127.0.0.1:1").arg("--help")).finish();
    assert_success(&help);
    assert!(!String::from_utf8_lossy(&help.stdout).contains("\n  data "));
    for args in [
        vec!["upload", "--help"],
        vec!["export", "--help"],
        vec!["coverage", "list", "--help"],
        vec!["recordings", "transfer", "--help"],
    ] {
        assert_success(
            &Process::spawn(workspace.command("http://127.0.0.1:1").args(args)).finish(),
        );
    }
    for args in [
        vec!["data", "export"],
        vec!["data", "import", "unused"],
        vec!["data", "coverage", "list"],
        vec!["data", "imports", "list"],
        vec!["data", "export", "--recording-id", "rec", "--help"],
        vec![
            "--config",
            "missing-directory/config.yaml",
            "data",
            "--help",
        ],
    ] {
        let output = Process::spawn(workspace.command("http://127.0.0.1:1").args(args)).finish();
        assert!(!output.status.success());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains("unrecognized subcommand 'data'"));
        assert!(!message.contains("removed in v2"));
        assert!(output.stdout.is_empty());
    }
    let output = Process::spawn(workspace.command("http://127.0.0.1:1").args([
        "upload",
        "unused",
        "--edge-recording-id",
        "rec",
    ]))
    .finish();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
}

#[test]
#[ignore = "requires loopback sockets"]
fn edge_transfer_reports_actual_status_and_rejects_unavailable_recordings() {
    let workspace = Workspace::new();
    for status in ["pending", "importing", "complete", "failed", "none"] {
        let body = format!(r#"{{"id":"rec_returned","importStatus":"{status}"}}"#);
        let server = Server::new(vec![Reply::json(
            "POST",
            "/v1/recordings/rec/import",
            &body,
        )]);
        let output =
            Process::spawn(
                workspace
                    .command(&server.url)
                    .args(["recordings", "transfer", "rec"]),
            )
            .finish();
        assert_success(&output);
        assert!(output.stdout.is_empty());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains("rec_returned"));
        assert!(message.contains(&format!("importStatus: {status}")));
        assert_eq!(message.contains("already available"), status == "complete");
        assert_eq!(
            message.contains("request accepted"),
            matches!(status, "pending" | "importing")
        );
        let requests = server.finish();
        let body = requests[0].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(body).unwrap(),
            serde_json::json!({})
        );
    }
    let mut reply = Reply::json(
        "POST",
        "/v1/recordings/rec/import",
        r#"{"message":"unavailable"}"#,
    );
    reply.status = 404;
    let server = Server::new(vec![reply]);
    let output =
        Process::spawn(
            workspace
                .command(&server.url)
                .args(["recordings", "transfer", "rec"]),
        )
        .finish();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to transfer edge recording"));
    server.finish();
}
