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

func TestEventTypesClient(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	require.NoError(t, err)
	client := api.NewMockAuthedClient(t, sv.BaseURL())

	listed, err := client.EventTypes(&api.EventTypesRequest{})
	require.NoError(t, err)
	require.NotEmpty(t, listed)
	assert.Equal(t, "evtt_default", listed[0].ID)

	created, err := client.CreateEventType(api.CreateEventTypeRequest{
		Name:      "Hardware Fault",
		ColorName: "red",
		CustomProperties: []api.EventTypeCustomProperty{
			{ID: "cprop_evt_str", Required: true},
		},
	})
	require.NoError(t, err)
	assert.NotEmpty(t, created.ID)
	assert.Equal(t, "Hardware Fault", created.Name)
	assert.Equal(t, "red", created.ColorName)
	assert.Equal(t, []api.EventTypeCustomProperty{{ID: "cprop_evt_str", Required: true}}, created.CustomProperties)

	got, err := client.GetEventType(created.ID)
	require.NoError(t, err)
	assert.Equal(t, created.ID, got.ID)

	updatedProps := []api.EventTypeCustomProperty{{ID: "cprop_evt_enum", Required: false}}
	updated, err := client.UpdateEventType(created.ID, api.UpdateEventTypeRequest{
		Name:             "Fault",
		ColorName:        "orange",
		CustomProperties: &updatedProps,
	})
	require.NoError(t, err)
	assert.Equal(t, "Fault", updated.Name)
	assert.Equal(t, "orange", updated.ColorName)
	assert.Equal(t, updatedProps, updated.CustomProperties)

	_, err = client.GetEventType("evtt_missing")
	assert.ErrorIs(t, err, api.ErrNotFound)

	err = client.DeleteEventType(created.ID)
	require.NoError(t, err)
	_, err = client.GetEventType(created.ID)
	assert.ErrorIs(t, err, api.ErrNotFound)
}

func TestAddEventTypeCommandSendsPublicBody(t *testing.T) {
	clientID := "custom-client"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, http.MethodPost, r.Method)
		assert.Equal(t, "/v1/event-types", r.URL.Path)
		body, err := io.ReadAll(r.Body)
		require.NoError(t, err)
		var req api.CreateEventTypeRequest
		require.NoError(t, json.Unmarshal(body, &req))
		assert.Equal(t, "Incident", req.Name)
		assert.Equal(t, "red", req.ColorName)
		assert.Equal(t, []api.EventTypeCustomProperty{
			{ID: "cprop_1", Required: true},
			{ID: "cprop_2", Required: false},
		}, req.CustomProperties)
		_, _ = w.Write([]byte(`{"id":"evtt_1","name":"Incident","colorName":"red","customProperties":[],"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
	}))
	defer server.Close()

	cmd := newAddEventTypeCommand(&baseParams{
		baseURL:   server.URL,
		clientID:  &clientID,
		token:     "custom-token",
		userAgent: "foxglove-cli/test-version",
	})
	cmd.SetArgs([]string{
		"--name", "Incident",
		"--color-name", "red",
		"--custom-property", "cprop_1:true",
		"--custom-property", "cprop_2",
	})
	assert.NoError(t, cmd.Execute())
}

func TestGetEditDeleteEventTypeCommands(t *testing.T) {
	clientID := "custom-client"
	var patchBody api.UpdateEventTypeRequest
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case r.Method == http.MethodGet && r.URL.Path == "/v1/event-types/evtt_1":
			_, _ = w.Write([]byte(`{"id":"evtt_1","name":"Incident","colorName":"red","customProperties":[],"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
		case r.Method == http.MethodPatch && r.URL.Path == "/v1/event-types/evtt_1":
			patchBody = api.UpdateEventTypeRequest{}
			require.NoError(t, json.NewDecoder(r.Body).Decode(&patchBody))
			_, _ = w.Write([]byte(`{"id":"evtt_1","name":"Fault","colorName":"orange","customProperties":[{"id":"cprop_1","required":true}],"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}`))
		case r.Method == http.MethodDelete && r.URL.Path == "/v1/event-types/evtt_1":
			_, _ = w.Write([]byte(`{"id":"evtt_1"}`))
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

	getCmd := newGetEventTypeCommand(params)
	getCmd.SetArgs([]string{"evtt_1", "--format", "json"})
	assert.NoError(t, getCmd.Execute())

	editCmd := newEditEventTypeCommand(params)
	editCmd.SetArgs([]string{"evtt_1", "--name", "Fault", "--color-name", "orange", "--custom-property", "cprop_1:true"})
	assert.NoError(t, editCmd.Execute())
	assert.Equal(t, "Fault", patchBody.Name)
	assert.Equal(t, "orange", patchBody.ColorName)
	require.NotNil(t, patchBody.CustomProperties)
	assert.Equal(t, []api.EventTypeCustomProperty{{ID: "cprop_1", Required: true}}, *patchBody.CustomProperties)

	clearCmd := newEditEventTypeCommand(params)
	clearCmd.SetArgs([]string{"evtt_1", "--clear-custom-properties"})
	assert.NoError(t, clearCmd.Execute())
	require.NotNil(t, patchBody.CustomProperties)
	assert.Empty(t, *patchBody.CustomProperties)

	invalidPropertiesCmd := newEditEventTypeCommand(params)
	invalidPropertiesCmd.SetArgs([]string{"evtt_1", "--custom-property", "cprop_1", "--clear-custom-properties"})
	assert.EqualError(t, invalidPropertiesCmd.Execute(), "--custom-property and --clear-custom-properties cannot be used together")

	deleteCmd := newDeleteEventTypeCommand(params)
	deleteCmd.SetArgs([]string{"evtt_1"})
	assert.NoError(t, deleteCmd.Execute())
}

func TestParseEventTypeCustomProperties(t *testing.T) {
	props, err := parseEventTypeCustomProperties([]string{"cprop_1:true", "cprop_2:false", "cprop_3"})
	assert.NoError(t, err)
	assert.Equal(t, []api.EventTypeCustomProperty{
		{ID: "cprop_1", Required: true},
		{ID: "cprop_2", Required: false},
		{ID: "cprop_3", Required: false},
	}, props)

	_, err = parseEventTypeCustomProperties([]string{"cprop_1:maybe"})
	assert.Error(t, err)
}
