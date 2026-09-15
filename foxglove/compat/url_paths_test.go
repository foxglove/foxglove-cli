package compat

import (
	"net/http"
	"testing"
)

func TestRustSessionURLContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	sessionBody := `{"id":"ses_fixture","name":"Fixture session","projectId":"prj_default","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","recordings":[]}`
	for _, key := range []string{"drive#1", "drive?雪"} {
		for _, operation := range []struct {
			name   string
			args   []string
			method string
			body   string
		}{
			{name: "get", args: []string{"sessions", "get", key}, method: http.MethodGet, body: sessionBody},
			{name: "delete", args: []string{"sessions", "delete", key}, method: http.MethodDelete},
			{name: "recordings-list", args: []string{"sessions", "recordings", "list", key}, method: http.MethodGet, body: sessionBody},
			{name: "recordings-add", args: []string{"sessions", "recordings", "add", key, "rec_fixture"}, method: http.MethodPatch, body: sessionBody},
			{name: "recordings-remove", args: []string{"sessions", "recordings", "remove", key, "rec_fixture"}, method: http.MethodPatch, body: sessionBody},
		} {
			t.Run(operation.name+"/"+key, func(t *testing.T) {
				testCase := oracleCase{
					ID:     operation.name,
					Args:   operation.args,
					Config: "auth_type: 1\nbase_url: {BASE_URL}\nbearer_token: fixture-token\ndefault_project_id: prj_default\n",
					Plans:  []responsePlan{{Method: operation.method, Path: "/v1/sessions/" + key, Body: operation.body, Headers: jsonHeaders}},
				}
				expected := runOracleCase(t, testCase, fixture)
				if expected.ExitCode != 0 || len(expected.Requests) != 1 || expected.Requests[0].Path != "/v1/sessions/"+key {
					t.Fatalf("Go oracle did not target the intended session: %+v", expected)
				}
				actual := runRustCaseWithFixture(t, testCase, fixture)
				assertCompatible(t, expected, actual)
			})
		}
	}
}
