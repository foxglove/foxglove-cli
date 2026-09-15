package compat

import (
	"net/http"
	"testing"
)

func TestRustEmptyEnvironmentContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	cases := []oracleCase{
		{
			ID: "saved-project-fallback", Args: []string{"config", "get", "project-id"},
			Config: "default_project_id: prj_saved\n",
			Env:    map[string]string{"DEFAULT_PROJECT_ID": ""},
		},
		{
			ID: "missing-project-stays-unset", Args: []string{"config", "get", "project-id"},
			Env: map[string]string{"DEFAULT_PROJECT_ID": ""},
		},
		{
			ID: "saved-empty-project-stays-set", Args: []string{"config", "get", "project-id"},
			Config: "default_project_id: \"\"\n",
			Env:    map[string]string{"DEFAULT_PROJECT_ID": ""},
		},
		{
			ID: "nonempty-environment-overrides-file", Args: []string{"config", "get", "project-id"},
			Config: "default_project_id: prj_saved\n",
			Env:    map[string]string{"DEFAULT_PROJECT_ID": "prj_environment"},
		},
		{
			ID: "whitespace-is-an-explicit-value", Args: []string{"config", "get", "project-id"},
			Config: "default_project_id: prj_saved\n",
			Env:    map[string]string{"DEFAULT_PROJECT_ID": " "},
		},
	}
	for _, args := range [][]string{
		{"recordings", "list", "--format", "json"},
		{"recordings", "list", "--format", "json", "--project-id="},
	} {
		cases = append(cases, oracleCase{
			ID: "saved-request-settings-" + args[len(args)-1], Args: args,
			Config: "auth_type: 1\nbase_url: {BASE_URL}\nbearer_token: fixture-token\ndefault_project_id: prj_saved\n",
			Env: map[string]string{
				"AUTH_TYPE": "", "BASE_URL": "", "BEARER_TOKEN": "", "DEFAULT_PROJECT_ID": "",
			},
			Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recordings", Body: "[]", Headers: map[string]string{"Content-Type": "application/json"}}},
		})
	}
	for _, testCase := range cases {
		t.Run(testCase.ID, func(t *testing.T) {
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			assertCompatible(t, expected, actual)
		})
	}
}
