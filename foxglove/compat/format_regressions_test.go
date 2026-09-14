package compat

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/foxglove/go-rosbag"
	"github.com/foxglove/mcap/go/mcap"
)

func formatRegressionStreamPlans(payload []byte) []responsePlan {
	return []responsePlan{
		{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/streamed-export"}`, Headers: map[string]string{"Content-Type": "application/json"}},
		{Method: http.MethodGet, Path: "/streamed-export", Body: string(payload)},
	}
}

func TestRustROS1Uint64JSONCompatibility(t *testing.T) {
	var output bytes.Buffer
	writer, err := mcap.NewWriter(&output, &mcap.WriterOptions{})
	if err != nil {
		t.Fatal(err)
	}
	for _, err := range []error{
		writer.WriteHeader(&mcap.Header{}),
		writer.WriteSchema(&mcap.Schema{ID: 1, Name: "fixture/Message", Encoding: "ros1msg", Data: []byte("uint64 value\nuint64[] values\n")}),
		writer.WriteChannel(&mcap.Channel{ID: 1, SchemaID: 1, Topic: "/fixture", MessageEncoding: "ros1"}),
	} {
		if err != nil {
			t.Fatal(err)
		}
	}
	values := []uint64{0, 42, 9_007_199_254_740_993, ^uint64(0)}
	for i, value := range values {
		payload := binary.LittleEndian.AppendUint64(nil, value)
		payload = binary.LittleEndian.AppendUint32(payload, uint32(len(values)))
		for _, element := range values {
			payload = binary.LittleEndian.AppendUint64(payload, element)
		}
		if err := writer.WriteMessage(&mcap.Message{ChannelID: 1, Sequence: uint32(i), LogTime: uint64(i + 1), PublishTime: uint64(i + 1), Data: payload}); err != nil {
			t.Fatal(err)
		}
	}
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	fixture := newFixtureServer()
	defer fixture.close()
	testCase := oracleCase{
		Args:  []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json"},
		Plans: formatRegressionStreamPlans(output.Bytes()),
	}
	expected := runOracleCase(t, testCase, fixture)
	if expected.ExitCode != 0 {
		t.Fatalf("Go failed to export uint64 fixture: %+v", expected)
	}
	for _, value := range values {
		if !strings.Contains(expected.Stdout, fmt.Sprintf(`"value":%d`, value)) {
			t.Fatalf("Go did not emit numeric uint64 %d: %s", value, expected.Stdout)
		}
	}
	actual := runRustCaseWithFixture(t, testCase, fixture)
	assertCompatible(t, expected, actual)
}

func TestRustLargeROSBagExportCompatibility(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	for _, compression := range []string{"none", "lz4"} {
		t.Run(compression, func(t *testing.T) {
			path := filepath.Join(t.TempDir(), "large.bag")
			file, err := os.Create(path)
			if err != nil {
				t.Fatal(err)
			}
			defer func() { _ = file.Close() }()
			writer, err := rosbag.NewWriter(file, rosbag.WithCompression(compression))
			if err != nil {
				t.Fatal(err)
			}
			if err := writer.WriteConnection(&rosbag.Connection{Conn: 1, Topic: "/fixture", Data: rosbag.ConnectionHeader{
				Topic: "/fixture", Type: "fixture/Message", MD5Sum: "fixture", MessageDefinition: []byte("uint8[] data\n"),
			}}); err != nil {
				t.Fatal(err)
			}
			const length = 65 * 1024 * 1024
			payload := make([]byte, 4+length)
			binary.LittleEndian.PutUint32(payload, length)
			for i := 4; i < len(payload); i++ {
				payload[i] = byte(i)
			}
			if err := writer.WriteMessage(&rosbag.Message{Conn: 1, Time: 1, Data: payload}); err != nil {
				t.Fatal(err)
			}
			if err := writer.Close(); err != nil {
				t.Fatal(err)
			}
			if err := file.Close(); err != nil {
				t.Fatal(err)
			}
			bag, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			testCase := oracleCase{
				Args:          []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "bag1", "--output-file", "{TMP}/output.bag"},
				Plans:         formatRegressionStreamPlans(bag),
				OutputFiles:   []string{"output.bag"},
				CaptureOutput: true,
			}
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			for _, result := range []commandSnapshot{expected, actual} {
				if result.ExitCode != 0 || len(result.Requests) != 2 {
					t.Fatalf("large bag export failed: exit=%d requests=%d stderr=%s", result.ExitCode, len(result.Requests), result.Stderr)
				}
				assertEquivalentBag(t, bag, result.outputContents["output.bag"])
			}
			// Both implementations rebuild the bag index. Compare the decoded
			// records above and the public request/result contract here.
			expected.OutputFiles = nil
			actual.OutputFiles = nil
			assertCompatible(t, expected, actual)
		})
	}
}
