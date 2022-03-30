package console

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"

	"github.com/ajg/form"
)

var (
	ErrForbidden = errors.New("Forbidden. Have you signed in with `foxglove auth login`?")
	ErrNotFound  = errors.New("not found")
)

type FoxgloveClient struct {
	baseurl   string
	clientID  string
	userAgent string
	authed    *http.Client
	unauthed  *http.Client
}

func unpackErrorResponse(r io.Reader) error {
	resp := ErrorResponse{}
	bytes, err := io.ReadAll(r)
	if err != nil {
		return fmt.Errorf("failed to read error response: %w", err)
	}
	err = json.Unmarshal(bytes, &resp)
	if err != nil {
		return fmt.Errorf(string(bytes))
	}
	return fmt.Errorf("%s", resp.Error)
}

// SignIn accepts a client ID token and uses it to authenticate to foxglove,
// returning a bearer token for use in subsequent HTTP requests.
func (c *FoxgloveClient) SignIn(token string) (string, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(SignInRequest{
		Token: token,
	})
	if err != nil {
		return "", fmt.Errorf("failed to encode request: %w", err)
	}
	resp, err := c.unauthed.Post(c.baseurl+"/v1/signin", "application/json", buf)
	if err != nil {
		return "", fmt.Errorf("sign in failure: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return "", unpackErrorResponse(resp.Body)
	}
	r := SignInResponse{}
	err = json.NewDecoder(resp.Body).Decode(&r)
	if err != nil {
		return "", fmt.Errorf("failed to decode sign in response: %w", err)
	}
	c.authed = makeClient(c.userAgent, r.BearerToken)
	return r.BearerToken, nil
}

// Stream returns a ReadCloser wrapping a binary output stream in response to
// the provided request.
func (c *FoxgloveClient) Stream(r StreamRequest) (io.ReadCloser, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(r)
	if err != nil {
		return nil, fmt.Errorf("failed to serialize request: %w", err)
	}
	resp, err := c.authed.Post(c.baseurl+"/v1/data/stream", "application/json", buf)
	if err != nil {
		return nil, fmt.Errorf("failed to get download link: %w", err)
	}
	switch resp.StatusCode {
	case http.StatusOK:
		break
	case http.StatusForbidden, http.StatusUnauthorized:
		return nil, ErrForbidden
	default:
		return nil, unpackErrorResponse(resp.Body)
	}
	link := StreamResponse{}
	err = json.NewDecoder(resp.Body).Decode(&link)
	if err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}
	resp, err = http.Get(link.Link)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch download: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return nil, unpackErrorResponse(resp.Body)
	}
	return resp.Body, nil
}

// Upload uploads the contents of a reader for a provided filenamem and device.
// It manages the indirection through GCS signed upload links for the caller.
func (c *FoxgloveClient) Upload(reader io.Reader, r UploadRequest) error {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(r)
	if err != nil {
		return fmt.Errorf("failed to encode import request: %w", err)
	}
	resp, err := c.authed.Post(c.baseurl+"/v1/data/upload", "application/json", buf)
	if err != nil {
		return fmt.Errorf("import request failure: %w", err)
	}
	switch resp.StatusCode {
	case http.StatusOK:
		break
	case http.StatusForbidden, http.StatusUnauthorized:
		return ErrForbidden
	default:
		return unpackErrorResponse(resp.Body)
	}
	link := UploadResponse{}
	err = json.NewDecoder(resp.Body).Decode(&link)
	if err != nil {
		return fmt.Errorf("failed to decode import response: %w", err)
	}
	client := &http.Client{}
	req, err := http.NewRequest("PUT", link.Link, reader)
	if err != nil {
		return fmt.Errorf("failed to build upload request: %w", err)
	}
	req.Header.Add("Content-Type", "application/octet-stream")
	resp, err = client.Do(req)
	if err != nil {
		return fmt.Errorf("upload failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("unexpected %d on upload request", resp.StatusCode)
	}
	return nil
}

// DeviceCode retrieves a device code, which may be used to correlate a login
// action with a token through the API.
func (c *FoxgloveClient) DeviceCode() (*DeviceCodeResponse, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(DeviceCodeRequest{
		ClientID: c.clientID,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to serialize device code request: %w", err)
	}
	resp, err := c.unauthed.Post(c.baseurl+"/v1/auth/device-code", "application/json", buf)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch device code: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return nil, unpackErrorResponse(resp.Body)
	}
	response := &DeviceCodeResponse{}
	err = json.NewDecoder(resp.Body).Decode(response)
	if err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}
	return response, nil
}

func (c *FoxgloveClient) post(
	endpoint string,
	req any,
	target any,
) error {
	buf := bytes.Buffer{}
	err := json.NewEncoder(&buf).Encode(req)
	if err != nil {
		return fmt.Errorf("failed to encode request: %w", err)
	}
	resp, err := c.authed.Post(c.baseurl+endpoint, "application/json", &buf)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	switch resp.StatusCode {
	case http.StatusForbidden:
		return ErrForbidden
	case http.StatusOK:
		break
	default:
		return unpackErrorResponse(resp.Body)
	}
	err = json.NewDecoder(resp.Body).Decode(target)
	if err != nil {
		return fmt.Errorf("failed to decode response: %w", err)
	}
	return nil
}

func (c *FoxgloveClient) CreateDevice(req CreateDeviceRequest) (resp CreateDeviceResponse, err error) {
	err = c.post("/v1/devices", req, &resp)
	return resp, err
}

func (c *FoxgloveClient) CreateEvent(req CreateEventRequest) (resp CreateEventResponse, err error) {
	err = c.post("/beta/device-events", req, &resp)
	return resp, err
}

func (c *FoxgloveClient) get(endpoint string, req any, target any) error {
	querystring, err := form.EncodeToString(req)
	if err != nil {
		return fmt.Errorf("failed to encode request: %w", err)
	}
	resp, err := c.authed.Get(c.baseurl + endpoint + "?" + querystring)
	if err != nil {
		return fmt.Errorf("failed to fetch records: %w", err)
	}
	switch resp.StatusCode {
	case http.StatusForbidden:
		return ErrForbidden
	case http.StatusOK:
		break
	default:
		return unpackErrorResponse(resp.Body)
	}
	err = json.NewDecoder(resp.Body).Decode(target)
	if err != nil {
		return fmt.Errorf("failed to decode response: %w", err)
	}
	return nil
}

func (c *FoxgloveClient) Devices(req DevicesRequest) (resp []DevicesResponse, err error) {
	err = c.get("/v1/devices", req, &resp)
	return resp, err
}

func (c *FoxgloveClient) Events(req *EventsRequest) (resp []EventsResponse, err error) {
	err = c.get("/beta/device-events", req, &resp)
	return resp, err
}

func (c *FoxgloveClient) Imports(req *ImportsRequest) (resp []ImportsResponse, err error) {
	err = c.get("/v1/data/imports", req, &resp)
	return resp, err
}

func (c *FoxgloveClient) Coverage(req *CoverageRequest) (resp []CoverageResponse, err error) {
	err = c.get("/v1/data/coverage", *req, &resp)
	return resp, err
}

// Token returns a token for the provided device code. If the token for the
// device code does not exist yet, ErrForbidden is returned. It is up to the
// caller to give up after sufficient retries.
func (c *FoxgloveClient) Token(deviceCode string) (string, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(TokenRequest{
		DeviceCode: deviceCode,
		ClientID:   c.clientID,
	})
	if err != nil {
		return "", fmt.Errorf("failed to encode token request: %w", err)
	}
	resp, err := c.unauthed.Post(c.baseurl+"/v1/auth/token", "application/json", buf)
	if err != nil {
		return "", fmt.Errorf("token request failure: %w", err)
	}
	switch {
	case resp.StatusCode == http.StatusForbidden:
		return "", ErrForbidden
	case resp.StatusCode != http.StatusOK:
		return "", fmt.Errorf("unexpected status %d", resp.StatusCode)
	}
	tokenResponse := TokenResponse{}
	err = json.NewDecoder(resp.Body).Decode(&tokenResponse)
	if err != nil {
		return "", fmt.Errorf("failed to parse response body: %w", err)
	}
	return tokenResponse.IDToken, nil
}

type customTransport struct {
	baseTransport http.RoundTripper
	token         string
	userAgent     string
}

func (t *customTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", t.token))
	req.Header.Add("User-Agent", t.userAgent)
	return t.baseTransport.RoundTrip(req)
}

func makeClient(userAgent, token string) *http.Client {
	return &http.Client{
		Transport: &customTransport{
			userAgent:     userAgent,
			token:         token,
			baseTransport: http.DefaultTransport,
		},
	}
}

// NewRemoteFoxgloveClient returns a client implementation backed by the remote
// cloud service. The "token" parameter will be passed in authorization headers.
// For unauthenticated usage (token, device code - the initial signin flow) it
// may be passed as empty, however authorized requests will fail.
func NewRemoteFoxgloveClient(baseurl, clientID, token, userAgent string) *FoxgloveClient {
	return &FoxgloveClient{
		baseurl:   baseurl,
		clientID:  clientID,
		userAgent: userAgent,
		authed:    makeClient(userAgent, token),
		unauthed:  makeClient(userAgent, ""),
	}
}
