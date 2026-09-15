package compat

import (
	"net/http"
	"testing"
)

func TestRustBooleanFlagContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	for _, flag := range []struct {
		name string
		args []string
	}{
		{name: "absent"},
		{name: "bare", args: []string{"--include-schemas"}},
		{name: "true", args: []string{"--include-schemas=true"}},
		{name: "false", args: []string{"--include-schemas=false"}},
		{name: "uppercase", args: []string{"--include-schemas=TRUE"}},
		{name: "numeric", args: []string{"--include-schemas=0"}},
		{name: "repeated", args: []string{"--include-schemas", "--include-schemas=false"}},
	} {
		t.Run(flag.name, func(t *testing.T) {
			testCase := oracleCase{
				ID:   "topics-boolean-" + flag.name,
				Args: append([]string{"topics", "list", "--recording-id", "rec_fixture", "--format", "json"}, flag.args...),
				Plans: []responsePlan{{
					Method:  http.MethodGet,
					Path:    "/v1/data/topics",
					Body:    "[]",
					Headers: map[string]string{"Content-Type": "application/json"},
				}},
			}
			expected := runOracleCase(t, testCase, fixture)
			if expected.ExitCode != 0 || len(expected.Requests) != 1 {
				t.Fatalf("Go oracle failed: %+v", expected)
			}
			actual := runRustCaseWithFixture(t, testCase, fixture)
			assertCompatible(t, expected, actual)
		})
	}
}
