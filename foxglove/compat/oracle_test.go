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
	"time"

	"github.com/foxglove/mcap/go/mcap"
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
	Method        string
	Path          string
	Status        int
	Body          string
	TruncateBytes int
	DelayMillis   int
	Headers       map[string]string
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
	if plan.DelayMillis > 0 {
		time.Sleep(time.Duration(plan.DelayMillis) * time.Millisecond)
	}
	response := []byte(strings.ReplaceAll(plan.Body, "{BASE_URL}", fixture.server.URL))
	if plan.TruncateBytes > 0 && plan.TruncateBytes < len(response) {
		response = response[:len(response)-plan.TruncateBytes]
	}
	_, _ = writer.Write(response)
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
	rustBuild := exec.Command("cargo", "build", "--quiet", "--features", "compat-test", "--manifest-path", filepath.Join(rustProjectRoot, "Cargo.toml"))
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

func TestRustPhase3ReadWireContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	cases := []oracleCase{
		{ID: "devices-list", Args: []string{"devices", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: "[]", Headers: jsonHeaders}}},
		{ID: "projects-list", Args: []string{"projects", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/projects", Body: "[]", Headers: jsonHeaders}}},
		{ID: "imports-list", Args: []string{"data", "imports", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/imports", Body: "[]", Headers: jsonHeaders}}},
		{ID: "coverage-list", Args: []string{"data", "coverage", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/coverage", Body: "[]", Headers: jsonHeaders}}},
		{ID: "recordings-list", Args: []string{"recordings", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recordings", Body: "[]", Headers: jsonHeaders}}},
		{ID: "attachments-list", Args: []string{"attachments", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments", Body: "[]", Headers: jsonHeaders}}},
		{ID: "sessions-list", Args: []string{"sessions", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions", Body: "[]", Headers: jsonHeaders}}},
		{ID: "events-list", Args: []string{"events", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/events", Body: "[]", Headers: jsonHeaders}}},
		{ID: "event-types-list", Args: []string{"event-types", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/event-types", Body: "[]", Headers: jsonHeaders}}},
		{ID: "pending-imports-list", Args: []string{"pending-imports", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/pending-imports", Body: "[]", Headers: jsonHeaders}}},
		{ID: "topics-list", Args: []string{"topics", "list", "--recording-id", "rec_fixture", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/topics", Body: "[]", Headers: jsonHeaders}}},
		{ID: "extensions-list", Args: []string{"extensions", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/extensions", Body: "[]", Headers: jsonHeaders}}},
		{ID: "session-get", Args: []string{"sessions", "get", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions/ses_fixture", Body: `{"id":"ses_fixture","name":"Fixture session","key":"fixture-key","projectId":"prj_default","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","recordings":[]}`, Headers: jsonHeaders}}},
		{ID: "session-recordings-list", Args: []string{"sessions", "recordings", "list", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/sessions/ses_fixture", Body: `{"id":"ses_fixture","name":"Fixture session","key":"fixture-key","projectId":"prj_default","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","recordings":[]}`, Headers: jsonHeaders}}},
	}
	for _, testCase := range cases {
		t.Run(testCase.ID, func(t *testing.T) {
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			if !reflect.DeepEqual(expected, actual) {
				t.Fatalf("Rust Phase 3 read contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}

	richCases := []oracleCase{
		{ID: "device-rich", Args: []string{"devices", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: `[{"id":"dev_fixture","name":"Fixture","properties":{"site":"lab"},"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","projectId":"prj_default"}]`, Headers: jsonHeaders}}},
		{ID: "project-rich", Args: []string{"projects", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/projects", Body: `[{"id":"prj_default","name":"Fixture","orgMemberCount":3,"lastSeenAt":"2024-01-02T03:04:05Z"}]`, Headers: jsonHeaders}}},
		{ID: "import-rich", Args: []string{"data", "imports", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/imports", Body: `[{"id":"imp_fixture","deviceId":"dev_fixture","filename":"fixture.mcap","importTime":"2024-01-02T03:04:05Z","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","inputType":"mcap","outputType":"mcap","inputSize":10,"totalOutputSize":20}]`, Headers: jsonHeaders}}},
		{ID: "coverage-rich", Args: []string{"data", "coverage", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/coverage", Body: `[{"deviceId":"dev_fixture","device":{"id":"dev_fixture","name":"Fixture"},"start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","status":"complete"}]`, Headers: jsonHeaders}}},
		{ID: "recording-rich", Args: []string{"recordings", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recordings", Body: `[{"id":"rec_fixture","path":"fixture.mcap","size":1024,"messageCount":2,"createdAt":"2024-01-02T03:04:05Z","importedAt":"2024-01-02T03:04:06Z","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","importStatus":"completed","site":{"id":"site_fixture","name":"Primary"},"edgeSite":{"id":"edge_fixture","name":"Edge"},"device":{"id":"dev_fixture","name":"Fixture"},"metadata":[{"name":"source","metadata":{"robot":"one"}}],"key":"recording-key","projectId":"prj_default"}]`, Headers: jsonHeaders}}},
		{ID: "attachment-rich", Args: []string{"attachments", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments", Body: `[{"id":"att_fixture","recordingId":"rec_fixture","siteId":"site_fixture","name":"map.bin","mediaType":"application/octet-stream","logTime":"2024-01-02T03:04:05Z","createTime":"2024-01-02T03:04:06Z","crc":7,"size":8,"fingerprint":"abc"}]`, Headers: jsonHeaders}}},
		{ID: "event-rich", Args: []string{"events", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/events", Body: `[{"createdAt":"2024-01-02T03:04:05Z","device":{"id":"dev_fixture","name":"Fixture"},"end":"2024-01-02T03:04:06Z","eventTypeId":"evtt_fixture","id":"evt_fixture","metadata":{"key":"value"},"properties":{"count":1},"start":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"}]`, Headers: jsonHeaders}}},
		{ID: "event-type-rich", Args: []string{"event-types", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/event-types", Body: `[{"colorName":"blue","createdAt":"2024-01-02T03:04:05Z","id":"evtt_fixture","name":"Fixture","properties":[{"key":"mode","label":"Mode","required":true,"values":["a"],"valueType":"string"}],"updatedAt":"2024-01-02T03:04:06Z"}]`, Headers: jsonHeaders}}},
		{ID: "pending-import-rich", Args: []string{"pending-imports", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/pending-imports", Body: `[{"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","orgId":"org_fixture","filename":"fixture.mcap","pipelineStage":"parse","requestId":"req_fixture","deviceId":"dev_fixture","deviceName":"Fixture","importId":"imp_fixture","siteId":"site_fixture","projectId":"prj_default","status":"pending","error":""}]`, Headers: jsonHeaders}}},
		{ID: "topic-rich", Args: []string{"topics", "list", "--recording-id", "rec_fixture", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/data/topics", Body: `[{"encoding":"cdr","schema":"uint32 value","schemaEncoding":"ros2msg","schemaName":"std_msgs/UInt32","topic":"/count","version":"1"}]`, Headers: jsonHeaders}}},
		{ID: "extension-rich", Args: []string{"extensions", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/extensions", Body: `[{"id":"ext_fixture","name":"demo","publisher":"foxglove","displayName":"Demo","description":null,"activeVersion":null,"sha256Sum":null}]`, Headers: jsonHeaders}}},
	}
	for _, testCase := range richCases {
		t.Run(testCase.ID, func(t *testing.T) {
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			if !reflect.DeepEqual(expected, actual) {
				t.Fatalf("Rust Phase 3 rich read contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}

	t.Run("attachments-list-error-approved-delta", func(t *testing.T) {
		testCase := oracleCase{ID: "attachments-list-error", Args: []string{"attachments", "list"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments", Status: http.StatusInternalServerError, Body: `{"message":"fixture failure"}`, Headers: jsonHeaders}}}
		expected := runRustCaseWithFixture(t, testCase, fixture)
		if expected.ExitCode != 1 || expected.Stdout != "" || expected.Stderr != "Failed to list attachments: fixture failure\n" {
			t.Fatalf("unexpected approved attachment error delta: %+v", expected)
		}
	})

	t.Run("devices-empty-csv-approved-delta", func(t *testing.T) {
		testCase := oracleCase{ID: "devices-empty-csv", Args: []string{"devices", "list", "--format", "csv"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: "[]", Headers: jsonHeaders}}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.Stdout != "ID,Name,Custom Properties,Created At,Updated At,Project ID\n" || actual.Stderr != "" {
			t.Fatalf("unexpected approved empty CSV delta: %+v", actual)
		}
	})
}

func TestRustPhase4MutationWireContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	cases := []oracleCase{
		{ID: "auth-info-api-key", Args: []string{"auth", "info"}, Config: "auth_type: 2\nbearer_token: fox_sk_fixture\n"},
		{ID: "auth-info-session", Args: []string{"auth", "info"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/me", Body: `{"email":"fixture@example.com","emailVerified":true,"orgId":"org_fixture","orgSlug":"fixture","admin":false}`, Headers: jsonHeaders}}},
		{ID: "attachment-download", Args: []string{"attachments", "download", "att_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments/att_fixture/download", Body: "fixture attachment", Headers: map[string]string{"Content-Type": "application/octet-stream"}}}},
		{ID: "extension-publish", Args: []string{"extensions", "publish", "{REPO}/foxglove/testdata/fg.mock-0.0.0.foxe"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/extension-upload", Body: "", Headers: jsonHeaders}}},
		{ID: "data-import", Args: []string{"data", "import", "{REPO}/foxglove/testdata/gps.bag", "--device-id", "dev_fixture"}, Plans: []responsePlan{
			{Method: http.MethodPost, Path: "/v1/data/upload", Body: `{"link":"{BASE_URL}/storage/import-fixture"}`, Headers: jsonHeaders},
			{Method: http.MethodPut, Path: "/storage/import-fixture", Body: "", Headers: map[string]string{"Content-Type": "application/octet-stream"}},
		}},
		{ID: "device-add", Args: []string{"devices", "add", "--name", "Fixture", "--property", "mode:auto", "--property", "count:7", "--property", "enabled:1"}, Plans: []responsePlan{
			{Method: http.MethodGet, Path: "/v1/custom-properties", Body: `[ {"key":"mode","valueType":"enum","values":["auto"]}, {"key":"count","valueType":"number","values":[]}, {"key":"enabled","valueType":"boolean","values":[]} ]`, Headers: jsonHeaders},
			{Method: http.MethodPost, Path: "/v1/devices", Body: `{"id":"dev_fixture","name":"Fixture"}`, Headers: jsonHeaders},
		}},
		{ID: "device-edit", Args: []string{"devices", "edit", "dev_fixture", "--name", "Updated"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/devices/dev_fixture", Body: `{"id":"dev_fixture","name":"Updated"}`, Headers: jsonHeaders}}},
		{ID: "event-add", Args: []string{"events", "add", "--device-id", "dev_fixture", "--start", "2024-01-02T03:04:05Z", "--end", "2024-01-02T03:04:06Z", "--event-type-id", "evtt_fixture", "--metadata", "mode:auto", "--metadata", "note:fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/events", Body: `{"id":"evt_fixture"}`, Headers: jsonHeaders}}},
		{ID: "edge-recording-import", Args: []string{"data", "imports", "add", "fixture.mcap", "--edge-recording-id", "edge_fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/recordings/edge_fixture/import", Body: `{"id":"imp_fixture"}`, Headers: jsonHeaders}}},
		{ID: "extension-unpublish", Args: []string{"extensions", "unpublish", "ext_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/extensions/ext_fixture", Body: "", Headers: jsonHeaders}}},
		{ID: "extension-unpublish-not-found", Args: []string{"extensions", "unpublish", "ext_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/extensions/ext_fixture", Status: http.StatusNotFound, Body: "", Headers: jsonHeaders}}},
		{ID: "recording-delete", Args: []string{"recordings", "delete", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/recordings/rec_fixture", Body: "", Headers: jsonHeaders}}},
		{ID: "recording-delete-not-found", Args: []string{"recordings", "delete", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/recordings/rec_fixture", Status: http.StatusNotFound, Body: "", Headers: jsonHeaders}}},
		{ID: "session-add", Args: []string{"sessions", "add", "--name", "Fixture session", "--device-id", "dev_fixture"}, Plans: []responsePlan{{Method: http.MethodPost, Path: "/v1/sessions", Body: `{"id":"ses_fixture","key":"fixture-key"}`, Headers: jsonHeaders}}},
		{ID: "session-delete", Args: []string{"sessions", "delete", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/sessions/ses_fixture", Body: "", Headers: jsonHeaders}}},
		{ID: "session-delete-not-found", Args: []string{"sessions", "delete", "ses_fixture"}, Plans: []responsePlan{{Method: http.MethodDelete, Path: "/v1/sessions/ses_fixture", Status: http.StatusNotFound, Body: "", Headers: jsonHeaders}}},
		{ID: "session-recording-add", Args: []string{"sessions", "recordings", "add", "ses_fixture", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/sessions/ses_fixture", Body: `{}`, Headers: jsonHeaders}}},
		{ID: "session-recording-remove", Args: []string{"sessions", "recordings", "remove", "ses_fixture", "rec_fixture"}, Plans: []responsePlan{{Method: http.MethodPatch, Path: "/v1/sessions/ses_fixture", Body: `{}`, Headers: jsonHeaders}}},
	}
	for _, testCase := range cases {
		t.Run(testCase.ID, func(t *testing.T) {
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			if testCase.ID == "extension-publish" {
				expected.Stderr = stripProgressPrefix(expected.Stderr, "Extension published\n")
				actual.Stderr = stripProgressPrefix(actual.Stderr, "Extension published\n")
			}
			if testCase.ID == "data-import" {
				expected.Stderr = ""
				actual.Stderr = ""
			}
			if !reflect.DeepEqual(expected, actual) {
				t.Fatalf("Rust Phase 4 mutation contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}

	t.Run("attachment-download-404-approved-delta", func(t *testing.T) {
		testCase := oracleCase{ID: "attachment-download-404", Args: []string{"attachments", "download", "att_fixture"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/recording-attachments/att_fixture/download", Status: http.StatusNotFound, Body: `{"message":"missing"}`, Headers: jsonHeaders}}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || actual.Stdout != "" || actual.Stderr != "Failed to fetch attachment: not found\n" {
			t.Fatalf("unexpected approved attachment download delta: %+v", actual)
		}
	})

	t.Run("data-import-requires-http-200", func(t *testing.T) {
		testCase := oracleCase{ID: "data-import-created", Args: []string{"data", "import", "{REPO}/foxglove/testdata/gps.bag", "--device-id", "dev_fixture"}, Plans: []responsePlan{
			{Method: http.MethodPost, Path: "/v1/data/upload", Body: `{"link":"{BASE_URL}/storage/import-created"}`, Headers: jsonHeaders},
			{Method: http.MethodPut, Path: "/storage/import-created", Status: http.StatusCreated, Body: "", Headers: map[string]string{"Content-Type": "application/octet-stream"}},
		}}
		expected := runOracleCase(t, testCase, fixture)
		actual := runRustCaseWithFixture(t, testCase, fixture)
		expected.Stderr = stripProgressToError(expected.Stderr, "Failed to import")
		actual.Stderr = stripProgressToError(actual.Stderr, "Failed to import")
		if !reflect.DeepEqual(expected, actual) {
			t.Fatalf("Rust upload status handling differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
		}
	})
}

func TestRustPhase6DirectExportContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()

	streamPlans := func(payload []byte) []responsePlan {
		t.Helper()
		return []responsePlan{
			{
				Method:  http.MethodPost,
				Path:    "/v1/data/stream",
				Body:    `{"link":"{BASE_URL}/streamed-export"}`,
				Headers: map[string]string{"Content-Type": "application/json"},
			},
			{Method: http.MethodGet, Path: "/streamed-export", Body: string(payload)},
		}
	}
	readFixture := func(filename string) []byte {
		t.Helper()
		bytes, err := os.ReadFile(filepath.Join(moduleRoot, "testdata", filename))
		if err != nil {
			t.Fatal(err)
		}
		return bytes
	}
	protobufMcap := func() []byte {
		var output bytes.Buffer
		writer, err := mcap.NewWriter(&output, &mcap.WriterOptions{})
		if err != nil {
			t.Fatal(err)
		}
		if err := writer.WriteHeader(&mcap.Header{}); err != nil {
			t.Fatal(err)
		}
		// FileDescriptorSet for `syntax = "proto3"; package fixture;
		// message Message { uint32 value = 1; }`.
		descriptor := []byte{0x0a, 0x38, 0x0a, 0x0d, 'f', 'i', 'x', 't', 'u', 'r', 'e', '.', 'p', 'r', 'o', 't', 'o', 0x12, 0x07, 'f', 'i', 'x', 't', 'u', 'r', 'e', 0x22, 0x16, 0x0a, 0x07, 'M', 'e', 's', 's', 'a', 'g', 'e', 0x12, 0x0d, 0x0a, 0x05, 'v', 'a', 'l', 'u', 'e', 0x18, 0x01, 0x20, 0x01, 0x28, 0x0d, 0x62, 0x06, 'p', 'r', 'o', 't', 'o', '3'}
		if err := writer.WriteSchema(&mcap.Schema{ID: 1, Name: "fixture.Message", Encoding: "protobuf", Data: descriptor}); err != nil {
			t.Fatal(err)
		}
		if err := writer.WriteChannel(&mcap.Channel{ID: 1, SchemaID: 1, Topic: "/fixture", MessageEncoding: "protobuf"}); err != nil {
			t.Fatal(err)
		}
		if err := writer.WriteMessage(&mcap.Message{ChannelID: 1, LogTime: 4_000_000_005, PublishTime: 6_000_000_007, Data: []byte{0x08, 0x2a}}); err != nil {
			t.Fatal(err)
		}
		if err := writer.Close(); err != nil {
			t.Fatal(err)
		}
		return output.Bytes()
	}

	cases := []struct {
		name    string
		args    []string
		payload []byte
	}{
		{"mcap", []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "mcap0"}, readFixture("gps.mcap")},
		{"bag", []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "bag1"}, readFixture("gps.bag")},
		{"json-ros1", []string{"data", "export", "--recording-id", "rec_fixture", "--json"}, readFixture("gps.mcap")},
		{"json-protobuf", []string{"data", "export", "--recording-id", "rec_fixture", "--json"}, protobufMcap()},
	}
	for _, testCase := range cases {
		testCase := testCase
		t.Run(testCase.name, func(t *testing.T) {
			fixtureCase := oracleCase{
				Args:       testCase.args,
				Plans:      streamPlans(testCase.payload),
				HashStdout: true,
			}
			expected := runOracleCase(t, fixtureCase, fixture)
			actual := runRustCaseWithFixture(t, fixtureCase, fixture)
			if expected.ExitCode != actual.ExitCode || expected.Stdout != actual.Stdout || !reflect.DeepEqual(expected.Requests, actual.Requests) {
				t.Fatalf("Rust Phase 6 direct export differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}
}

func TestRustPhase6ExportErrorsAndOptions(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	cases := []oracleCase{
		{
			ID:   "invalid-output-format",
			Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "mcap"},
		},
		{
			ID:   "initial-stream-error",
			Args: []string{"data", "export", "--recording-id", "rec_fixture"},
			Plans: []responsePlan{{
				Method:  http.MethodPost,
				Path:    "/v1/data/stream",
				Status:  http.StatusInternalServerError,
				Body:    `{"message":"fixture export failure"}`,
				Headers: jsonHeaders,
			}},
		},
		{
			ID: "all-stream-options",
			Args: []string{
				"data", "export", "--device-id", "dev_fixture",
				"--start", "2024-01-02T03:04:05.123456789Z",
				"--end", "2024-01-02T04:05:06.987654321Z",
				"--output-format", "mcap0", "--compression", "zstd",
				"--include-attachments", "--topics", "/one,,/two",
				"--replay-policy", "lastPerChannel", "--replay-lookback-seconds", "2.5",
			},
			Plans: []responsePlan{
				{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/empty-stream"}`, Headers: jsonHeaders},
				{Method: http.MethodGet, Path: "/empty-stream"},
			},
		},
	}
	for _, testCase := range cases {
		testCase := testCase
		t.Run(testCase.ID, func(t *testing.T) {
			expected := runOracleCase(t, testCase, fixture)
			actual := runRustCaseWithFixture(t, testCase, fixture)
			if expected.ExitCode != actual.ExitCode || expected.Stdout != actual.Stdout || expected.Stderr != actual.Stderr || !reflect.DeepEqual(expected.Requests, actual.Requests) {
				t.Fatalf("Rust Phase 6 export contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
			}
		})
	}
}

func TestRustPhase7ResilientExportContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	readFixture := func(filename string) []byte {
		t.Helper()
		payload, err := os.ReadFile(filepath.Join(moduleRoot, "testdata", filename))
		if err != nil {
			t.Fatal(err)
		}
		return payload
	}
	streamPlans := func(payload []byte, truncate int) []responsePlan {
		return []responsePlan{
			{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/streamed-export"}`, Headers: jsonHeaders},
			{Method: http.MethodGet, Path: "/streamed-export", Body: string(payload), TruncateBytes: truncate},
		}
	}

	t.Run("complete-mcap-preserves-server-bytes", func(t *testing.T) {
		testCase := oracleCase{
			Args:        []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
			Plans:       streamPlans(readFixture("gps.mcap"), 0),
			OutputFiles: []string{"output.mcap"},
		}
		expected := runOracleCase(t, testCase, fixture)
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if expected.ExitCode != actual.ExitCode || expected.Stdout != actual.Stdout || expected.Stderr != actual.Stderr || !reflect.DeepEqual(expected.OutputFiles, actual.OutputFiles) || !reflect.DeepEqual(expected.Requests, actual.Requests) {
			t.Fatalf("Rust Phase 7 complete MCAP export differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
		}
	})

	t.Run("truncated-mcap-retries-and-produces-output", func(t *testing.T) {
		payload := readFixture("gps.mcap")
		testCase := oracleCase{
			Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
			Plans: append(
				streamPlans(payload, 4),
				streamPlans(payload, 0)...,
			),
			OutputFiles: []string{"output.mcap"},
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.OutputFiles["output.mcap"] == "{ABSENT}" || len(actual.Requests) != 4 || !strings.Contains(actual.Requests[2].Body, `"start"`) {
			t.Fatalf("Rust did not recover and retry a truncated MCAP export: %+v", actual)
		}
	})

	t.Run("bag-retries-until-the-boundary-stops-advancing", func(t *testing.T) {
		payload := readFixture("gps.bag")
		testCase := oracleCase{
			Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "bag1", "--output-file", "{TMP}/output.bag"},
			Plans: append(append(
				streamPlans(payload, 0),
				streamPlans(payload, 0)...,
			), streamPlans(payload, 0)...),
			OutputFiles: []string{"output.bag"},
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.OutputFiles["output.bag"] == "{ABSENT}" || len(actual.Requests) != 6 {
			t.Fatalf("Rust did not complete the bounded ROS bag recovery loop: %+v", actual)
		}
	})

	t.Run("cancellation-preserves-destination-and-cleans-staging", func(t *testing.T) {
		temporaryDirectory := t.TempDir()
		homeDirectory := filepath.Join(temporaryDirectory, "home")
		if err := os.MkdirAll(homeDirectory, 0o755); err != nil {
			t.Fatal(err)
		}
		config := "auth_type: 1\nbase_url: " + fixture.server.URL + "\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
		if err := os.WriteFile(filepath.Join(homeDirectory, ".foxgloverc"), []byte(config), 0o600); err != nil {
			t.Fatal(err)
		}
		output := filepath.Join(temporaryDirectory, "output.mcap")
		if err := os.WriteFile(output, []byte("original destination"), 0o600); err != nil {
			t.Fatal(err)
		}
		fixture.reset([]responsePlan{
			{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/slow-export"}`, Headers: jsonHeaders},
			{Method: http.MethodGet, Path: "/slow-export", Body: string(readFixture("gps.mcap")), DelayMillis: 1_000},
		})
		command := exec.Command(rustBinary, "data", "export", "--recording-id", "rec_fixture", "--output-file", output)
		command.Dir = temporaryDirectory
		command.Env = isolatedEnvironment(homeDirectory)
		if err := command.Start(); err != nil {
			t.Fatal(err)
		}
		deadline := time.Now().Add(2 * time.Second)
		for len(fixture.snapshots()) < 2 && time.Now().Before(deadline) {
			time.Sleep(10 * time.Millisecond)
		}
		if len(fixture.snapshots()) < 2 {
			t.Fatal("export did not begin before cancellation")
		}
		if err := command.Process.Signal(os.Interrupt); err != nil {
			t.Fatal(err)
		}
		err := command.Wait()
		exitError, ok := err.(*exec.ExitError)
		if !ok || exitError.ExitCode() != 130 {
			t.Fatalf("cancellation exit status: %v", err)
		}
		contents, err := os.ReadFile(output)
		if err != nil {
			t.Fatal(err)
		}
		if string(contents) != "original destination" {
			t.Fatalf("cancellation replaced destination: %q", contents)
		}
		staging, err := filepath.Glob(filepath.Join(temporaryDirectory, ".foxglove-export-*"))
		if err != nil {
			t.Fatal(err)
		}
		if len(staging) != 0 {
			t.Fatalf("cancellation left staging directories: %v", staging)
		}
	})
}

func TestRustAuthLoginWireContract(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	testCase := oracleCase{
		ID:   "auth-login",
		Args: []string{"auth", "login", "--base-url", "{BASE_URL}"},
		Plans: []responsePlan{
			{Method: http.MethodPost, Path: "/v1/auth/device-code", Body: `{"deviceCode":"device_fixture","userCode":"ABCD-1234","expiresIn":600,"interval":5,"verificationUri":"https://fixture.example/verify","verificationUriComplete":"https://fixture.example/verify?code=ABCD-1234"}`, Headers: jsonHeaders},
			{Method: http.MethodPost, Path: "/v1/auth/token", Status: http.StatusForbidden, Body: `{"message":"pending"}`, Headers: jsonHeaders},
			{Method: http.MethodPost, Path: "/v1/auth/token", Body: `{"idToken":"id-token-fixture"}`, Headers: jsonHeaders},
			{Method: http.MethodPost, Path: "/v1/signin", Body: `{"bearerToken":"bearer-fixture"}`, Headers: jsonHeaders},
		},
	}
	actual := runRustCaseWithFixture(t, testCase, fixture)
	expected := commandSnapshot{
		Args:     testCase.Args,
		ExitCode: 0,
		Stdout: "copy/paste the following link into your browser:\n\nhttps://fixture.example/verify?code=ABCD-1234\n\n" +
			"Verify this code and click 'Authorize' to complete login:  ABCD-1234\n",
		Config: "auth_type: 1\nbase_url: {BASE_URL}\nbearer_token: bearer-fixture\ndefault_project_id: prj_default\n",
		Requests: []requestSnapshot{
			{Method: http.MethodPost, Path: "/v1/auth/device-code", Headers: map[string]string{"Content-Type": "application/json", "User-Agent": "foxglove-cli/v1.0.33"}, Body: "{\"clientId\":\"d51173be08ed4cf7a734aed9ac30afd0\"}\n", BodyLength: 48},
			{Method: http.MethodPost, Path: "/v1/auth/token", Headers: map[string]string{"Content-Type": "application/json", "User-Agent": "foxglove-cli/v1.0.33"}, Body: "{\"clientId\":\"d51173be08ed4cf7a734aed9ac30afd0\",\"deviceCode\":\"device_fixture\"}\n", BodyLength: 78},
			{Method: http.MethodPost, Path: "/v1/auth/token", Headers: map[string]string{"Content-Type": "application/json", "User-Agent": "foxglove-cli/v1.0.33"}, Body: "{\"clientId\":\"d51173be08ed4cf7a734aed9ac30afd0\",\"deviceCode\":\"device_fixture\"}\n", BodyLength: 78},
			{Method: http.MethodPost, Path: "/v1/signin", Headers: map[string]string{"Content-Type": "application/json", "User-Agent": "foxglove-cli/v1.0.33"}, Body: "{\"idToken\":\"id-token-fixture\"}\n", BodyLength: 31},
		},
	}
	if !reflect.DeepEqual(expected, actual) {
		t.Fatalf("Rust auth login wire contract differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
	}
}

func TestRustAuthLoginErrorContexts(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	jsonHeaders := map[string]string{"Content-Type": "application/json"}
	validDeviceCode := responsePlan{Method: http.MethodPost, Path: "/v1/auth/device-code", Body: `{"deviceCode":"device_fixture","userCode":"ABCD-1234","verificationUriComplete":"https://fixture.example/verify"}`, Headers: jsonHeaders}
	validToken := responsePlan{Method: http.MethodPost, Path: "/v1/auth/token", Body: `{"idToken":"id-token-fixture"}`, Headers: jsonHeaders}
	baseCase := oracleCase{Args: []string{"auth", "login", "--base-url", "{BASE_URL}"}}

	t.Run("device-code-decode", func(t *testing.T) {
		testCase := baseCase
		testCase.Plans = []responsePlan{{Method: http.MethodPost, Path: "/v1/auth/device-code", Body: `{`}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || !strings.Contains(actual.Stderr, "Login failed: failed to fetch device code: failed to decode response:") {
			t.Fatalf("missing device-code decode context: %+v", actual)
		}
	})

	t.Run("token-unexpected-status", func(t *testing.T) {
		testCase := baseCase
		testCase.Plans = []responsePlan{validDeviceCode, responsePlan{Method: http.MethodPost, Path: "/v1/auth/token", Status: http.StatusUnauthorized, Body: `{"message":"unauthorized"}`, Headers: jsonHeaders}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || actual.Stderr != "Login failed: failed to request token: unexpected status 401\n" || len(actual.Requests) != 2 {
			t.Fatalf("unexpected token-status result: %+v", actual)
		}
	})

	t.Run("sign-in-decode", func(t *testing.T) {
		testCase := baseCase
		testCase.Plans = []responsePlan{validDeviceCode, validToken, responsePlan{Method: http.MethodPost, Path: "/v1/signin", Body: `{`}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || !strings.Contains(actual.Stderr, "Login failed: failed to sign in: failed to decode sign in response:") {
			t.Fatalf("missing sign-in decode context: %+v", actual)
		}
	})

	t.Run("device-code-transport", func(t *testing.T) {
		testCase := oracleCase{Args: []string{"auth", "login", "--base-url", "http://127.0.0.1:1"}}
		actual := runRustCase(t, testCase)
		if actual.ExitCode != 1 || !strings.Contains(actual.Stderr, "Login failed: failed to fetch device code: failed to fetch device code:") {
			t.Fatalf("missing device-code transport context: %+v", actual)
		}
	})
}

func stripProgressPrefix(stderr, finalLine string) string {
	if index := strings.LastIndex(stderr, finalLine); index >= 0 {
		return stderr[index:]
	}
	return stderr
}

func stripProgressToError(stderr, prefix string) string {
	if index := strings.LastIndex(stderr, prefix); index >= 0 {
		return stderr[index:]
	}
	return stderr
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
		arg = strings.ReplaceAll(arg, "{BASE_URL}", baseURL)
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
	return runRustCaseWithFixture(t, testCase, nil)
}

func runRustCaseWithFixture(t *testing.T, testCase oracleCase, fixture *fixtureServer) commandSnapshot {
	t.Helper()
	temporaryDirectory := t.TempDir()
	homeDirectory := filepath.Join(temporaryDirectory, "home")
	if err := os.MkdirAll(homeDirectory, 0o755); err != nil {
		t.Fatal(err)
	}
	configPath := filepath.Join(homeDirectory, ".foxgloverc")
	baseURL := ""
	if fixture != nil {
		baseURL = fixture.server.URL
		fixture.reset(testCase.Plans)
	}
	config := strings.ReplaceAll(testCase.Config, "{BASE_URL}", baseURL)
	if len(testCase.Plans) > 0 && config == "" {
		config = "auth_type: 1\nbase_url: " + baseURL + "\nbearer_token: fixture-token\ndefault_project_id: prj_default\n"
	}
	if config != "" {
		if err := os.WriteFile(configPath, []byte(config), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	args := make([]string, len(testCase.Args))
	for index, arg := range testCase.Args {
		arg = strings.ReplaceAll(arg, "{TMP}", temporaryDirectory)
		arg = strings.ReplaceAll(arg, "{REPO}", repositoryRoot)
		arg = strings.ReplaceAll(arg, "{BASE_URL}", baseURL)
		args[index] = arg
	}
	command := exec.Command(rustBinary, args...)
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
		baseURL:            "{BASE_URL}",
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
	if fixture != nil {
		snapshot.Requests = fixture.snapshots()
		if remaining := fixture.remainingPlans(); len(remaining) > 0 {
			t.Fatalf("Rust did not make %d planned request(s): %+v", len(remaining), remaining)
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
