package compat

import (
	"bytes"
	"encoding/binary"
	"net/http"
	"testing"

	"github.com/foxglove/mcap/go/mcap"
)

func exportFixture(t *testing.T, messages []*mcap.Message, complete bool) []byte {
	t.Helper()
	var output bytes.Buffer
	writer, err := mcap.NewWriter(&output, &mcap.WriterOptions{})
	if err != nil {
		t.Fatal(err)
	}
	for _, err := range []error{
		writer.WriteHeader(&mcap.Header{}),
		writer.WriteSchema(&mcap.Schema{ID: 1, Name: "example/Message", Encoding: "ros1msg", Data: []byte("uint8 value\n")}),
		writer.WriteChannel(&mcap.Channel{ID: 1, SchemaID: 1, Topic: "/typed", MessageEncoding: "ros1"}),
		writer.WriteChannel(&mcap.Channel{ID: 2, SchemaID: 0, Topic: "/untyped", MessageEncoding: "json"}),
	} {
		if err != nil {
			t.Fatal(err)
		}
	}
	for _, message := range messages {
		if err := writer.WriteMessage(message); err != nil {
			t.Fatal(err)
		}
	}
	if complete {
		if err := writer.Close(); err != nil {
			t.Fatal(err)
		}
	}
	return output.Bytes()
}

func exportPlans(payload []byte) []responsePlan {
	return []responsePlan{
		{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/export"}`, Headers: map[string]string{"Content-Type": "application/json"}},
		{Method: http.MethodGet, Path: "/export", Body: string(payload)},
	}
}

func TestRustLargeMCAPExportPreservesServerBytes(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	// A valid schemaless JSON message whose MCAP record exceeds 64 MiB.
	data := bytes.Repeat([]byte(" "), 64*1024*1024)
	copy(data, `{"value":1}`)
	payload := exportFixture(t, []*mcap.Message{{ChannelID: 2, LogTime: 1_000_000_000, Data: data}}, true)
	testCase := oracleCase{
		Args:  []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
		Plans: exportPlans(payload), InitialFiles: map[string]string{"output.mcap": "existing destination"},
		OutputFiles: []string{"output.mcap"}, CaptureOutput: true,
	}
	actual := runRustCaseWithFixture(t, testCase, fixture)
	if actual.ExitCode != 0 || len(actual.Requests) != 2 || !bytes.Equal(actual.outputContents["output.mcap"], payload) {
		t.Fatalf("large export lost data: exit=%d requests=%d output bytes=%d want=%d stderr=%q", actual.ExitCode, len(actual.Requests), len(actual.outputContents["output.mcap"]), len(payload), actual.Stderr)
	}
}

func TestRustRecoveryPreservesSchemalessChannels(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	messages := []*mcap.Message{
		{ChannelID: 1, LogTime: 1_000_000_000, Data: []byte{1}},
		{ChannelID: 2, LogTime: 2_000_000_000, Data: []byte(`{"value":1}`)},
		{ChannelID: 1, LogTime: 3_000_000_000, Data: []byte{2}},
		{ChannelID: 2, LogTime: 4_000_000_000, Data: []byte(`{"value":2}`)},
	}
	partial := exportFixture(t, messages[:2], false)
	suffix := exportFixture(t, messages[1:], true)
	actual := runRustCaseWithFixture(t, oracleCase{
		Args:        []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
		Plans:       append(exportPlans(partial), exportPlans(suffix)...),
		OutputFiles: []string{"output.mcap"}, CaptureOutput: true,
	}, fixture)
	if actual.ExitCode != 0 || len(actual.Requests) != 4 {
		t.Fatalf("recovery failed: exit=%d requests=%d stderr=%q", actual.ExitCode, len(actual.Requests), actual.Stderr)
	}
	output := actual.outputContents["output.mcap"]
	reader, err := mcap.NewReader(bytes.NewReader(output))
	if err != nil {
		t.Fatal(err)
	}
	iterator, err := reader.Messages()
	if err != nil {
		t.Fatal(err)
	}
	// The Go CLI also offsets schema ID zero, so assert schema semantics
	// directly rather than treating Go's recovered output as correct.
	count := 0
	if err := mcap.Range(iterator, func(schema *mcap.Schema, channel *mcap.Channel, message *mcap.Message) error {
		if count >= len(messages) {
			t.Fatal("recovery duplicated messages")
		}
		want := messages[count]
		if message.LogTime != want.LogTime || !bytes.Equal(message.Data, want.Data) {
			t.Errorf("recovered message %d changed: %+v", count, message)
		}
		count++
		switch channel.Topic {
		case "/untyped":
			if channel.SchemaID != 0 || schema != nil {
				t.Errorf("schemaless channel acquired schema %d", channel.SchemaID)
			}
		case "/typed":
			if schema == nil || schema.Name != "example/Message" || string(schema.Data) != "uint8 value\n" {
				t.Errorf("typed channel lost its schema: %+v", schema)
			}
		}
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	if count != len(messages) {
		t.Fatalf("recovery lost messages: got %d want %d", count, len(messages))
	}
}

func TestRustInvalidMCAPRecordLengthPreservesDestination(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	payload := exportFixture(t, nil, false)
	// An impossible record length is a fatal format error, not a truncated
	// transfer to retry and eventually report as successful EOF.
	payload = append(payload, 5) // Message opcode
	payload = binary.LittleEndian.AppendUint64(payload, ^uint64(0))
	actual := runRustCaseWithFixture(t, oracleCase{
		Args:  []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
		Plans: exportPlans(payload), InitialFiles: map[string]string{"output.mcap": "existing destination"},
		OutputFiles: []string{"output.mcap"}, CaptureOutput: true,
	}, fixture)
	if actual.ExitCode != 1 || len(actual.Requests) != 2 || string(actual.outputContents["output.mcap"]) != "existing destination" {
		t.Fatalf("invalid record was not rejected safely: exit=%d requests=%d output=%q stderr=%q", actual.ExitCode, len(actual.Requests), actual.outputContents["output.mcap"], actual.Stderr)
	}
}
