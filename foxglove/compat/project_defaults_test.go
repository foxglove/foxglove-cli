package compat

import (
	"net/http"
	"reflect"
	"testing"
)

func TestRustProjectDefaultContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	cases := []oracleCase{
		{ID: "devices-list", Args: []string{"devices", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: "[]", Headers: jsonHeaders}}},
		{ID: "recordings-list", Args: []string{"recordings", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recordings", Body: "[]", Headers: jsonHeaders}}},
		{ID: "sessions-list", Args: []string{"sessions", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions", Body: "[]", Headers: jsonHeaders}}},
		{ID: "pending-imports-list", Args: []string{"pending-imports", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/pending-imports", Body: "[]", Headers: jsonHeaders}}},
		{ID: "coverage-list", Args: []string{"data", "coverage", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/coverage", Body: "[]", Headers: jsonHeaders}}},
		{ID: "topics-list", Args: []string{"topics", "list", "--recording-id", "rec_fixture", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/topics", Body: "[]", Headers: jsonHeaders}}},
		{ID: "devices-add", Args: []string{"devices", "add", "--name", "Fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/devices", Body: `{"id":"dev_fixture","name":"Fixture"}`, Headers: jsonHeaders}}},
	}
	for _, baseCase := range cases {
		for _, flag := range []struct {
			name string
			args []string
		}{
			{name: "absent"},
			{name: "empty-equals", args: []string{"--project-id="}},
			{name: "empty-argument", args: []string{"--project-id", ""}},
			{name: "explicit", args: []string{"--project-id", "prj_explicit"}},
		} {
			t.Run(baseCase.ID+"/"+flag.name, func(t *testing.T) {
				testCase := baseCase
				testCase.Args = append(append([]string{}, baseCase.Args...), flag.args...)
				testCase.Config = "auth_type: 1\nbase_url: {BASE_URL}\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
				expected := runOracleCase(t, testCase, fixture)
				if expected.ExitCode != 0 {
					t.Fatalf("Go oracle failed: %+v", expected)
				}
				actual := runRustCaseWithFixture(t, testCase, fixture)
				// Go encodes the empty devices filter as an empty query key.
				// Both encodings omit projectId and leave the request unfiltered.
				if baseCase.ID == "devices-list" && (flag.name == "empty-equals" || flag.name == "empty-argument") {
					if len(expected.Requests) != 1 || len(actual.Requests) != 1 || expected.Requests[0].RawQuery != "=" || actual.Requests[0].RawQuery != "" {
						t.Fatalf("unexpected empty devices query: Go=%+v Rust=%+v", expected.Requests, actual.Requests)
					}
					expected.Requests[0].RawQuery = ""
				}
				assertCompatible(t, expected, actual)
			})
		}
	}
}

func TestLegacyUnscopedProjectArgsMatchesCommandsNotValues(t *testing.T) {
	for _, tc := range []struct {
		args        []string
		appendEmpty bool
	}{
		{[]string{"--debug", "data", "export"}, true},
		{[]string{"--config", "events list", "--client-id=x", "attachments", "list"}, true},
		{[]string{"--debug=false", "events", "add"}, true},
		{[]string{"devices", "list", "--project-id", "events list"}, false},
		{[]string{"devices", "add", "--name", "data export"}, false},
		{[]string{"events", "list", "--project-id=prj_one"}, false},
		{[]string{"--config", "attachments list", "version"}, false},
	} {
		expected := append([]string(nil), tc.args...)
		if tc.appendEmpty {
			expected = append(expected, "--project-id=")
		}
		if actual := legacyUnscopedProjectArgs(tc.args); !reflect.DeepEqual(actual, expected) {
			t.Fatalf("args %q: want %q, got %q", tc.args, expected, actual)
		}
	}
}
