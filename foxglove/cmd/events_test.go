package cmd

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestEventsClient(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	require.NoError(t, err)
	client := api.NewMockAuthedClient(t, sv.BaseURL())

	created, err := client.CreateEvent(api.CreateEventRequest{
		DeviceID:    "test-device",
		Start:       "2023-04-19T13:26:37Z",
		End:         "2023-04-19T13:26:38Z",
		EventTypeID: "evtt_default",
		Properties:  map[string]interface{}{"severity": "high"},
	})
	require.NoError(t, err)
	assert.NotEmpty(t, created.ID)
	assert.Equal(t, "test-device", created.Device.ID)
	assert.Equal(t, "evtt_default", created.EventTypeID)
	assert.Empty(t, created.Metadata)

	listed, err := client.Events(&api.EventsRequest{
		DeviceID:    "test-device",
		EventTypeID: "evtt_default",
		EventID:     created.ID,
		Query:       "severity:high",
		QueryFields: "properties",
	})
	require.NoError(t, err)
	require.Len(t, listed, 1)
	assert.Equal(t, created.ID, listed[0].ID)

	got, err := client.GetEvent(created.ID)
	require.NoError(t, err)
	assert.Equal(t, created.ID, got.ID)

	clearedType := ""
	metadata := map[string]string{"note": "updated"}
	updated, err := client.UpdateEvent(created.ID, api.UpdateEventRequest{
		Start:       "2023-04-19T13:26:40Z",
		End:         "2023-04-19T13:26:41Z",
		Metadata:    &metadata,
		EventTypeID: &clearedType,
		Properties:  map[string]interface{}{"severity": nil},
	})
	require.NoError(t, err)
	assert.Equal(t, created.ID, updated.ID)
	assert.Equal(t, "2023-04-19T13:26:40Z", updated.Start)
	assert.Equal(t, "", updated.EventTypeID)
	assert.Equal(t, map[string]string{"note": "updated"}, updated.Metadata)

	_, err = client.GetEvent("evt_missing")
	assert.ErrorIs(t, err, api.ErrNotFound)

	err = client.DeleteEvent(created.ID)
	require.NoError(t, err)
	listed, err = client.Events(&api.EventsRequest{EventID: created.ID})
	require.NoError(t, err)
	assert.Empty(t, listed)
}

func TestListEventsCommandSendsPublicQueryParams(t *testing.T) {
	clientID := "custom-client"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, http.MethodGet, r.Method)
		assert.Equal(t, "/v1/events", r.URL.Path)
		q := r.URL.Query()
		assert.Equal(t, "dev_1", q.Get("deviceId"))
		assert.Equal(t, "RobotA", q.Get("deviceName"))
		assert.Equal(t, "prj_1", q.Get("projectId"))
		assert.Equal(t, "evt_1", q.Get("eventId"))
		assert.Equal(t, "evtt_1", q.Get("eventTypeId"))
		assert.Equal(t, "2024-01-01T00:00:00Z", q.Get("start"))
		assert.Equal(t, "2024-01-02T00:00:00Z", q.Get("end"))
		assert.Equal(t, "2024-01-03T00:00:00Z", q.Get("createdAfter"))
		assert.Equal(t, "2024-01-04T00:00:00Z", q.Get("updatedAfter"))
		assert.Equal(t, "severity:high", q.Get("query"))
		assert.Equal(t, "metadata,properties", q.Get("queryFields"))
		assert.Equal(t, "createdAt", q.Get("sortBy"))
		assert.Equal(t, "desc", q.Get("sortOrder"))
		assert.Equal(t, "10", q.Get("limit"))
		assert.Equal(t, "5", q.Get("offset"))
		_, _ = w.Write([]byte(`[]`))
	}))
	defer server.Close()

	cmd := newListEventsCommand(&baseParams{
		baseURL:   server.URL,
		clientID:  &clientID,
		token:     "custom-token",
		userAgent: "foxglove-cli/test-version",
	})
	cmd.SetArgs([]string{
		"--device-id", "dev_1",
		"--device-name", "RobotA",
		"--project-id", "prj_1",
		"--event-id", "evt_1",
		"--event-type-id", "evtt_1",
		"--start", "2024-01-01T00:00:00Z",
		"--end", "2024-01-02T00:00:00Z",
		"--created-after", "2024-01-03T00:00:00Z",
		"--updated-after", "2024-01-04T00:00:00Z",
		"--query", "severity:high",
		"--query-field", "metadata",
		"--query-field", "properties",
		"--sort-by", "createdAt",
		"--sort-order", "desc",
		"--limit", "10",
		"--offset", "5",
		"--format", "json",
	})
	assert.NoError(t, cmd.Execute())
}

func TestAddEventCommandSendsPublicBody(t *testing.T) {
	clientID := "custom-client"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, http.MethodPost, r.Method)
		assert.Equal(t, "/v1/events", r.URL.Path)
		body, err := io.ReadAll(r.Body)
		require.NoError(t, err)
		var req api.CreateEventRequest
		require.NoError(t, json.Unmarshal(body, &req))
		assert.Equal(t, "dev_1", req.DeviceID)
		assert.Equal(t, "RobotA", req.DeviceName)
		assert.Equal(t, "prj_1", req.ProjectID)
		assert.Equal(t, "2024-01-01T00:00:00Z", req.Start)
		assert.Equal(t, "2024-01-01T00:00:01Z", req.End)
		assert.Equal(t, "evtt_1", req.EventTypeID)
		assert.Empty(t, req.Metadata)
		_, _ = w.Write([]byte(`{"id":"evt_created","start":"2024-01-01T00:00:00Z","end":"2024-01-01T00:00:01Z","metadata":{},"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	cmd := newAddEventCommand(&baseParams{
		baseURL:   server.URL,
		clientID:  &clientID,
		token:     "custom-token",
		userAgent: "foxglove-cli/test-version",
	})
	cmd.SetArgs([]string{
		"--device-id", "dev_1",
		"--device-name", "RobotA",
		"--project-id", "prj_1",
		"--start", "2024-01-01T00:00:00Z",
		"--end", "2024-01-01T00:00:01Z",
		"--event-type-id", "evtt_1",
	})
	assert.NoError(t, cmd.Execute())
}

func TestGetEditDeleteEventCommands(t *testing.T) {
	clientID := "custom-client"
	var patchBody api.UpdateEventRequest
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case r.Method == http.MethodGet && r.URL.Path == "/v1/events/evt_1":
			_, _ = w.Write([]byte(`{"id":"evt_1","start":"2024-01-01T00:00:00Z","end":"2024-01-01T00:00:01Z","device":{"id":"dev_1","name":"RobotA"},"metadata":{},"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
		case r.Method == http.MethodPatch && r.URL.Path == "/v1/events/evt_1":
			patchBody = api.UpdateEventRequest{}
			require.NoError(t, json.NewDecoder(r.Body).Decode(&patchBody))
			_, _ = w.Write([]byte(`{"id":"evt_1","start":"2024-01-01T00:00:10Z","end":"2024-01-01T00:00:11Z","metadata":{"k":"v"},"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
		case r.Method == http.MethodDelete && r.URL.Path == "/v1/events/evt_1":
			_, _ = w.Write([]byte(`{"id":"evt_1"}`))
		default:
			t.Fatalf("unexpected request %s %s", r.Method, r.URL.Path)
		}
	}))
	defer server.Close()
	params := &baseParams{
		baseURL:   server.URL,
		clientID:  &clientID,
		token:     "custom-token",
		userAgent: "foxglove-cli/test-version",
	}

	getCmd := newGetEventCommand(params)
	getCmd.SetArgs([]string{"evt_1", "--format", "json"})
	assert.NoError(t, getCmd.Execute())

	editCmd := newEditEventCommand(params)
	editCmd.SetArgs([]string{"evt_1", "--start", "2024-01-01T00:00:10Z", "--end", "2024-01-01T00:00:11Z", "--metadata", "k:v", "--event-type-id", "", "--remove-property", "severity"})
	assert.NoError(t, editCmd.Execute())
	assert.Equal(t, "2024-01-01T00:00:10Z", patchBody.Start)
	assert.Equal(t, "2024-01-01T00:00:11Z", patchBody.End)
	require.NotNil(t, patchBody.Metadata)
	assert.Equal(t, map[string]string{"k": "v"}, *patchBody.Metadata)
	require.NotNil(t, patchBody.EventTypeID)
	assert.Equal(t, "", *patchBody.EventTypeID)
	assert.Contains(t, patchBody.Properties, "severity")
	assert.Nil(t, patchBody.Properties["severity"])

	clearMetadataCmd := newEditEventCommand(params)
	clearMetadataCmd.SetArgs([]string{"evt_1", "--clear-metadata"})
	assert.NoError(t, clearMetadataCmd.Execute())
	require.NotNil(t, patchBody.Metadata)
	assert.Empty(t, *patchBody.Metadata)

	invalidMetadataCmd := newEditEventCommand(params)
	invalidMetadataCmd.SetArgs([]string{"evt_1", "--metadata", "k:v", "--clear-metadata"})
	assert.EqualError(t, invalidMetadataCmd.Execute(), "--metadata and --clear-metadata cannot be used together")

	deleteCmd := newDeleteEventCommand(params)
	deleteCmd.SetArgs([]string{"evt_1"})
	assert.NoError(t, deleteCmd.Execute())
}

func TestJoinQueryFields(t *testing.T) {
	value, err := joinQueryFields([]string{"metadata", "properties"})
	assert.NoError(t, err)
	assert.Equal(t, "metadata,properties", value)

	_, err = joinQueryFields([]string{"nope"})
	assert.EqualError(t, err, `invalid --query-field value "nope": must be "metadata" or "properties"`)
}

func TestParseMetadataPairs(t *testing.T) {
	metadata, err := parseMetadataPairs([]string{"a:b", "c:d:e"})
	assert.NoError(t, err)
	assert.Equal(t, map[string]string{"a": "b", "c": "d:e"}, metadata)

	_, err = parseMetadataPairs([]string{"novalue"})
	assert.Error(t, err)
}

func TestParseEventPropertiesToRemove(t *testing.T) {
	properties, err := parseEventProperties(nil, nil, []string{"severity", "note"})
	require.NoError(t, err)
	assert.Equal(t, map[string]interface{}{
		"severity": nil,
		"note":     nil,
	}, properties)

	_, err = parseEventProperties(nil, nil, []string{""})
	assert.EqualError(t, err, "property key to remove cannot be empty")
}

func TestValidateCreateEventData(t *testing.T) {
	assert.EqualError(t,
		validateCreateEventData("evtt_1", []string{"note:value"}, nil),
		"--metadata cannot be used with --event-type-id",
	)
	assert.EqualError(t,
		validateCreateEventData("", []string{"note:value"}, []string{"severity:high"}),
		"--metadata cannot be used with --property",
	)
	assert.NoError(t, validateCreateEventData("", []string{"note:value"}, nil))
	assert.NoError(t, validateCreateEventData("", nil, []string{"severity:high"}))
	assert.NoError(t, validateCreateEventData("evtt_1", nil, []string{"severity:high"}))
}
