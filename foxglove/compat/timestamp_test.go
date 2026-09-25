package compat

import (
	"net/http"
	"os"
	"path/filepath"
	"testing"
)

func TestRustISO8601TimestampCompatibility(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	payload, err := os.ReadFile(filepath.Join(moduleRoot, "testdata", "gps.mcap"))
	if err != nil {
		t.Fatal(err)
	}
	// Fractional precision intentionally differs from Go; Rust regression tests cover it.
	for _, timestamp := range []string{
		"2026-09-14", "2026-09-14T", "2026-09-14T12", "2026-09-14T12:34",
		"2026-09-14T12:34:56",
		"2026-09-14T12Z", "2026-09-14T12:34+05",
		"2026-09-14T12:34:56+0545", "2026-09-14T12-06:30",
		"+2026-9-14T12:34:56Z",
		"2026-09-14T1:2:3",
	} {
		t.Run(timestamp, func(t *testing.T) {
			for _, testCase := range []oracleCase{
				{
					ID:    "recordings",
					Args:  []string{"recordings", "list", "--format", "json", "--start", timestamp, "--end", timestamp},
					Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recordings", Body: "[]"}},
				},
				{
					ID:   "export",
					Args: []string{"data", "export", "--recording-id", "rec_fixture", "--start", timestamp, "--end", timestamp},
					Plans: []responsePlan{
						{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/streamed-export"}`},
						{Method: http.MethodGet, Path: "/streamed-export", Body: string(payload)},
					},
				},
			} {
				t.Run(testCase.ID, func(t *testing.T) {
					expected := runOracleCase(t, testCase, fixture)
					if expected.ExitCode != 0 || len(expected.Requests) != len(testCase.Plans) {
						t.Fatalf("Go did not accept timestamp: %+v", expected)
					}
					actual := runRustCaseWithFixture(t, testCase, fixture)
					assertCompatible(t, expected, actual)
				})
			}
		})
	}
}
