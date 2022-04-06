package console

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/ajg/form"
)

var (
	ErrForbidden = errors.New("Forbidden. Have you signed in with `foxglove auth login`?")
	ErrNotFound  = errors.New("not found")
)

type Request any

type Record interface {
	Headers() []string
	Fields() []string
}

type TokenRequest struct {
	ClientID   string `json:"clientId"`
	DeviceCode string `json:"deviceCode"`
}

type TokenResponse struct {
	IDToken string `json:"idToken"`
}

type UploadRequest struct {
	Filename string `json:"filename"`
	DeviceID string `json:"deviceId"`
}

type UploadResponse struct {
	Link string `json:"link"`
}

type StreamRequest struct {
	DeviceID     string    `json:"deviceId"`
	Start        time.Time `json:"start"`
	End          time.Time `json:"end"`
	OutputFormat string    `json:"outputFormat"`
	Topics       []string  `json:"topics"`
}

type StreamResponse struct {
	Link string `json:"link"`
}

type DeviceCodeRequest struct {
	ClientID string `json:"clientId"`
}

type DeviceCodeResponse struct {
	DeviceCode              string `json:"deviceCode"`
	UserCode                string `json:"userCode"`
	ExpiresIn               int    `json:"expiresIn"`
	Interval                int    `json:"interval"`
	VerificationUri         string `json:"verificationUri"`
	VerificationUriComplete string `json:"verificationUriComplete"`
}

type DevicesRequest struct{}

type DevicesResponse struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"createdAt"`
	UpdatedAt time.Time `json:"updatedAt"`
}

func (r DevicesResponse) Fields() []string {
	return []string{
		r.ID,
		r.Name,
		r.CreatedAt.Format(time.RFC3339),
		r.UpdatedAt.Format(time.RFC3339),
	}
}

func (r DevicesResponse) Headers() []string {
	return []string{
		"ID",
		"Name",
		"Created At",
		"Updated At",
	}
}

type ImportsRequest struct {
	DeviceID       string `json:"deviceId" form:"deviceId,omitempty"`
	Start          string `json:"start" form:"start,omitempty"`
	End            string `json:"end" form:"end,omitempty"`
	DataStart      string `json:"dataStart" form:"dataStart,omitempty"`
	DataEnd        string `json:"dataEnd" form:"dataEnd,omitempty"`
	IncludeDeleted bool   `json:"includeDeleted" form:"includeDeleted,omitempty"`
}

type ImportsResponse struct {
	ImportID        string    `json:"importId"`
	DeviceID        string    `json:"deviceId"`
	Filename        string    `json:"filename"`
	ImportTime      time.Time `json:"importTime"`
	Start           time.Time `json:"start"`
	End             time.Time `json:"end"`
	InputType       string    `json:"inputType"`
	OutputType      string    `json:"outputType"`
	InputSize       int64     `json:"inputSize"`
	TotalOutputSize int64     `json:"totalOutputSize"`
}

func (r ImportsResponse) Fields() []string {
	return []string{
		r.ImportID,
		r.DeviceID,
		r.Filename,
		r.ImportTime.Format(time.RFC3339),
		r.Start.Format(time.RFC3339),
		r.End.Format(time.RFC3339),
		r.InputType,
		r.OutputType,
		fmt.Sprintf("%d", r.InputSize),
		fmt.Sprintf("%d", r.TotalOutputSize),
	}
}

func (r ImportsResponse) Headers() []string {
	return []string{
		"Import ID",
		"Device ID",
		"Filename",
		"Import Time",
		"Start",
		"End",
		"Input Type",
		"Output Type",
		"Input Size",
		"Total Output Size",
	}
}

type EventsRequest struct {
	DeviceID   string `json:"deviceId" form:"deviceId,omitempty"`
	DeviceName string `json:"deviceName" form:"deviceName,omitempty"`
	SortBy     string `json:"sortBy" form:"sortBy,omitempty"`
	SortOrder  string `json:"sortOrder" form:"sortOrder,omitempty"`
	Limit      int    `json:"limit" form:"limit,omitempty"`
	Offset     int    `json:"offset" form:"offset,omitempty"`
	Start      string `json:"start" form:"start,omitempty"`
	End        string `json:"end" form:"end,omitempty"`
	Key        string `json:"key" form:"key,omitempty"`
	Value      string `json:"value" form:"value,omitempty"`
}

type EventsResponse struct {
	ID             string            `json:"id"`
	DeviceID       string            `json:"deviceId"`
	TimestampNanos string            `json:"timestampNanos"`
	DurationNanos  string            `json:"durationNanos"`
	Metadata       map[string]string `json:"metadata"`
	CreatedAt      string            `json:"createdAt"`
	UpdatedAt      string            `json:"updatedAt"`
}

func (r EventsResponse) Fields() []string {
	metadata, _ := json.Marshal(r.Metadata)
	return []string{
		r.ID,
		r.DeviceID,
		r.TimestampNanos,
		r.DurationNanos,
		r.CreatedAt,
		r.UpdatedAt,
		string(metadata),
	}
}

func (r EventsResponse) Headers() []string {
	return []string{
		"ID",
		"Device ID",
		"Timestamp",
		"Duration",
		"Created At",
		"Updated At",
		"Metadata",
	}
}

type CoverageRequest struct {
	Start string `json:"start" form:"start,omitempty"`
	End   string `json:"end" form:"end,omitempty"`
}
type CoverageResponse struct {
	DeviceID string `json:"deviceId"`
	Start    string `json:"start"`
	End      string `json:"end"`
}

func (r CoverageResponse) Headers() []string {
	return []string{
		"Device ID",
		"Start",
		"End",
	}
}

func (r CoverageResponse) Fields() []string {
	return []string{
		r.DeviceID,
		r.Start,
		r.End,
	}
}

type FoxgloveClient struct {
	baseurl   string
	clientID  string
	userAgent string
	authed    *http.Client
	unauthed  *http.Client
}

type SignInRequest struct {
	Token string `json:"idToken"`
}

type SignInResponse struct {
	BearerToken string `json:"bearerToken"`
}

type ErrorResponse struct {
	Error string `json:"error"`
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

func list[
	RequestType any, ResponseType any,
](
	c *FoxgloveClient,
	endpoint string,
	req RequestType,
) ([]ResponseType, error) {
	querystring, err := form.EncodeToString(req)
	if err != nil {
		return nil, fmt.Errorf("failed to encode request: %w", err)
	}
	resp, err := c.authed.Get(c.baseurl + endpoint + "?" + querystring)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch records: %w", err)
	}
	switch resp.StatusCode {
	case http.StatusForbidden:
		return nil, ErrForbidden
	case http.StatusOK:
		break
	default:
		return nil, unpackErrorResponse(resp.Body)
	}
	response := []ResponseType{}
	err = json.NewDecoder(resp.Body).Decode(&response)
	if err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}
	return response, nil
}

func (c *FoxgloveClient) Devices(req DevicesRequest) ([]DevicesResponse, error) {
	return list[DevicesRequest, DevicesResponse](c, "/v1/devices", req)
}

func (c *FoxgloveClient) Events(req *EventsRequest) ([]EventsResponse, error) {
	return list[EventsRequest, EventsResponse](c, "/beta/device-events", *req)
}

func (c *FoxgloveClient) Imports(req *ImportsRequest) ([]ImportsResponse, error) {
	return list[ImportsRequest, ImportsResponse](c, "/v1/data/imports", *req)
}

func (c *FoxgloveClient) Coverage(req *CoverageRequest) ([]CoverageResponse, error) {
	return list[CoverageRequest, CoverageResponse](c, "/v1/data/coverage", *req)
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
