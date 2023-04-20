package console

import "encoding/json"

type BetaEventsRequest struct {
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

type BetaEventsResponse struct {
	ID             string            `json:"id"`
	DeviceID       string            `json:"deviceId"`
	TimestampNanos string            `json:"timestampNanos"`
	DurationNanos  string            `json:"durationNanos"`
	Metadata       map[string]string `json:"metadata"`
	CreatedAt      string            `json:"createdAt"`
	UpdatedAt      string            `json:"updatedAt"`
}

func (r BetaEventsResponse) Fields() []string {
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

func (r BetaEventsResponse) Headers() []string {
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

type BetaCreateEventRequest struct {
	DeviceID      string            `json:"deviceId"`
	DeviceName    string            `json:"deviceName"`
	Timestamp     string            `json:"timestamp"`
	DurationNanos string            `json:"durationNanos"`
	Metadata      map[string]string `json:"metadata"`
}

type BetaCreateEventResponse struct {
	ID             string            `json:"id"`
	DeviceID       string            `json:"deviceId"`
	TimestampNanos string            `json:"timestampNanos"`
	DurationNanos  string            `json:"durationNanos"`
	Metadata       map[string]string `json:"metadata"`
	CreatedAt      string            `json:"createdAt"`
	UpdatedAt      string            `json:"updatedAt"`
}
