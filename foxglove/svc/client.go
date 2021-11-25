package svc

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"time"
)

var (
	ErrForbidden = errors.New("forbidden")
	ErrNotFound  = errors.New("not found")
)

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

type FoxgloveClient interface {
	Stream(StreamRequest) (io.ReadCloser, error)
	Upload(io.Reader, UploadRequest) error
	DeviceCode() (*DeviceCodeResponse, error)
	Token(string) (string, error)
	SignIn(string) (string, error)
}

type foxgloveClient struct {
	baseurl  string
	clientID string
	httpc    *http.Client
}

type SignInRequest struct {
	Token string `json:"idToken"`
}

type SignInResponse struct {
	BearerToken string `json:"bearerToken"`
}

func (c *foxgloveClient) SignIn(token string) (string, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(SignInRequest{
		Token: token,
	})
	if err != nil {
		return "", fmt.Errorf("failed to encode request: %w", err)
	}
	resp, err := http.Post(c.baseurl+"/v1/signin", "application/json", buf)
	if err != nil {
		return "", fmt.Errorf("sign in failure: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		bytes, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("unexpected %d sign in response: %s", resp.StatusCode, string(bytes))
	}
	r := SignInResponse{}
	err = json.NewDecoder(resp.Body).Decode(&r)
	if err != nil {
		return "", fmt.Errorf("failed to decode sign in response: %w", err)
	}
	return r.BearerToken, nil
}

func (c *foxgloveClient) Stream(r StreamRequest) (io.ReadCloser, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(r)
	if err != nil {
		return nil, fmt.Errorf("failed to serialize request: %w", err)
	}

	req, err := http.NewRequest("POST", c.baseurl+"/v1/data/stream", buf)
	if err != nil {
		return nil, fmt.Errorf("failed to build streaming request: %w", err)
	}
	resp, err := c.httpc.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to get download link: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		bytes, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("unexpected %d response: %s", resp.StatusCode, string(bytes))
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
		bytes, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("unexpected %d from stream service: %s", resp.StatusCode, string(bytes))
	}
	return resp.Body, nil
}

func (c *foxgloveClient) Upload(reader io.Reader, r UploadRequest) error {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(r)
	if err != nil {
		return fmt.Errorf("failed to encode import request: %w", err)
	}
	req, err := http.NewRequest("POST", c.baseurl+"/v1/data/upload", buf)
	if err != nil {
		return fmt.Errorf("failed to build upload request: %w", err)
	}
	resp, err := c.httpc.Do(req)
	if err != nil {
		return fmt.Errorf("import request failure: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		bytes, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("import request failure (%d): %s", resp.StatusCode, string(bytes))
	}
	link := UploadResponse{}
	err = json.NewDecoder(resp.Body).Decode(&link)
	if err != nil {
		return fmt.Errorf("failed to decode import response: %w", err)
	}
	client := &http.Client{}
	req, err = http.NewRequest("PUT", link.Link, reader)
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

func (c *foxgloveClient) DeviceCode() (*DeviceCodeResponse, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(DeviceCodeRequest{
		ClientID: c.clientID,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to serialize device code request: %w", err)
	}
	resp, err := http.Post(c.baseurl+"/v1/auth/device-code", "application/json", buf)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch device code: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("unexpected response: %s", string(body))
	}
	response := &DeviceCodeResponse{}
	err = json.NewDecoder(resp.Body).Decode(response)
	if err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}
	return response, nil
}

func (c *foxgloveClient) Token(deviceCode string) (string, error) {
	buf := &bytes.Buffer{}
	err := json.NewEncoder(buf).Encode(TokenRequest{
		DeviceCode: deviceCode,
		ClientID:   c.clientID,
	})
	if err != nil {
		return "", fmt.Errorf("failed to encode token request: %w", err)
	}
	resp, err := http.Post(c.baseurl+"/v1/auth/token", "application/json", buf)
	if err != nil {
		return "", fmt.Errorf("token request failure", err)
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
}

func (t *customTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", t.token))
	req.Header.Add("Content-Type", "application/json")
	return t.baseTransport.RoundTrip(req)
}

// NewRemoteFoxgloveClient returns a client implementation backed by the remote
// cloud service. The "token" parameter will be passed in authorization headers.
// For unauthenticated usage (token, device code) it may be passed as empty,
// however authorized requests will fail.
func NewRemoteFoxgloveClient(baseurl, clientID, token string) FoxgloveClient {
	httpc := &http.Client{
		Transport: &customTransport{
			token:         token,
			baseTransport: http.DefaultTransport,
		},
	}
	return &foxgloveClient{
		baseurl:  baseurl,
		clientID: clientID,
		httpc:    httpc,
	}
}
