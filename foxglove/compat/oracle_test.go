package compat

import (
	"bytes"
	"crypto/sha256"
	"encoding/csv"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/foxglove/go-rosbag"
	"github.com/foxglove/mcap/go/mcap"
	"gopkg.in/yaml.v2"
)

const baselineVersion = "v1.0.33"

var longHelpFlag = regexp.MustCompile(`--[a-z][a-z0-9-]*`)

func helpFlags(help string) []string {
	unique := map[string]struct{}{}
	for _, flag := range longHelpFlag.FindAllString(help, -1) {
		unique[flag] = struct{}{}
	}
	flags := make([]string, 0, len(unique))
	for flag := range unique {
		flags = append(flags, flag)
	}
	sort.Strings(flags)
	return flags
}

// These deprecated Go commands are intentionally absent from the Rust release
// CLI; their supported replacements are `recordings list` and `data import`.
var rustOmittedCommandSurface = map[string]struct{}{
	"data-imports":      {},
	"data-imports-add":  {},
	"data-imports-list": {},
}

var rustOmittedFlagHelpLine = regexp.MustCompile(`(?m)^.*--(?:json|serial-number).*\n`)

func rustCommandSurfaceExpected(snapshot commandSnapshot) commandSnapshot {
	snapshot.Stdout = rustOmittedFlagHelpLine.ReplaceAllString(snapshot.Stdout, "")
	return snapshot
}

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
	// Captured only by resilience tests. It deliberately stays out of golden
	// snapshots, where output files are represented by stable digests.
	outputContents map[string][]byte
}

// assertCompatible checks the CLI contract without treating presentation as a
// wire format. Human tables, help text, and successful transfer progress may
// evolve; structured output, requests, files, and command outcomes may not.
func assertCompatible(t *testing.T, expected, actual commandSnapshot) {
	t.Helper()
	if !reflect.DeepEqual(expected.Args, actual.Args) || expected.ExitCode != actual.ExitCode ||
		!reflect.DeepEqual(semanticConfig(t, expected.Config), semanticConfig(t, actual.Config)) || !reflect.DeepEqual(expected.OutputFiles, actual.OutputFiles) {
		t.Fatalf("command result differs\n--- expected\n%+v\n--- actual\n%+v", expected, actual)
	}
	assertRequestsEqual(t, expected.Requests, actual.Requests)
	switch {
	case expected.ExitCode != 0:
		// Failures have no machine-readable result, but must not leak unexpected
		// data to stdout.
		if expected.Stdout != actual.Stdout {
			t.Fatalf("failure stdout differs\n--- expected\n%q\n--- actual\n%q", expected.Stdout, actual.Stdout)
		}
	case isNDJSON(expected.Args):
		assertNDJSONEqual(t, expected.Stdout, actual.Stdout)
	case requestedFormat(expected.Args) == "json":
		assertJSONEqual(t, expected.Stdout, actual.Stdout, "stdout")
	case requestedFormat(expected.Args) == "csv":
		assertCSVEqual(t, expected.Stdout, actual.Stdout)
	default:
		if !isHumanPresentation(expected.Args) && expected.Stdout != actual.Stdout {
			t.Fatalf("stdout differs\n--- expected\n%q\n--- actual\n%q", expected.Stdout, actual.Stdout)
		}
	}
	if !isSuccessfulTransfer(expected.Args, expected.ExitCode) &&
		!isErrorReportingOnly(expected.Args, expected.ExitCode, expected.Stderr, actual.Stderr) &&
		expected.Stderr != actual.Stderr {
		t.Fatalf("stderr differs\n--- expected\n%q\n--- actual\n%q", expected.Stderr, actual.Stderr)
	}
}

func semanticConfig(t *testing.T, config string) any {
	t.Helper()
	var decoded any
	if err := yaml.Unmarshal([]byte(config), &decoded); err != nil {
		t.Fatalf("parse config YAML %q: %v", config, err)
	}
	return decoded
}

func requestedFormat(args []string) string {
	for index, arg := range args {
		if arg == "--json" {
			return "json"
		}
		if arg == "--format" && index+1 < len(args) {
			return args[index+1]
		}
		if arg == "--output-format" && index+1 < len(args) {
			return args[index+1]
		}
		if strings.HasPrefix(arg, "--format=") {
			return strings.TrimPrefix(arg, "--format=")
		}
		if strings.HasPrefix(arg, "--output-format=") {
			return strings.TrimPrefix(arg, "--output-format=")
		}
	}
	return ""
}

func isHumanPresentation(args []string) bool {
	if len(args) == 0 || requestedFormat(args) != "" {
		return len(args) == 0
	}
	for _, arg := range args {
		if arg == "help" || arg == "--help" {
			return true
		}
	}
	for _, arg := range args {
		if arg == "list" || arg == "get" {
			return true
		}
	}
	return len(args) >= 2 && args[0] == "auth" && args[1] == "info"
}

func isSuccessfulTransfer(args []string, exitCode int) bool {
	for index := range args {
		if exitCode == 0 && index+1 < len(args) && args[index] == "data" && (args[index+1] == "import" || args[index+1] == "export") {
			return true
		}
		if exitCode == 0 && index+1 < len(args) && args[index] == "extensions" && args[index+1] == "publish" {
			return true
		}
	}
	return false
}

func isErrorReportingOnly(args []string, exitCode int, expected, actual string) bool {
	return exitCode != 0 && isNDJSON(args) && strings.TrimSpace(expected) != "" && strings.TrimSpace(actual) != ""
}

func isNDJSON(args []string) bool {
	for index, arg := range args {
		if (arg == "--output-format" && index+1 < len(args) && args[index+1] == "json") || arg == "--output-format=json" {
			return true
		}
	}
	return false
}

func assertJSONEqual(t *testing.T, expected, actual, name string) {
	t.Helper()
	decode := func(value string) any {
		decoder := json.NewDecoder(strings.NewReader(value))
		decoder.UseNumber()
		var decoded any
		if err := decoder.Decode(&decoded); err != nil {
			t.Fatalf("parse %s JSON %q: %v", name, value, err)
		}
		var trailing any
		if err := decoder.Decode(&trailing); err != io.EOF {
			t.Fatalf("%s contains more than one JSON value: %q", name, value)
		}
		return decoded
	}
	if !semanticJSONEqual(decode(expected), decode(actual)) {
		t.Fatalf("%s JSON differs\n--- expected\n%s\n--- actual\n%s", name, expected, actual)
	}
}

func semanticJSONEqual(expected, actual any) bool {
	if expectedObject, ok := expected.(map[string]any); ok {
		actualObject, ok := actual.(map[string]any)
		if !ok || len(expectedObject) != len(actualObject) {
			return false
		}
		for key, expectedValue := range expectedObject {
			actualValue, ok := actualObject[key]
			if !ok {
				return false
			}
			// The baseline Go ROS1 decoder renders these fixture-specific signed
			// values as unsigned bit patterns. Keep the approved pairs narrow;
			// all other parsed values still use exact semantic equality.
			if isApprovedRos1SignedDelta(key, expectedValue, actualValue) {
				continue
			}
			if !semanticJSONEqual(expectedValue, actualValue) {
				return false
			}
		}
		return true
	}
	if expectedList, ok := expected.([]any); ok {
		actualList, ok := actual.([]any)
		if !ok || len(expectedList) != len(actualList) {
			return false
		}
		for index := range expectedList {
			if !semanticJSONEqual(expectedList[index], actualList[index]) {
				return false
			}
		}
		return true
	}
	return reflect.DeepEqual(expected, actual)
}

func isApprovedRos1SignedDelta(key string, expected, actual any) bool {
	// These are the only signed ROS1 fields known to the fixture. The Go
	// baseline reports their two's-complement bit patterns as unsigned values.
	// Requiring the exact bit-preserving conversion keeps the exception narrow.
	width, ok := map[string]int{
		"velN":      32,
		"velE":      32,
		"velD":      32,
		"relPosE":   32,
		"relPosHPE": 8,
		"prRes":     16,
		"elev":      8,
	}[key]
	if !ok {
		return false
	}
	want, ok := expected.(json.Number)
	if !ok {
		return false
	}
	got, ok := actual.(json.Number)
	if !ok {
		return false
	}
	unsigned, err := strconv.ParseUint(string(want), 10, width)
	if err != nil || unsigned < 1<<(width-1) {
		return false
	}
	signed, err := strconv.ParseInt(string(got), 10, width)
	return err == nil && signed == int64(unsigned)-int64(1<<width)
}

func assertCSVEqual(t *testing.T, expected, actual string) {
	t.Helper()
	parse := func(value string) [][]string {
		records, err := csv.NewReader(strings.NewReader(value)).ReadAll()
		if err != nil {
			t.Fatalf("parse CSV %q: %v", value, err)
		}
		return records
	}
	if !reflect.DeepEqual(parse(expected), parse(actual)) {
		t.Fatalf("CSV differs\n--- expected\n%s\n--- actual\n%s", expected, actual)
	}
}

func assertNDJSONEqual(t *testing.T, expected, actual string) {
	t.Helper()
	want := bytes.Split(bytes.TrimSpace([]byte(expected)), []byte{'\n'})
	got := bytes.Split(bytes.TrimSpace([]byte(actual)), []byte{'\n'})
	if len(want) != len(got) {
		t.Fatalf("NDJSON record count differs: want %d, got %d", len(want), len(got))
	}
	for index := range want {
		assertJSONEqual(t, string(want[index]), string(got[index]), fmt.Sprintf("NDJSON record %d", index))
	}
}

func assertRequestsEqual(t *testing.T, expected, actual []requestSnapshot) {
	t.Helper()
	if len(expected) != len(actual) {
		t.Fatalf("request count differs: want %d, got %d", len(expected), len(actual))
	}
	for index := range expected {
		want, got := expected[index], actual[index]
		if want.Method != got.Method || want.Path != got.Path || !reflect.DeepEqual(semanticHeaders(want.Headers), semanticHeaders(got.Headers)) || want.BodySHA256 != got.BodySHA256 {
			t.Fatalf("request %d differs\n--- expected\n%+v\n--- actual\n%+v", index, want, got)
		}
		assertQueryEqual(t, want.RawQuery, got.RawQuery)
		if want.BodySHA256 != "" {
			if want.BodyLength != got.BodyLength {
				t.Fatalf("request %d binary payload length differs: want %d, got %d", index, want.BodyLength, got.BodyLength)
			}
		} else if json.Valid([]byte(want.Body)) && json.Valid([]byte(got.Body)) {
			assertJSONEqual(t, want.Body, got.Body, fmt.Sprintf("request %d body", index))
		} else if want.Body != got.Body || want.BodyLength != got.BodyLength {
			t.Fatalf("request %d payload differs\n--- expected\n%+v\n--- actual\n%+v", index, want, got)
		}
	}
}

func semanticHeaders(headers map[string]string) map[string]string {
	result := make(map[string]string, len(headers))
	for name, value := range headers {
		// HTTP clients identify themselves differently; it does not change the
		// request payload or endpoint contract.
		if name != "User-Agent" {
			result[name] = value
		}
	}
	return result
}

func assertQueryEqual(t *testing.T, expected, actual string) {
	t.Helper()
	parse := func(value string) url.Values {
		parsed, err := url.ParseQuery(value)
		if err != nil {
			t.Fatalf("parse query %q: %v", value, err)
		}
		for _, values := range parsed {
			sort.Strings(values)
		}
		return parsed
	}
	if !reflect.DeepEqual(parse(expected), parse(actual)) {
		t.Fatalf("request query differs: want %q, got %q", expected, actual)
	}
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
	ID            string
	Args          []string
	Config        string
	Stdin         string
	Env           map[string]string
	Plans         []responsePlan
	InitialFiles  map[string]string
	OutputFiles   []string
	HashStdout    bool
	CaptureOutput bool
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
	rustBuild := exec.Command("cargo", "build", "--locked", "--quiet", "--features", "compat-test", "--manifest-path", filepath.Join(rustProjectRoot, "Cargo.toml"))
	rustBuild.Dir = rustProjectRoot
	// The compatibility suite compares the Rust CLI to the fixed v1.0.33 Go
	// oracle. A tagged release candidate would otherwise be embedded by
	// build.rs and make the version command (and User-Agent) differ solely
	// because of the ref CI checked out.
	rustBuild.Env = append(os.Environ(), "FOXGLOVE_VERSION="+baselineVersion)
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

// TestRustPhase1OfflineContract checks offline behavior against the shared,
// reviewed fixtures. HTTP behavior is covered by the wire contract tests.
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
		if _, omitted := rustOmittedCommandSurface[id]; omitted {
			continue
		}
		expected = rustCommandSurfaceExpected(expected)
		id, expected := id, expected
		t.Run("surface/"+id, func(t *testing.T) {
			actual := runRustCase(t, oracleCase{Args: expected.Args})
			if actual.ExitCode != 0 || actual.Stderr != "" || actual.Stdout == "" || !strings.Contains(actual.Stdout, "Usage:") {
				t.Fatalf("Rust command help is unavailable\n--- actual\n%+v", actual)
			}
			goFlags, rustFlags := helpFlags(expected.Stdout), helpFlags(actual.Stdout)
			// v2 adds project scoping to the existing event operations.
			if id == "events-add" || id == "events-list" {
				goFlags = append(goFlags, "--project-id")
				sort.Strings(goFlags)
			}
			if !reflect.DeepEqual(goFlags, rustFlags) {
				t.Fatalf("Rust command flags differ\n--- Go\n%v\n--- Rust\n%v", goFlags, rustFlags)
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
			switch testCase.ID {
			case "root-no-args", "help-command", "positional-help":
				if actual.ExitCode != 0 || actual.Stderr != "" || actual.Stdout == "" || !strings.Contains(actual.Stdout, "Usage:") {
					t.Fatalf("Rust generated help is invalid: %+v", actual)
				}
				return
			case "unknown-command", "missing-positional", "extra-positional":
				if actual.ExitCode == 0 || actual.Stdout != "" || actual.Stderr == "" || !strings.Contains(actual.Stderr, "Usage:") {
					t.Fatalf("Rust parser diagnostic is invalid: %+v", actual)
				}
				return
			}
			assertCompatible(t, expected, actual)
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
	for _, shell := range []string{"bash", "fish", "powershell", "zsh"} {
		t.Run(shell, func(t *testing.T) {
			actual := runRustCase(t, oracleCase{Args: []string{"completion", shell}})
			if actual.ExitCode != 0 || actual.Stderr != "" || actual.Stdout == "" || !strings.Contains(actual.Stdout, "foxglove") {
				t.Fatalf("Rust generated completion is invalid: %+v", actual)
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
			assertCompatible(t, expected, actual)
		})
	}

	richCases := []oracleCase{
		{ID: "device-rich", Args: []string{"devices", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/devices", Body: `[{"id":"dev_fixture","name":"Fixture","properties":{"site":"lab"},"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T04:05:06Z","projectId":"prj_default"}]`, Headers: jsonHeaders}}},
		{ID: "project-rich", Args: []string{"projects", "list", "--format", "json"}, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/projects", Body: `[{"id":"prj_default","name":"Fixture","orgMemberCount":3,"lastSeenAt":"2024-01-02T03:04:05Z"}]`, Headers: jsonHeaders}}},
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
			assertCompatible(t, expected, actual)
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
			assertCompatible(t, expected, actual)
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
		assertCompatible(t, expected, actual)
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
		{"json-ros1", []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json"}, readFixture("gps.mcap")},
		{"json-protobuf", []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json"}, protobufMcap()},
	}
	for _, testCase := range cases {
		testCase := testCase
		t.Run(testCase.name, func(t *testing.T) {
			fixtureCase := oracleCase{
				Args:       testCase.args,
				Plans:      streamPlans(testCase.payload),
				HashStdout: !strings.HasPrefix(testCase.name, "json-"),
			}
			expected := runOracleCase(t, fixtureCase, fixture)
			actual := runRustCaseWithFixture(t, fixtureCase, fixture)
			assertCompatible(t, expected, actual)
		})
	}

	t.Run("debug-precedes-direct-export-progress", func(t *testing.T) {
		fixtureCase := oracleCase{
			Args:  []string{"--debug", "data", "export", "--recording-id", "rec_fixture", "--output-format", "json"},
			Plans: streamPlans(readFixture("gps.mcap")),
		}
		expected := runOracleCase(t, fixtureCase, fixture)
		actual := runRustCaseWithFixture(t, fixtureCase, fixture)
		assertCompatible(t, expected, actual)
	})
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
			assertCompatible(t, expected, actual)
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
		assertCompatible(t, expected, actual)
	})

	t.Run("json-output-file-is-honored", func(t *testing.T) {
		testCase := oracleCase{
			Args:          []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json", "--output-file", "{TMP}/output.json"},
			Plans:         streamPlans(readFixture("gps.mcap"), 0),
			OutputFiles:   []string{"output.json"},
			CaptureOutput: true,
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.Stdout != "" || actual.OutputFiles["output.json"] == "{ABSENT}" {
			t.Fatalf("Rust JSON output-file status: exit=%d stdout=%q stderr=%q output=%q", actual.ExitCode, actual.Stdout, actual.Stderr, actual.OutputFiles["output.json"])
		}
		for _, line := range bytes.Split(bytes.TrimSpace(actual.outputContents["output.json"]), []byte{'\n'}) {
			var record map[string]any
			if err := json.Unmarshal(line, &record); err != nil {
				t.Fatalf("JSON output-file contains invalid NDJSON %q: %v", line, err)
			}
		}
	})

	t.Run("json-export-error-preserves-destination", func(t *testing.T) {
		testCase := oracleCase{
			Args:          []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json", "--output-file", "{TMP}/output.json"},
			Plans:         []responsePlan{{Method: http.MethodPost, Path: "/v1/data/stream", Status: http.StatusInternalServerError, Body: `{"message":"fixture export failure"}`, Headers: jsonHeaders}},
			InitialFiles:  map[string]string{"output.json": "original destination"},
			OutputFiles:   []string{"output.json"},
			CaptureOutput: true,
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || string(actual.outputContents["output.json"]) != "original destination" {
			t.Fatalf("Rust failed JSON export changed destination: exit=%d stderr=%q output=%q", actual.ExitCode, actual.Stderr, actual.outputContents["output.json"])
		}
	})

	t.Run("truncated-mcap-retries-and-produces-output", func(t *testing.T) {
		payload := readFixture("gps.mcap")
		suffix := mcapSuffixFromLastTimestamp(t, payload)
		testCase := oracleCase{
			Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", "{TMP}/output.mcap"},
			Plans: append(
				streamPlans(payload, 4),
				streamPlans(suffix, 0)...,
			),
			OutputFiles:   []string{"output.mcap"},
			CaptureOutput: true,
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.OutputFiles["output.mcap"] == "{ABSENT}" || len(actual.Requests) != 4 || !strings.Contains(actual.Requests[2].Body, `"start"`) {
			t.Fatalf("MCAP recovery status: Rust exit=%d requests=%d stderr=%q output=%q", actual.ExitCode, len(actual.Requests), actual.Stderr, actual.OutputFiles["output.mcap"])
		}
		assertEquivalentMCAP(t, payload, actual.outputContents["output.mcap"])
	})

	t.Run("bag-retries-until-the-boundary-stops-advancing", func(t *testing.T) {
		payload := readFixture("gps.bag")
		suffix := bagSuffixFromLastTimestamp(t, payload)
		testCase := oracleCase{
			Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "bag1", "--output-file", "{TMP}/output.bag"},
			Plans: append(
				streamPlans(payload, 4),
				streamPlans(suffix, 0)...,
			),
			OutputFiles:   []string{"output.bag"},
			CaptureOutput: true,
		}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 0 || actual.OutputFiles["output.bag"] == "{ABSENT}" || len(actual.Requests) != 4 || !strings.Contains(actual.Requests[2].Body, `"start"`) {
			t.Fatalf("bag recovery status: Rust exit=%d requests=%d stderr=%q output=%q", actual.ExitCode, len(actual.Requests), actual.Stderr, actual.OutputFiles["output.bag"])
		}
		assertEquivalentBag(t, payload, actual.outputContents["output.bag"])
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

// The complete fixture payload is the Go oracle's direct-export result. The
// Rust recovery writer is allowed to rewrite indexes and record IDs, so hashes
// are not meaningful here; compare the observable message stream instead.
func assertEquivalentMCAP(t *testing.T, expected, actual []byte) {
	t.Helper()
	read := func(data []byte) []string {
		reader, err := mcap.NewReader(bytes.NewReader(data))
		if err != nil {
			t.Fatal(err)
		}
		iterator, err := reader.Messages()
		if err != nil {
			t.Fatal(err)
		}
		messages := []string{}
		err = mcap.Range(iterator, func(_ *mcap.Schema, channel *mcap.Channel, message *mcap.Message) error {
			messages = append(messages, fmt.Sprintf("%s:%d:%d:%x", channel.Topic, message.LogTime, message.PublishTime, message.Data))
			return nil
		})
		if err != nil {
			t.Fatal(err)
		}
		return messages
	}
	if got, want := read(actual), read(expected); !reflect.DeepEqual(got, want) {
		t.Fatal(messageDifference("recovered MCAP messages differ from Go oracle", want, got))
	}
}

// The retry endpoint receives the timestamp of the recovered final message.
// Model that server contract with an inclusive suffix, rather than replaying
// the entire recording and letting a set comparison hide duplicate messages.
func mcapSuffixFromLastTimestamp(t *testing.T, data []byte) []byte {
	t.Helper()
	reader, err := mcap.NewReader(bytes.NewReader(data))
	if err != nil {
		t.Fatal(err)
	}
	iterator, err := reader.Messages()
	if err != nil {
		t.Fatal(err)
	}
	var last uint64
	if err := mcap.Range(iterator, func(_ *mcap.Schema, _ *mcap.Channel, message *mcap.Message) error {
		last = message.LogTime
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	reader, err = mcap.NewReader(bytes.NewReader(data))
	if err != nil {
		t.Fatal(err)
	}
	iterator, err = reader.Messages()
	if err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	writer, err := mcap.NewWriter(&output, &mcap.WriterOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if err := writer.WriteHeader(&mcap.Header{}); err != nil {
		t.Fatal(err)
	}
	schemas := map[uint16]bool{}
	channels := map[uint16]bool{}
	err = mcap.Range(iterator, func(schema *mcap.Schema, channel *mcap.Channel, message *mcap.Message) error {
		if message.LogTime < last {
			return nil
		}
		if schema != nil && !schemas[schema.ID] {
			if err := writer.WriteSchema(schema); err != nil {
				return err
			}
			schemas[schema.ID] = true
		}
		if !channels[channel.ID] {
			if err := writer.WriteChannel(channel); err != nil {
				return err
			}
			channels[channel.ID] = true
		}
		return writer.WriteMessage(message)
	})
	if err != nil {
		t.Fatal(err)
	}
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	return output.Bytes()
}

func assertEquivalentBag(t *testing.T, expected, actual []byte) {
	t.Helper()
	read := func(data []byte) []string {
		reader, err := rosbag.NewReader(bytes.NewReader(data))
		if err != nil {
			t.Fatal(err)
		}
		iterator, err := reader.Messages()
		if err != nil {
			t.Fatal(err)
		}
		messages := []string{}
		var previousTime uint64
		for iterator.More() {
			connection, message, err := iterator.Next()
			if err != nil {
				t.Fatal(err)
			}
			if len(messages) > 0 && message.Time < previousTime {
				t.Fatalf("bag iterator returned decreasing timestamps: %d after %d", message.Time, previousTime)
			}
			previousTime = message.Time
			messages = append(messages, fmt.Sprintf("%s:%d:%x", connection.Topic, message.Time, message.Data))
		}
		// Indexed readers may choose a different connection first when multiple
		// messages share a timestamp. Identity includes the timestamp, so sorting
		// permits only that tie reordering while preserving multiplicity.
		sort.Strings(messages)
		return messages
	}
	if got, want := read(actual), read(expected); !reflect.DeepEqual(got, want) {
		t.Fatal(messageDifference("recovered bag messages differ from Go oracle", want, got))
	}
}

func messageDifference(label string, want, got []string) string {
	limit := len(want)
	if len(got) < limit {
		limit = len(got)
	}
	for index := 0; index < limit; index++ {
		if want[index] != got[index] {
			return fmt.Sprintf("%s: want %d messages, got %d; first mismatch at %d\nwant: %.240s\n got: %.240s", label, len(want), len(got), index, want[index], got[index])
		}
	}
	return fmt.Sprintf("%s: want %d messages, got %d; common prefix length %d", label, len(want), len(got), limit)
}

func bagSuffixFromLastTimestamp(t *testing.T, data []byte) []byte {
	t.Helper()
	read := func(visit func(*rosbag.Connection, *rosbag.Message) error) {
		reader, err := rosbag.NewReader(bytes.NewReader(data))
		if err != nil {
			t.Fatal(err)
		}
		iterator, err := reader.Messages()
		if err != nil {
			t.Fatal(err)
		}
		for iterator.More() {
			connection, message, err := iterator.Next()
			if err != nil {
				t.Fatal(err)
			}
			if err := visit(connection, message); err != nil {
				t.Fatal(err)
			}
		}
	}
	var last uint64
	read(func(_ *rosbag.Connection, message *rosbag.Message) error {
		last = message.Time
		return nil
	})
	file, err := os.CreateTemp(t.TempDir(), "suffix-*.bag")
	if err != nil {
		t.Fatal(err)
	}
	path := file.Name()
	writer, err := rosbag.NewWriter(file)
	if err != nil {
		t.Fatal(err)
	}
	connections := map[uint32]bool{}
	read(func(connection *rosbag.Connection, message *rosbag.Message) error {
		if message.Time < last {
			return nil
		}
		if !connections[connection.Conn] {
			if err := writer.WriteConnection(connection); err != nil {
				return err
			}
			connections[connection.Conn] = true
		}
		return writer.WriteMessage(message)
	})
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	if err := file.Close(); err != nil {
		t.Fatal(err)
	}
	result, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return result
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
	assertCompatible(t, expected, actual)
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

	t.Run("token-unauthorized-approved-delta", func(t *testing.T) {
		testCase := baseCase
		testCase.Plans = []responsePlan{validDeviceCode, responsePlan{Method: http.MethodPost, Path: "/v1/auth/token", Status: http.StatusUnauthorized, Body: `{"message":"unauthorized"}`, Headers: jsonHeaders}}
		actual := runRustCaseWithFixture(t, testCase, fixture)
		if actual.ExitCode != 1 || actual.Stderr != "Login failed: failed to request token: forbidden: have you signed in with `foxglove auth login`?\n" || len(actual.Requests) != 2 {
			t.Fatalf("unexpected token-unauthorized result: %+v", actual)
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
	if len(expected) != len(actual) {
		t.Fatalf("oracle contract case count differs from %s: want %d, got %d", goldenPath, len(expected), len(actual))
	}
	for id, expectedSnapshot := range expected {
		actualSnapshot, ok := actual[id]
		if !ok {
			t.Fatalf("oracle contract is missing %q from %s", id, goldenPath)
		}
		assertCompatible(t, expectedSnapshot, actualSnapshot)
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
	writeInitialFiles(t, temporaryDirectory, testCase.InitialFiles)
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
		if testCase.CaptureOutput {
			snapshot.outputContents = map[string][]byte{}
		}
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
			if testCase.CaptureOutput {
				snapshot.outputContents[name] = bytes
			}
		}
	}
	return snapshot
}

func runRustCase(t *testing.T, testCase oracleCase) commandSnapshot {
	return runRustCaseWithFixture(t, testCase, nil)
}

// Legacy export/attachment/event cases compare unscoped Go behavior. The approved
// consistent-project-defaults delta is covered independently by Rust tests.
func legacyUnscopedProjectArgs(args []string) []string {
	command := args
globalFlags:
	for len(command) > 0 {
		switch {
		case command[0] == "--debug", strings.HasPrefix(command[0], "--debug="), strings.HasPrefix(command[0], "--config="), strings.HasPrefix(command[0], "--client-id="):
			command = command[1:]
		case (command[0] == "--config" || command[0] == "--client-id") && len(command) > 1:
			command = command[2:]
		default:
			break globalFlags
		}
	}
	if len(command) < 2 || !((command[0] == "data" && command[1] == "export") ||
		(command[0] == "attachments" && command[1] == "list") ||
		(command[0] == "events" && (command[1] == "list" || command[1] == "add"))) {
		return args
	}
	for _, arg := range args {
		if arg == "--project-id" || strings.HasPrefix(arg, "--project-id=") {
			return args
		}
	}
	return append(args, "--project-id=")
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
	writeInitialFiles(t, temporaryDirectory, testCase.InitialFiles)
	command := exec.Command(rustBinary, legacyUnscopedProjectArgs(args)...)
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
	// Approved project-scope diagnostics: compare retained Go behavior while
	// independent Rust tests verify the exact new stderr lines.
	snapshot.Stderr = strings.TrimSuffix(snapshot.Stderr, "Using default project prj_default; pass --project-id= to omit project scope.\n")
	snapshot.Stderr = strings.TrimPrefix(snapshot.Stderr, "[DEBUG] Project scope: unscoped (source: flag)\n")
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
		if testCase.CaptureOutput {
			snapshot.outputContents = map[string][]byte{}
		}
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
			if testCase.CaptureOutput {
				snapshot.outputContents[name] = bytes
			}
		}
	}
	return snapshot
}

func writeInitialFiles(t *testing.T, directory string, files map[string]string) {
	t.Helper()
	for name, contents := range files {
		path := filepath.Join(directory, name)
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(contents), 0o600); err != nil {
			t.Fatal(err)
		}
	}
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
