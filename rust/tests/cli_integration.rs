mod support;

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;

use foxglove_rust::format::{Channel, McapWriter, Message, RecordSink, Schema};
use support::{assert_success, Process, Reply, Server, Workspace};

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
