package compat

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/json"
	"encoding/pem"
	"io"
	"log"
	"math/big"
	"net"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/foxglove/mcap/go/mcap"
)

// These responses mirror optional fields in the API response mappers. Test
// omissions and explicit nulls without changing the released Go snapshots.
func TestRustOptionalResponseFields(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	cases := []struct {
		command, path, body string
		optional            []string
	}{
		{"data coverage", "/v1/data/coverage", `{"start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","status":"imported"}`, []string{"deviceId", "device"}},
		{"pending-imports", "/v1/data/pending-imports", `{"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","orgId":"org_fixture","filename":"fixture.mcap","pipelineStage":"parse","requestId":"req_fixture","siteId":"site_fixture"}`, []string{"deviceId", "deviceName", "importId", "projectId", "status", "error"}},
		{"events", "/v1/events", `{"id":"evt_fixture","device":{"id":"dev_fixture","name":"Fixture"},"start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","metadata":{},"properties":{}}`, []string{"eventTypeId"}},
		{"recordings", "/v1/recordings", `{"id":"rec_fixture","path":"fixture.mcap","size":128,"createdAt":"2024-01-02T03:04:05Z","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","importStatus":"none","projectId":"prj_default"}`, []string{"messageCount", "importedAt", "site", "edgeSite", "device", "key", "metadata"}},
		{"event-types", "/v1/event-types", `{"id":"evtt_fixture","name":"Fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"}`, []string{"colorName", "properties"}},
	}
	for _, tc := range cases {
		for _, variant := range []string{"missing", "null"} {
			for _, format := range []string{"json", "csv"} {
				t.Run(tc.command+"/"+variant+"/"+format, func(t *testing.T) {
					var record map[string]any
					if err := json.Unmarshal([]byte(tc.body), &record); err != nil {
						t.Fatal(err)
					}
					if variant == "null" {
						for _, field := range tc.optional {
							record[field] = nil
						}
					}
					body, err := json.Marshal([]any{record})
					if err != nil {
						t.Fatal(err)
					}
					testCase := oracleCase{Args: append(strings.Fields(tc.command), "list", "--format", format), Plans: []responsePlan{{Method: http.MethodGet, Path: tc.path, Body: string(body), Headers: map[string]string{"Content-Type": "application/json"}}}}
					expected := runOracleCase(t, testCase, fixture)
					actual := runRustCaseWithFixture(t, testCase, fixture)
					if expected.ExitCode != 0 || actual.ExitCode != 0 {
						t.Fatalf("list failed: Go=%+v Rust=%+v", expected, actual)
					}
					assertCompatible(t, expected, actual)
				})
			}
		}
	}
}

func TestRustRepeatedEventQueryFields(t *testing.T) {
	fixture := newFixtureServer()
	defer fixture.close()
	for _, fields := range [][]string{{"metadata"}, {"properties"}, {"metadata", "properties"}} {
		t.Run(fields[0]+"-"+fields[len(fields)-1], func(t *testing.T) {
			args := []string{"events", "list", "--format", "json", "--query", "robot & camera", "--device-name", "Robot A", "--limit", "3", "--offset", "1"}
			for _, field := range fields {
				args = append(args, "--query-field", field)
			}
			tc := oracleCase{Args: args, Plans: []responsePlan{{Method: http.MethodGet, Path: "/v1/events", Body: "[]", Headers: map[string]string{"Content-Type": "application/json"}}}}
			expected := runOracleCase(t, tc, fixture)
			actual := runRustCaseWithFixture(t, tc, fixture)
			if actual.ExitCode != 0 {
				t.Fatalf("query failed: %+v", actual)
			}
			// Go sends queryFields/0, queryFields/1, which the API ignores.
			// The approved event-query-fields delta uses repeated parameters.
			query, err := url.ParseQuery(expected.Requests[0].RawQuery)
			if err != nil {
				t.Fatal(err)
			}
			for index, field := range fields {
				query.Del("queryFields/" + strconv.Itoa(index))
				query.Add("queryFields", field)
			}
			expected.Requests[0].RawQuery = query.Encode()
			assertCompatible(t, expected, actual)
		})
	}
}

func TestRustNestedRosPackageExport(t *testing.T) {
	var data bytes.Buffer
	writer, err := mcap.NewWriter(&data, &mcap.WriterOptions{})
	if err != nil {
		t.Fatal(err)
	}
	definition := []byte("pkg_a/Helper own\npkg_b/Child child\n================================================================================\nMSG: pkg_a/Helper\nuint32 own_value\n================================================================================\nMSG: pkg_b/Child\nHelper[] helpers\n================================================================================\nMSG: pkg_b/Helper\nuint32 child_value\n")
	for _, err := range []error{
		writer.WriteHeader(&mcap.Header{}),
		writer.WriteSchema(&mcap.Schema{ID: 1, Name: "pkg_a/Root", Encoding: "ros1msg", Data: definition}),
		writer.WriteChannel(&mcap.Channel{ID: 1, SchemaID: 1, Topic: "/nested", MessageEncoding: "ros1"}),
		writer.WriteMessage(&mcap.Message{ChannelID: 1, LogTime: 1, PublishTime: 1, Data: []byte{1, 0, 0, 0, 1, 0, 0, 0, 42, 0, 0, 0}}),
		writer.Close(),
	} {
		if err != nil {
			t.Fatal(err)
		}
	}
	fixture := newFixtureServer()
	defer fixture.close()
	tc := oracleCase{Args: []string{"data", "export", "--recording-id", "rec_fixture", "--output-format", "json"}, Plans: []responsePlan{
		{Method: http.MethodPost, Path: "/v1/data/stream", Body: `{"link":"{BASE_URL}/nested"}`, Headers: map[string]string{"Content-Type": "application/json"}},
		{Method: http.MethodGet, Path: "/nested", Body: data.String()},
	}}
	expected := runOracleCase(t, tc, fixture)
	actual := runRustCaseWithFixture(t, tc, fixture)
	if expected.ExitCode != 0 || actual.ExitCode != 0 {
		t.Fatalf("export failed: Go=%+v Rust=%+v", expected, actual)
	}
	assertCompatible(t, expected, actual)
}

func TestRustCustomCertificateTrust(t *testing.T) {
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/projects" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = io.WriteString(w, "[]")
	}))
	server.Config.ErrorLog = log.New(io.Discard, "", 0)
	certificate, rootPEM := tlsFixtureCertificate(t)
	server.TLS = &tls.Config{Certificates: []tls.Certificate{certificate}, MinVersion: tls.VersionTLS12}
	server.StartTLS()
	defer server.Close()
	directory := t.TempDir()
	certPath := filepath.Join(directory, "ca.pem")
	cert := rootPEM
	if err := os.WriteFile(certPath, cert, 0o600); err != nil {
		t.Fatal(err)
	}
	emptyDirectory := t.TempDir()
	tc := oracleCase{Args: []string{"projects", "list", "--format", "json"}, Config: "base_url: " + server.URL + "\n", Env: map[string]string{"SSL_CERT_FILE": certPath, "SSL_CERT_DIR": emptyDirectory}}
	actual := runRustCase(t, tc)
	if actual.ExitCode != 0 || actual.Stdout != "[]\n" {
		t.Fatalf("custom CA not trusted: %+v", actual)
	}
	// The same server must fail when its certificate is absent from the trust
	// store. This guards against accidentally disabling certificate verification.
	if err := os.WriteFile(certPath, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	actual = runRustCase(t, tc)
	if actual.ExitCode == 0 || actual.Stdout != "" {
		t.Fatalf("untrusted certificate accepted: %+v", actual)
	}
}

// Use a CA and a separate server leaf. Go's default httptest certificate is
// itself a CA, which Rustls correctly rejects when used as an end entity.
func tlsFixtureCertificate(t *testing.T) (tls.Certificate, []byte) {
	t.Helper()
	rootKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	root := &x509.Certificate{SerialNumber: big.NewInt(1), Subject: pkix.Name{CommonName: "CLI fixture CA"}, NotBefore: time.Now().Add(-time.Hour), NotAfter: time.Now().Add(time.Hour), IsCA: true, BasicConstraintsValid: true, KeyUsage: x509.KeyUsageCertSign}
	rootDER, err := x509.CreateCertificate(rand.Reader, root, root, &rootKey.PublicKey, rootKey)
	if err != nil {
		t.Fatal(err)
	}
	leafKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	leaf := &x509.Certificate{SerialNumber: big.NewInt(2), Subject: pkix.Name{CommonName: "localhost"}, NotBefore: root.NotBefore, NotAfter: root.NotAfter, BasicConstraintsValid: true, KeyUsage: x509.KeyUsageDigitalSignature, ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth}, IPAddresses: []net.IP{net.ParseIP("127.0.0.1")}}
	leafDER, err := x509.CreateCertificate(rand.Reader, leaf, root, &leafKey.PublicKey, rootKey)
	if err != nil {
		t.Fatal(err)
	}
	keyDER, err := x509.MarshalECPrivateKey(leafKey)
	if err != nil {
		t.Fatal(err)
	}
	cert, err := tls.X509KeyPair(pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: leafDER}), pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER}))
	if err != nil {
		t.Fatal(err)
	}
	return cert, pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: rootDER})
}

// Exercise the real signal handler and staging cleanup, in addition to the
// portable CancellationToken tests run on every native Rust platform.
func TestRustCtrlCDuringResponseBody(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("os.Interrupt delivery requires a Unix process; native Rust tests cover cancellation tokens on Windows")
	}
	cases := []struct {
		name, endpoint string
		status         int
		login          bool
	}{
		{"export-success", "/v1/data/stream", 200, false},
		{"export-error", "/v1/data/stream", 500, false},
		{"login-device-code", "/v1/auth/device-code", 200, true},
		{"login-token", "/v1/auth/token", 200, true},
		{"login-signin", "/v1/signin", 200, true},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			ready := make(chan struct{}, 1)
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				_, _ = io.Copy(io.Discard, r.Body)
				w.Header().Set("Content-Type", "application/json")
				if r.URL.Path == tc.endpoint {
					w.Header().Set("Content-Length", "1000")
					w.WriteHeader(tc.status)
					_, _ = io.WriteString(w, "{")
					w.(http.Flusher).Flush()
					ready <- struct{}{}
					<-r.Context().Done()
					return
				}
				switch r.URL.Path {
				case "/v1/auth/device-code":
					_, _ = io.WriteString(w, `{"deviceCode":"fixture","userCode":"1234","verificationUriComplete":"https://example.invalid"}`)
				case "/v1/auth/token":
					_, _ = io.WriteString(w, `{"idToken":"fixture"}`)
				default:
					http.NotFound(w, r)
				}
			}))
			defer server.Close()
			directory := t.TempDir()
			configPath := filepath.Join(directory, ".foxgloverc")
			config := "base_url: " + server.URL + "\nbearer_token: original-fixture-token\n"
			if err := os.WriteFile(configPath, []byte(config), 0o600); err != nil {
				t.Fatal(err)
			}
			output := filepath.Join(directory, "output.mcap")
			if err := os.WriteFile(output, []byte("original destination"), 0o600); err != nil {
				t.Fatal(err)
			}
			args := []string{"data", "export", "--recording-id", "rec_fixture", "--output-file", output}
			if tc.login {
				args = []string{"auth", "login", "--base-url", server.URL}
			}
			ctx, cancel := context.WithTimeout(context.Background(), 8*time.Second)
			defer cancel()
			command := exec.CommandContext(ctx, rustBinary, args...)
			command.Env = isolatedEnvironment(directory)
			var stderr bytes.Buffer
			command.Stderr = &stderr
			if err := command.Start(); err != nil {
				t.Fatal(err)
			}
			done := make(chan error, 1)
			go func() { done <- command.Wait() }()
			select {
			case <-ready:
			case err := <-done:
				t.Fatalf("command exited before stalled response: %v %s", err, stderr.String())
			case <-ctx.Done():
				t.Fatal("command did not reach stalled response")
			}
			// Allow the headers to reach the client before delivering Ctrl-C.
			time.Sleep(100 * time.Millisecond)
			if err := command.Process.Signal(os.Interrupt); err != nil {
				t.Fatal(err)
			}
			err := <-done
			exitError, ok := err.(*exec.ExitError)
			if !ok || exitError.ExitCode() != 130 {
				t.Fatalf("expected exit 130: %v %s", err, stderr.String())
			}
			for path, want := range map[string]string{configPath: config, output: "original destination"} {
				data, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				if string(data) != want {
					t.Fatalf("cancellation changed %s", path)
				}
			}
			staging, err := filepath.Glob(filepath.Join(directory, ".foxglove-export-*"))
			if err != nil {
				t.Fatal(err)
			}
			if len(staging) != 0 {
				t.Fatalf("cancellation left staging directories: %v", staging)
			}
		})
	}
}
