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
	ImportID     string     `json:"importId,omitempty"`
	DeviceID     string     `json:"device.id,omitempty"`
	DeviceName   string     `json:"device.name,omitempty"`
	Start        *time.Time `json:"start,omitempty"`
	End          *time.Time `json:"end,omitempty"`
	OutputFormat string     `json:"outputFormat"`
	Topics       []string   `json:"topics"`
}

func (req *StreamRequest) Validate() error {
	if req.ImportID == "" && req.DeviceID == "" && req.DeviceName == "" {
		return fmt.Errorf("either import-id or device-id/device-name, start, and end are required")
	}
	if req.DeviceID != "" && req.DeviceName != "" && req.ImportID == "" && (req.Start == nil || req.End == nil) {
		return fmt.Errorf("start/end are required if device is supplied")
	}
	if req.Start != nil && req.End != nil && req.End.Before(*req.Start) {
		return fmt.Errorf("end must be after or equal to start")
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

type AttachmentsRequest struct {
	ImportID    string `form:"importId,omitempty"`
	RecordingID string `form:"recordingId,omitempty"`
}

type AttachmentsResponse struct {
	ID          string `json:"id"`
	RecordingID string `json:"recordingId"`
	SiteID      string `json:"siteId"`
	Name        string `json:"name"`
	MediaType   string `json:"mediaType"`
	LogTime     string `json:"logTime"`
	CreateTime  string `json:"createTime"`
	CRC         uint32 `json:"crc"`
	Size        int    `json:"size"`
	Fingerprint string `json:"fingerprint"`
}

func (r AttachmentsResponse) Fields() []string {
	return []string{
		r.ID,
		r.RecordingID,
		r.SiteID,
		r.Name,
		r.MediaType,
		r.LogTime,
		r.CreateTime,
		fmt.Sprintf("%d", r.CRC),
		fmt.Sprintf("%d", r.Size),
		r.Fingerprint,
	}
}

func (r AttachmentsResponse) Headers() []string {
	return []string{
		"ID",
		"Recording ID",
		"Site ID",
		"Name",
		"Media Type",
		"Log Time",
		"Create Time",
		"CRC",
		"Size",
		"Fingerprint",
	}
}

type RecordingsRequest struct {
	DeviceID     string `json:"device.id" form:"device.id,omitempty"`
	Start        string `json:"start" form:"start,omitempty"`
	End          string `json:"end" form:"end,omitempty"`
	Path         string `json:"path" form:"path,omitempty"`
	SiteID       string `json:"site.id" form:"site.id,omitempty"`
	EdgeSiteID   string `json:"edgeSite.id" form:"edgeSite.id,omitempty"`
	ImportStatus string `json:"importStatus" form:"importStatus,omitempty"`
}

type SiteSummary struct {
	Name string `json:"name"`
	ID   string `json:"id"`
}

type DeviceSummary struct {
	Name string `json:"name"`
	ID   string `json:"id"`
}

type MetadataRecord struct {
	Name     string            `json:"name"`
	Metadata map[string]string `json:"metadata"`
}

type RecordingsResponse struct {
	ID           string           `json:"id"`
	Path         string           `json:"path"`
	Size         int64            `json:"size"`
	MessageCount int64            `json:"messageCount"`
	CreatedAt    string           `json:"createdAt"`
	ImportedAt   string           `json:"importedAt"`
	Start        string           `json:"start"`
	End          string           `json:"end"`
	ImportStatus string           `json:"importStatus"`
	Site         SiteSummary      `json:"site"`
	EdgeSite     SiteSummary      `json:"edgeSite"`
	Device       DeviceSummary    `json:"device"`
	Metadata     []MetadataRecord `json:"metadata"`
}

func (r RecordingsResponse) Headers() []string {
	return []string{
		"Recording ID",
		"Path",
		"Size",
		"Message Count",
		"Created At",
		"Imported At",
		"Start",
		"End",
		"Import Status",
		"Site ID",
		"Site Name",
		"Edge Site ID",
		"Edge Site Name",
		"Device ID",
		"Device Name",
		"Metadata",
	}
}

func (r RecordingsResponse) Fields() []string {
	metadata, _ := json.Marshal(r.Metadata)
	return []string{
		r.ID,
		r.Path,
		humanReadableBytes(r.Size),
		fmt.Sprint(r.MessageCount),
		r.CreatedAt,
		r.ImportedAt,
		r.Start,
		r.End,
		r.ImportStatus,
		r.Site.ID,
		r.Site.Name,
		r.EdgeSite.ID,
		r.EdgeSite.Name,
		r.Device.ID,
		r.Device.Name,
		string(metadata),
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
	DeviceID  string `json:"deviceId" form:"deviceId,omitempty"`
	SortBy    string `json:"sortBy" form:"sortBy,omitempty"`
	SortOrder string `json:"sortOrder" form:"sortOrder,omitempty"`
	Limit     int    `json:"limit" form:"limit,omitempty"`
	Offset    int    `json:"offset" form:"offset,omitempty"`
	Start     string `json:"start" form:"start,omitempty"`
	End       string `json:"end" form:"end,omitempty"`
	Query     string `json:"key" form:"query,omitempty"`
}

type EventResponseItem struct {
	ID        string            `json:"id"`
	DeviceID  string            `json:"deviceId"`
	Start     string            `json:"start"`
	End       string            `json:"end"`
	Metadata  map[string]string `json:"metadata"`
	CreatedAt string            `json:"createdAt"`
	UpdatedAt string            `json:"updatedAt"`
}

func (r EventResponseItem) Fields() []string {
	metadata, _ := json.Marshal(r.Metadata)
	return []string{
		r.ID,
		r.DeviceID,
		r.Start,
		r.End,
		r.CreatedAt,
		r.UpdatedAt,
		string(metadata),
	}
}

func (r EventResponseItem) Headers() []string {
	return []string{
		"ID",
		"Device ID",
		"Start",
		"End",
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
	Error   string `json:"error"`
	Message string `json:"message"`
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
	DeviceID string            `json:"deviceId"`
	Start    string            `json:"start"`
	End      string            `json:"end"`
	Metadata map[string]string `json:"metadata"`
}

type CreateEventResponse = EventResponseItem

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

func humanReadableBytes(b int64) string {
	const unit = 1024
	if b < unit {
		return fmt.Sprintf("%d B", b)
	}
	div, exp := int64(unit), 0
	for n := b / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %ciB", float64(b)/float64(div), "KMGTPE"[exp])
}
