package console

import (
	"encoding/json"
	"fmt"
	"time"
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
	ImportID     string     `json:"importId"`
	DeviceID     string     `json:"deviceId"`
	Start        *time.Time `json:"start,omitempty"`
	End          *time.Time `json:"end,omitempty"`
	OutputFormat string     `json:"outputFormat"`
	Topics       []string   `json:"topics"`
}

func (req *StreamRequest) Validate() error {
	if req.ImportID == "" && req.DeviceID == "" {
		return fmt.Errorf("device-id or import-id is required")
	}
	if req.DeviceID != "" && req.ImportID == "" && (req.Start == nil || req.End == nil) {
		return fmt.Errorf("start/end are required if device-id is supplied")
	}
	return nil
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
	ID              string    `json:"id"`
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
		r.ID,
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

type SignInRequest struct {
	Token string `json:"idToken"`
}

type SignInResponse struct {
	BearerToken string `json:"bearerToken"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}

type CreateDeviceRequest struct {
	Name         string `json:"name"`
	SerialNumber string `json:"serialNumber,omitempty"`
}
type CreateDeviceResponse struct {
	ID           string  `json:"id"`
	Name         string  `json:"name"`
	SerialNumber *string `json:"serialNumber"`
}

type CreateEventRequest struct {
	DeviceID      string            `json:"deviceId"`
	DeviceName    string            `json:"deviceName"`
	Timestamp     string            `json:"timestamp"`
	DurationNanos string            `json:"durationNanos"`
	Metadata      map[string]string `json:"metadata"`
}

type CreateEventResponse struct {
	ID             string            `json:"id"`
	DeviceID       string            `json:"deviceId"`
	TimestampNanos string            `json:"timestampNanos"`
	DurationNanos  string            `json:"durationNanos"`
	Metadata       map[string]string `json:"metadata"`
	CreatedAt      string            `json:"createdAt"`
	UpdatedAt      string            `json:"updatedAt"`
}

type ExtensionsRequest struct{}

type ExtensionResponse struct {
	ID            string  `json:"id"`
	Name          string  `json:"name"`
	Publisher     string  `json:"publisher"`
	DisplayName   string  `json:"displayName"`
	Description   *string `json:"description"`
	ActiveVersion *string `json:"activeVersion"`
	Sha256Sum     *string `json:"sha256Sum"`
}

func (r ExtensionResponse) Fields() []string {
	return []string{
		r.ID,
		r.Name,
		r.Publisher,
		r.DisplayName,
		requiredVal(r.Description),
		requiredVal(r.ActiveVersion),
		requiredVal(r.Sha256Sum),
	}
}

func (r ExtensionResponse) Headers() []string {
	return []string{
		"ID",
		"Name",
		"Publisher",
		"Display Name",
		"Description",
		"Active Version",
		"SHA-256 Sum",
	}
}

func (e ExtensionResponse) String() string {
	version := e.ActiveVersion
	if version == nil {
		return fmt.Sprintf("%s.%s", e.Publisher, e.Name)
	}
	return fmt.Sprintf("%s.%s-%s", e.Publisher, e.Name, *version)
}

func requiredVal(val *string) string {
	if val != nil {
		return *val
	} else {
		return ""
	}
}
