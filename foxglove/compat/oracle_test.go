package compat

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"runtime"
	"sort"
	"strings"
	"sync"
	"testing"
)

const baselineVersion = "v1.0.33"

var (
	oracleBinary   string
	rustBinary     string
	moduleRoot     string
	repositoryRoot string
)

type requestSnapshot struct {
	Method     string            `json:"method"`
	Path       string            `json:"path"`
	RawQuery   string            `json:"rawQuery,omitempty"`
	Headers    map[string]string `json:"headers,omitempty"`
	Body       string            `json:"body,omitempty"`
	BodySHA256 string            `json:"bodySha256,omitempty"`
	BodyLength int               `json:"bodyLength,omitempty"`
}

type commandSnapshot struct {
	Args        []string          `json:"args"`
	ExitCode    int               `json:"exitCode"`
	Stdout      string            `json:"stdout"`
	Stderr      string            `json:"stderr"`
	Config      string            `json:"config,omitempty"`
	Requests    []requestSnapshot `json:"requests,omitempty"`
	OutputFiles map[string]string `json:"outputFiles,omitempty"`
}

type responsePlan struct {
	Method  string
	Path    string
	Status  int
	Body    string
	Headers map[string]string
}

type oracleCase struct {
	ID          string
	Args        []string
	Config      string
	Stdin       string
	Env         map[string]string
	Plans       []responsePlan
	OutputFiles []string
	HashStdout  bool
}

type fixtureServer struct {
	server   *httptest.Server
	mu       sync.Mutex
	plans    []responsePlan
	requests []requestSnapshot
}

func newFixtureServer() *fixtureServer {
	fixture := &fixtureServer{}
	fixture.server = httptest.NewServer(http.HandlerFunc(fixture.serveHTTP))
	return fixture
}

func (fixture *fixtureServer) close() {
	fixture.server.Close()
}

func (fixture *fixtureServer) reset(plans []responsePlan) {
	fixture.mu.Lock()
	defer fixture.mu.Unlock()
	fixture.plans = append([]responsePlan(nil), plans...)
	fixture.requests = nil
}

func (fixture *fixtureServer) snapshots() []requestSnapshot {
	fixture.mu.Lock()
	defer fixture.mu.Unlock()
	return append([]requestSnapshot(nil), fixture.requests...)
}

func (fixture *fixtureServer) remainingPlans() []responsePlan {
	fixture.mu.Lock()
	defer fixture.mu.Unlock()
	return append([]responsePlan(nil), fixture.plans...)
}

func (fixture *fixtureServer) serveHTTP(writer http.ResponseWriter, request *http.Request) {
	body, _ := io.ReadAll(request.Body)
	headers := map[string]string{}
	for _, name := range []string{"Authorization", "Content-Type", "User-Agent"} {
		if value := request.Header.Get(name); value != "" {
			headers[name] = value
		}
	}
	snapshot := requestSnapshot{
		Method:   request.Method,
		Path:     request.URL.Path,
		RawQuery: request.URL.RawQuery,
		Headers:  headers,
	}
	if len(body) > 0 {
		snapshot.BodyLength = len(body)
		if request.Header.Get("Content-Type") == "application/octet-stream" {
			digest := sha256.Sum256(body)
			snapshot.BodySHA256 = hex.EncodeToString(digest[:])
		} else {
			snapshot.Body = string(body)
		}
	}

	fixture.mu.Lock()
	fixture.requests = append(fixture.requests, snapshot)
	planIndex := -1
	for index, plan := range fixture.plans {
		if plan.Method == request.Method && plan.Path == request.URL.Path {
			planIndex = index
			break
		}
	}
	if planIndex < 0 {
		fixture.mu.Unlock()
		http.Error(writer, "unexpected fixture request", http.StatusInternalServerError)
		return
	}
	plan := fixture.plans[planIndex]
	fixture.plans = append(fixture.plans[:planIndex], fixture.plans[planIndex+1:]...)
	fixture.mu.Unlock()

	for name, value := range plan.Headers {
		writer.Header().Set(name, strings.ReplaceAll(value, "{BASE_URL}", fixture.server.URL))
	}
	status := plan.Status
	if status == 0 {
		status = http.StatusOK
	}
	writer.WriteHeader(status)
	_, _ = writer.Write([]byte(strings.ReplaceAll(plan.Body, "{BASE_URL}", fixture.server.URL)))
}

func TestMain(main *testing.M) {
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		fmt.Fprintln(os.Stderr, "failed to locate compatibility package")
		os.Exit(1)
	}
	moduleRoot = filepath.Dir(filepath.Dir(filename))
	repositoryRoot = filepath.Dir(moduleRoot)
	if err := verifyBaselineSource(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	temporaryDirectory, err := os.MkdirTemp("", "foxglove-oracle-")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer func() { _ = os.RemoveAll(temporaryDirectory) }()
	oracleBinary = filepath.Join(temporaryDirectory, "foxglove-oracle")
	if runtime.GOOS == "windows" {
		oracleBinary += ".exe"
	}
	build := exec.Command("go", "build", "-ldflags", "-X main.Version="+baselineVersion, "-o", oracleBinary, ".")
	build.Dir = moduleRoot
	build.Stdout = os.Stdout
	build.Stderr = os.Stderr
	if err := build.Run(); err != nil {
		fmt.Fprintln(os.Stderr, "failed to build oracle:", err)
		os.Exit(1)
	}
	rustProjectRoot := filepath.Join(repositoryRoot, "rust")
	rustBuild := exec.Command("cargo", "build", "--quiet", "--manifest-path", filepath.Join(rustProjectRoot, "Cargo.toml"))
	rustBuild.Dir = rustProjectRoot
	rustBuild.Stdout = os.Stdout
	rustBuild.Stderr = os.Stderr
	if err := rustBuild.Run(); err != nil {
		fmt.Fprintln(os.Stderr, "failed to build Rust compatibility binary:", err)
		os.Exit(1)
	}
	rustBinary = filepath.Join(rustProjectRoot, "target", "debug", "foxglove-rust")
	if runtime.GOOS == "windows" {
		rustBinary += ".exe"
	}
	os.Exit(main.Run())
}

func verifyBaselineSource() error {
	list := exec.Command("git", "ls-tree", "-r", "--name-only", baselineVersion, "--", "foxglove")
	list.Dir = repositoryRoot
	output, err := list.Output()
	if err != nil {
		return fmt.Errorf("list %s oracle source: %w", baselineVersion, err)
	}
	files := []string{}
	for _, name := range strings.Fields(string(output)) {
		if strings.HasSuffix(name, ".go") || strings.HasSuffix(name, "go.mod") || strings.HasSuffix(name, "go.sum") {
			files = append(files, name)
		}
	}
	arguments := append([]string{"diff", "--quiet", baselineVersion, "--"}, files...)
	diff := exec.Command("git", arguments...)
	diff.Dir = repositoryRoot
	if err := diff.Run(); err != nil {
		return fmt.Errorf("Go oracle source differs from %s; rebaseline explicitly before updating compatibility fixtures", baselineVersion)
	}
	return nil
}

func TestCommandSurfaceGolden(t *testing.T) {
	paths := [][]string{
		{},
		{"attachments"}, {"attachments", "list"}, {"attachments", "download"},
		{"auth"}, {"auth", "configure-api-key"}, {"auth", "info"}, {"auth", "login"},
		{"completion"}, {"completion", "bash"}, {"completion", "fish"}, {"completion", "powershell"}, {"completion", "zsh"},
		{"config"}, {"config", "get"}, {"config", "set"}, {"config", "unset"},
		{"data"}, {"data", "coverage"}, {"data", "coverage", "list"}, {"data", "export"}, {"data", "import"},
		{"data", "imports"}, {"data", "imports", "add"}, {"data", "imports", "list"},
		{"devices"}, {"devices", "add"}, {"devices", "edit"}, {"devices", "list"},
		{"event-types"}, {"event-types", "list"}, {"events"}, {"events", "add"}, {"events", "list"},
		{"extensions"}, {"extensions", "list"}, {"extensions", "publish"}, {"extensions", "unpublish"},
		{"help"}, {"pending-imports"}, {"pending-imports", "list"}, {"projects"}, {"projects", "list"},
		{"recordings"}, {"recordings", "delete"}, {"recordings", "list"},
		{"sessions"}, {"sessions", "add"}, {"sessions", "delete"}, {"sessions", "get"}, {"sessions", "list"},
		{"sessions", "recordings"}, {"sessions", "recordings", "add"}, {"sessions", "recordings", "list"}, {"sessions", "recordings", "remove"},
		{"topics"}, {"topics", "list"}, {"version"},
	}
	cases := make([]oracleCase, 0, len(paths))
	for _, path := range paths {
		id := "root"
		if len(path) > 0 {
			id = strings.Join(path, "-")
		}
		args := append(append([]string(nil), path...), "--help")
		cases = append(cases, oracleCase{ID: id, Args: args})
	}
	assertGolden(t, "command_surface.json", cases, nil)
}

func TestOfflineBehaviorGolden(t *testing.T) {
	baseConfig := "auth_type: 1\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
	cases := []oracleCase{
		{ID: "root-no-args"},
		{ID: "version", Args: []string{"version"}},
		{ID: "unknown-command", Args: []string{"not-a-command"}},
		{ID: "help-command", Args: []string{"help", "devices"}},
		{ID: "global-before-command", Args: []string{"--debug", "version"}},
		{ID: "global-after-command", Args: []string{"version", "--debug"}},
		{ID: "positional-help", Args: []string{"sessions", "get", "fixture", "--help"}},
		{ID: "missing-positional", Args: []string{"sessions", "get"}},
		{ID: "extra-positional", Args: []string{"config", "get", "project-id", "extra"}},
		{ID: "equals-flag", Args: []string{"events", "list", "--query-field=invalid"}},
		{ID: "format-conflict", Args: []string{"devices", "list", "--json", "--format", "csv"}},
		{ID: "session-key-requires-project", Args: []string{"attachments", "list", "--session-key", "fixture"}},
		{ID: "invalid-query-field", Args: []string{"events", "list", "--query-field", "invalid"}},
		{ID: "config-get", Args: []string{"config", "get", "project-id"}, Config: baseConfig},
		{ID: "config-get-environment", Args: []string{"config", "get", "project-id"}, Env: map[string]string{"DEFAULT_PROJECT_ID": "prj_environment"}},
		{ID: "config-set", Args: []string{"config", "set", "project-id", "prj_updated"}, Config: baseConfig},
		{ID: "config-unset", Args: []string{"config", "unset", "project-id"}, Config: baseConfig},
		{ID: "config-unset-missing", Args: []string{"config", "unset", "project-id"}},
		{ID: "configure-api-key", Args: []string{"auth", "configure-api-key", "--api-key", "fox_sk_fixture", "--base-url", "https://example.test"}, Config: baseConfig},
		{ID: "configure-api-key-interactive", Args: []string{"auth", "configure-api-key"}, Config: baseConfig, Stdin: "fox_sk_interactive\n"},
	}
	assertGolden(t, "offline_behavior.json", cases, nil)
}

// TestRustPhase1OfflineContract runs the new binary against the shared,
// reviewed fixtures. HTTP-backed cases intentionally remain Phase 2 work.
func TestRustPhase1OfflineContract(t *testing.T) {
	commandSurfacePath := filepath.Join(repositoryRoot, "compat", "goldens", baselineVersion, "command_surface.json")
	commandSurface := map[string]commandSnapshot{}
	bytes, err := os.ReadFile(commandSurfacePath)
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(bytes, &commandSurface); err != nil {
		t.Fatal(err)
	}
	for id, expected := range commandSurface {
		id, expected := id, expected
		t.Run("surface/"+id, func(t *testing.T) {
			actual := runRustCase(t, oracleCase{Args: expected.Args})
			if actual.ExitCode != expected.ExitCode || actual.Stdout != expected.Stdout || actual.Stderr != expected.Stderr {
				t.Fatalf("Rust command surface differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}

	baseConfig := "auth_type: 1\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
	cases := []oracleCase{
		{ID: "root-no-args"},
		{ID: "version", Args: []string{"version"}},
		{ID: "unknown-command", Args: []string{"not-a-command"}},
		{ID: "help-command", Args: []string{"help", "devices"}},
		{ID: "global-before-command", Args: []string{"--debug", "version"}},
		{ID: "global-after-command", Args: []string{"version", "--debug"}},
		{ID: "positional-help", Args: []string{"sessions", "get", "fixture", "--help"}},
		{ID: "missing-positional", Args: []string{"sessions", "get"}},
		{ID: "extra-positional", Args: []string{"config", "get", "project-id", "extra"}},
		{ID: "equals-flag", Args: []string{"events", "list", "--query-field=invalid"}},
		{ID: "format-conflict", Args: []string{"devices", "list", "--json", "--format", "csv"}},
		{ID: "session-key-requires-project", Args: []string{"attachments", "list", "--session-key", "fixture"}},
		{ID: "invalid-query-field", Args: []string{"events", "list", "--query-field", "invalid"}},
		{ID: "config-get", Args: []string{"config", "get", "project-id"}, Config: baseConfig},
		{ID: "config-get-environment", Args: []string{"config", "get", "project-id"}, Env: map[string]string{"DEFAULT_PROJECT_ID": "prj_environment"}},
		{ID: "config-set", Args: []string{"config", "set", "project-id", "prj_updated"}, Config: baseConfig},
		{ID: "config-unset", Args: []string{"config", "unset", "project-id"}, Config: baseConfig},
		{ID: "config-unset-missing", Args: []string{"config", "unset", "project-id"}},
		{ID: "configure-api-key", Args: []string{"auth", "configure-api-key", "--api-key", "fox_sk_fixture", "--base-url", "https://example.test"}, Config: baseConfig},
		{ID: "configure-api-key-interactive", Args: []string{"auth", "configure-api-key"}, Config: baseConfig, Stdin: "fox_sk_interactive\n"},
	}
	offlinePath := filepath.Join(repositoryRoot, "compat", "goldens", baselineVersion, "offline_behavior.json")
	offline := map[string]commandSnapshot{}
	bytes, err = os.ReadFile(offlinePath)
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(bytes, &offline); err != nil {
		t.Fatal(err)
	}
	for _, testCase := range cases {
		testCase := testCase
		t.Run("offline/"+testCase.ID, func(t *testing.T) {
			actual := runRustCase(t, testCase)
			expected := offline[testCase.ID]
			if !reflect.DeepEqual(expected, actual) {
				t.Fatalf("Rust offline contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}
}

func TestCompletionContractGolden(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	cases := []oracleCase{
		{ID: "static-command", Args: []string{"__completeNoDesc", "dev"}},
		{ID: "global-flag", Args: []string{"__completeNoDesc", "--d"}},
		{ID: "nested-command", Args: []string{"__completeNoDesc", "devices", "l"}},
		{ID: "command-flag", Args: []string{"__completeNoDesc", "devices", "list", "--f"}},
		{ID: "flag-value", Args: []string{"__completeNoDesc", "config", "get", ""}},
		{ID: "bash-script", Args: []string{"completion", "bash", "--no-descriptions"}, HashStdout: true},
		{ID: "fish-script", Args: []string{"completion", "fish", "--no-descriptions"}, HashStdout: true},
		{ID: "powershell-script", Args: []string{"completion", "powershell", "--no-descriptions"}, HashStdout: true},
		{ID: "zsh-script", Args: []string{"completion", "zsh", "--no-descriptions"}, HashStdout: true},
		{
			ID:   "dynamic-device-id",
			Args: []string{"__complete", "data", "coverage", "list", "--device-id", ""},
			Plans: []responsePlan{{
				Method:  http.MethodGet,
				Path:    "/v1/devices",
				Body:    `[{"id":"dev_fixture","name":"Fixture Robot","properties":{},"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:05Z","projectId":"prj_default"}]`,
				Headers: map[string]string{"Content-Type": "application/json"},
			}},
		},
	}
	assertGolden(t, "completion_contract.json", cases, fixture)
}

func TestRustPhase1CompletionContract(t *testing.T) {
	path := filepath.Join(repositoryRoot, "compat", "goldens", baselineVersion, "completion_contract.json")
	bytes, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	expected := map[string]commandSnapshot{}
	if err := json.Unmarshal(bytes, &expected); err != nil {
		t.Fatal(err)
	}
	cases := []oracleCase{
		{ID: "static-command", Args: []string{"__completeNoDesc", "dev"}},
		{ID: "global-flag", Args: []string{"__completeNoDesc", "--d"}},
		{ID: "nested-command", Args: []string{"__completeNoDesc", "devices", "l"}},
		{ID: "command-flag", Args: []string{"__completeNoDesc", "devices", "list", "--f"}},
		{ID: "flag-value", Args: []string{"__completeNoDesc", "config", "get", ""}},
		{ID: "bash-script", Args: []string{"completion", "bash", "--no-descriptions"}, HashStdout: true},
		{ID: "fish-script", Args: []string{"completion", "fish", "--no-descriptions"}, HashStdout: true},
		{ID: "powershell-script", Args: []string{"completion", "powershell", "--no-descriptions"}, HashStdout: true},
		{ID: "zsh-script", Args: []string{"completion", "zsh", "--no-descriptions"}, HashStdout: true},
	}
	for _, testCase := range cases {
		testCase := testCase
		t.Run(testCase.ID, func(t *testing.T) {
			actual := runRustCase(t, testCase)
			if !reflect.DeepEqual(expected[testCase.ID], actual) {
				t.Fatalf("Rust completion contract differs\n--- expected\n%+v\n--- actual\n%+v", expected[testCase.ID], actual)
			}
		})
	}
}

func TestWireContractGolden(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	listCases := []struct {
		id       string
		args     []string
		endpoint string
	}{
		{"devices-list", []string{"devices", "list", "--format", "json"}, "/v1/devices"},
		{"projects-list", []string{"projects", "list", "--format", "json"}, "/v1/projects"},
		{"imports-list", []string{"data", "imports", "list", "--format", "json"}, "/v1/data/imports"},
		{"coverage-list", []string{"data", "coverage", "list", "--format", "json"}, "/v1/data/coverage"},
		{"recordings-list", []string{"recordings", "list", "--format", "json"}, "/v1/recordings"},
		{"attachments-list", []string{"attachments", "list", "--format", "json"}, "/v1/recording-attachments"},
		{"sessions-list", []string{"sessions", "list", "--format", "json"}, "/v1/sessions"},
		{"events-list", []string{"events", "list", "--format", "json"}, "/v1/events"},
		{"event-types-list", []string{"event-types", "list", "--format", "json"}, "/v1/event-types"},
		{"pending-imports-list", []string{"pending-imports", "list", "--format", "json"}, "/v1/data/pending-imports"},
		{"topics-list", []string{"topics", "list", "--recording-id", "rec_fixture", "--format", "json"}, "/v1/data/topics"},
		{"extensions-list", []string{"extensions", "list", "--format", "json"}, "/v1/extensions"},
	}
	cases := make([]oracleCase, 0, len(listCases)+16)
	for _, item := range listCases {
		cases = append(cases, oracleCase{
			ID: item.id, Args: item.args,
			Plans: []responsePlan{{Method: http.MethodGet, Path: item.endpoint, Body: "[]", Headers: jsonHeaders}},
		})
	}
	sessionBody := `{"id":"ses_fixture","name":"Fixture session","key":"fixture-key","projectId":"prj_default","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","recordings":[]}`
	cases = append(cases,
		oracleCase{ID: "auth-info", Args: []string{"auth", "info"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/me", Body: `{"email":"fox@example.com","emailVerified":true,"orgId":"org_fixture","orgSlug":"fixture","admin":false}`, Headers: jsonHeaders}}},
		oracleCase{ID: "session-get", Args: []string{"sessions", "get", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions/ses_fixture", Body: sessionBody, Headers: jsonHeaders}}},
		oracleCase{ID: "session-recordings-list", Args: []string{"sessions", "recordings", "list", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions/ses_fixture", Body: sessionBody, Headers: jsonHeaders}}},
		oracleCase{ID: "device-add", Args: []string{"devices", "add", "--name", "Fixture"}, Plans: []responsePlan{
			{Method: http.MethodPost, Path: "/v1/devices", Body: `{"id":"dev_fixture","name":"Fixture","properties":{},"projectId":"prj_default"}`, Headers: jsonHeaders},
		}},
		oracleCase{ID: "device-edit", Args: []string{"devices", "edit", "dev_fixture", "--name", "Updated"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/devices/dev_fixture", Body: `{"id":"dev_fixture","name":"Updated","properties":{},"projectId":"prj_default"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "event-add", Args: []string{"events", "add", "--device-id", "dev_fixture", "--start", "2024-01-02T03:04:05Z", "--end", "2024-01-02T03:04:06Z", "--metadata", "key:value"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/events", Body: `{"id":"evt_fixture"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "session-add", Args: []string{"sessions", "add", "--name", "Fixture", "--device-id", "dev_fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/sessions", Body: `{"id":"ses_fixture","key":"fixture-key","projectId":"prj_default","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:05Z"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "session-recording-add", Args: []string{"sessions", "recordings", "add", "ses_fixture", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/sessions/ses_fixture", Body: `{"id":"ses_fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:05Z"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "session-recording-remove", Args: []string{"sessions", "recordings", "remove", "ses_fixture", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/sessions/ses_fixture", Body: `{"id":"ses_fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:05Z"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "session-delete", Args: []string{"sessions", "delete", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/sessions/ses_fixture"}}},
		oracleCase{ID: "recording-delete", Args: []string{"recordings", "delete", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/recordings/rec_fixture"}}},
		oracleCase{ID: "extension-unpublish", Args: []string{"extensions", "unpublish", "ext_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/extensions/ext_fixture"}}},
		oracleCase{ID: "edge-import", Args: []string{"data", "import", "ignored", "--edge-recording-id", "edge_fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/recordings/edge_fixture/import", Body: `{"id":"imp_fixture","importStatus":"pending"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "attachment-download", Args: []string{"attachments", "download", "att_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments/att_fixture/download", Body: "fixture attachment"}}},
		oracleCase{ID: "attachments-list-error", Args: []string{"attachments", "list"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments", Status: http.StatusInternalServerError, Body: `{"message":"fixture failure"}`, Headers: jsonHeaders}}},
		oracleCase{ID: "attachment-download-404", Args: []string{"attachments", "download", "missing"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments/missing/download", Status: http.StatusNotFound, Body: "fixture attachment missing"}}},
		oracleCase{ID: "devices-empty-csv", Args: []string{"devices", "list", "--format", "csv"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: "[]", Headers: jsonHeaders}}},
		oracleCase{ID: "export-initial-download-error", Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/data/stream", Status: http.StatusInternalServerError, Body: `{"message":"fixture export failure"}`, Headers: jsonHeaders}}, OutputFiles: []string{"output.mcap"}},
	)
	assertGolden(t, "wire_contract.json", cases, fixture)
}

func assertGolden(t *testing.T, filename string, cases []oracleCase, fixture *fixtureServer) {
	t.Helper()
	actual := map[string]commandSnapshot{}
	for _, testCase := range cases {
		testCase := testCase
		t.Run(testCase.ID, func(t *testing.T) {
			actual[testCase.ID] = runOracleCase(t, testCase, fixture)
		})
	}
	goldenPath := filepath.Join(repositoryRoot, "compat", "goldens", baselineVersion, filename)
	if os.Getenv("UPDATE_COMPAT_GOLDENS") == "1" {
		writeGolden(t, goldenPath, actual)
		return
	}
	expectedBytes, err := os.ReadFile(goldenPath)
	if err != nil {
		t.Fatalf("read golden %s: %v (run with UPDATE_COMPAT_GOLDENS=1 to create it)", goldenPath, err)
	}
	expected := map[string]commandSnapshot{}
	if err := json.Unmarshal(expectedBytes, &expected); err != nil {
		t.Fatalf("parse golden %s: %v", goldenPath, err)
	}
	if !reflect.DeepEqual(expected, actual) {
		expectedJSON, _ := json.MarshalIndent(expected, "", "  ")
		actualJSON, _ := json.MarshalIndent(actual, "", "  ")
		t.Fatalf("oracle contract differs from %s\n--- expected\n%s\n--- actual\n%s", goldenPath, expectedJSON, actualJSON)
	}
}

func runOracleCase(t *testing.T, testCase oracleCase, fixture *fixtureServer) commandSnapshot {
	t.Helper()
	temporaryDirectory := t.TempDir()
	homeDirectory := filepath.Join(temporaryDirectory, "home")
	if err := os.MkdirAll(homeDirectory, 0o755); err != nil {
		t.Fatal(err)
	}
	baseURL := ""
	if fixture != nil {
		baseURL = fixture.server.URL
		fixture.reset(testCase.Plans)
	}
	config := strings.ReplaceAll(testCase.Config, "{BASE_URL}", baseURL)
	if len(testCase.Plans) > 0 && config == "" {
		config = "auth_type: 1\nbase_url: " + baseURL + "\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
	}
	configPath := filepath.Join(homeDirectory, ".foxgloverc")
	if config != "" {
		if err := os.WriteFile(configPath, []byte(config), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	args := make([]string, len(testCase.Args))
	for index, arg := range testCase.Args {
		arg = strings.ReplaceAll(arg, "{TMP}", temporaryDirectory)
		arg = strings.ReplaceAll(arg, "{REPO}", repositoryRoot)
		args[index] = arg
	}
	command := exec.Command(oracleBinary, args...)
	command.Dir = temporaryDirectory
	command.Env = caseEnvironment(homeDirectory, testCase.Env)
	command.Stdin = strings.NewReader(testCase.Stdin)
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	command.Stdout = &stdout
	command.Stderr = &stderr
	err := command.Run()
	exitCode := 0
	if err != nil {
		if exitError, ok := err.(*exec.ExitError); ok {
			exitCode = exitError.ExitCode()
		} else {
			t.Fatalf("execute oracle: %v", err)
		}
	}
	replacements := map[string]string{
		repositoryRoot:     "{REPO}",
		temporaryDirectory: "{TMP}",
		homeDirectory:      "{HOME}",
		baseURL:            "{BASE_URL}",
	}
	snapshot := commandSnapshot{
		Args:     normalizeArgs(testCase.Args),
		ExitCode: exitCode,
		Stdout:   normalizeText(stdout.String(), replacements),
		Stderr:   normalizeText(stderr.String(), replacements),
	}
	if strings.Contains(snapshot.Stderr, "panic: runtime error: index out of range") {
		snapshot.Stderr = "{GO_PANIC_EMPTY_CSV}\n"
	}
	if bytes, err := os.ReadFile(configPath); err == nil {
		snapshot.Config = normalizeText(string(bytes), replacements)
	}
	if fixture != nil {
		snapshot.Requests = fixture.snapshots()
		if remaining := fixture.remainingPlans(); len(remaining) > 0 {
			t.Fatalf("oracle did not make %d planned request(s): %+v", len(remaining), remaining)
		}
	}
	if testCase.HashStdout {
		digest := sha256.Sum256([]byte(snapshot.Stdout))
		snapshot.Stdout = fmt.Sprintf("sha256:%s bytes:%d", hex.EncodeToString(digest[:]), len(snapshot.Stdout))
	}
	if len(testCase.OutputFiles) > 0 {
		snapshot.OutputFiles = map[string]string{}
		for _, name := range testCase.OutputFiles {
			path := filepath.Join(temporaryDirectory, name)
			bytes, err := os.ReadFile(path)
			if os.IsNotExist(err) {
				snapshot.OutputFiles[name] = "{ABSENT}"
				continue
			}
			if err != nil {
				t.Fatal(err)
			}
			digest := sha256.Sum256(bytes)
			snapshot.OutputFiles[name] = fmt.Sprintf("sha256:%s bytes:%d", hex.EncodeToString(digest[:]), len(bytes))
		}
	}
	return snapshot
}

func runRustCase(t *testing.T, testCase oracleCase) commandSnapshot {
	t.Helper()
	temporaryDirectory := t.TempDir()
	homeDirectory := filepath.Join(temporaryDirectory, "home")
	if err := os.MkdirAll(homeDirectory, 0o755); err != nil {
		t.Fatal(err)
	}
	configPath := filepath.Join(homeDirectory, ".foxgloverc")
	if testCase.Config != "" {
		if err := os.WriteFile(configPath, []byte(testCase.Config), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	command := exec.Command(rustBinary, testCase.Args...)
	command.Dir = temporaryDirectory
	command.Env = caseEnvironment(homeDirectory, testCase.Env)
	command.Stdin = strings.NewReader(testCase.Stdin)
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	command.Stdout = &stdout
	command.Stderr = &stderr
	err := command.Run()
	exitCode := 0
	if err != nil {
		if exitError, ok := err.(*exec.ExitError); ok {
			exitCode = exitError.ExitCode()
		} else {
			t.Fatalf("execute Rust binary: %v", err)
		}
	}
	replacements := map[string]string{
		repositoryRoot:     "{REPO}",
		temporaryDirectory: "{TMP}",
		homeDirectory:      "{HOME}",
	}
	snapshot := commandSnapshot{
		Args:     normalizeArgs(testCase.Args),
		ExitCode: exitCode,
		Stdout:   normalizeText(stdout.String(), replacements),
		Stderr:   normalizeText(stderr.String(), replacements),
	}
	if config, err := os.ReadFile(configPath); err == nil {
		snapshot.Config = normalizeText(string(config), replacements)
		if testCase.Config != "" && runtime.GOOS != "windows" {
			info, err := os.Stat(configPath)
			if err != nil {
				t.Fatal(err)
			}
			if actual := info.Mode().Perm(); actual != 0o600 {
				t.Fatalf("Rust config permissions changed: got %04o, want 0600", actual)
			}
		}
	}
	if testCase.HashStdout {
		digest := sha256.Sum256([]byte(snapshot.Stdout))
		snapshot.Stdout = fmt.Sprintf("sha256:%s bytes:%d", hex.EncodeToString(digest[:]), len(snapshot.Stdout))
	}
	return snapshot
}

func caseEnvironment(home string, additions map[string]string) []string {
	environment := isolatedEnvironment(home)
	keys := make([]string, 0, len(additions))
	for key := range additions {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		environment = append(environment, key+"="+additions[key])
	}
	return environment
}

func isolatedEnvironment(home string) []string {
	blocked := map[string]bool{
		"AUTH_TYPE": true, "BASE_URL": true, "BEARER_TOKEN": true,
		"DEFAULT_PROJECT_ID": true, "HOME": true, "USERPROFILE": true,
	}
	environment := make([]string, 0, len(os.Environ())+3)
	for _, entry := range os.Environ() {
		name := strings.SplitN(entry, "=", 2)[0]
		if !blocked[name] {
			environment = append(environment, entry)
		}
	}
	environment = append(environment, "HOME="+home, "USERPROFILE="+home, "NO_COLOR=1")
	return environment
}

func normalizeArgs(args []string) []string {
	result := append([]string(nil), args...)
	for index, arg := range result {
		result[index] = strings.ReplaceAll(arg, repositoryRoot, "{REPO}")
	}
	return result
}

func normalizeText(value string, replacements map[string]string) string {
	value = strings.ReplaceAll(value, "\r\n", "\n")
	keys := make([]string, 0, len(replacements))
	for from := range replacements {
		if from != "" {
			keys = append(keys, from)
		}
	}
	sort.Slice(keys, func(i, j int) bool { return len(keys[i]) > len(keys[j]) })
	for _, from := range keys {
		value = strings.ReplaceAll(value, from, replacements[from])
	}
	exportPartial := regexp.MustCompile(`\./export[0-9]+/export[0-9]+`)
	value = exportPartial.ReplaceAllString(value, "{EXPORT_PARTIAL}")
	return value
}

func writeGolden(t *testing.T, path string, value any) {
	t.Helper()
	bytes, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	bytes = append(bytes, '\n')
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, bytes, 0o644); err != nil {
		t.Fatal(err)
	}
}
