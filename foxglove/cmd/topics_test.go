package cmd

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestTopicsCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.NoError(t, err)

	t.Run("lists topics", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		topics, err := client.Topics(&api.TopicsRequest{})
		assert.NoError(t, err)
		assert.Contains(t, topics, api.TopicsResponse{
			Encoding:       "cdr",
			SchemaEncoding: "ros2msg",
			SchemaName:     "sensor_msgs/msg/Image",
			Topic:          "/camera/image",
			Version:        "1",
		})
	})
}

func TestTopicsRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/v1/data/topics", r.URL.Path)
		assert.Equal(t, "Bearer token", r.Header.Get("Authorization"))
		assert.Equal(t, "device-123", r.URL.Query().Get("deviceId"))
		assert.Equal(t, "2024-01-01T00:00:00Z", r.URL.Query().Get("start"))
		assert.Empty(t, r.URL.Query().Get("end"))
		assert.Equal(t, "true", r.URL.Query().Get("includeSchemas"))
		assert.Equal(t, "topic", r.URL.Query().Get("sortBy"))
		assert.Equal(t, "10", r.URL.Query().Get("limit"))
		_, _ = w.Write([]byte(`[{"encoding":"cdr","schema":"c2NoZW1h","schemaEncoding":"ros2msg","schemaName":"pkg/msg/Foo","topic":"/foo","version":"1"}]`))
	}))
	defer server.Close()

	client := api.NewRemoteFoxgloveClient(server.URL, "client", "token", "user-agent")
	topics, err := client.Topics(&api.TopicsRequest{
		DeviceID:       "device-123",
		Start:          "2024-01-01T00:00:00Z",
		IncludeSchemas: true,
		SortBy:         "topic",
		Limit:          10,
	})

	assert.NoError(t, err)
	assert.Equal(t, []api.TopicsResponse{{
		Encoding:       "cdr",
		Schema:         "c2NoZW1h",
		SchemaEncoding: "ros2msg",
		SchemaName:     "pkg/msg/Foo",
		Topic:          "/foo",
		Version:        "1",
	}}, topics)
}

func TestListTopicsCommandUsesBaseParams(t *testing.T) {
	clientID := "custom-client"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "Bearer custom-token", r.Header.Get("Authorization"))
		assert.Equal(t, "foxglove-cli/test-version", r.Header.Get("User-Agent"))
		assert.Equal(t, "rec_123", r.URL.Query().Get("recordingId"))
		_, _ = w.Write([]byte(`[]`))
	}))
	defer server.Close()

	cmd := newListTopicsCommand(&baseParams{
		baseURL:   server.URL,
		clientID:  &clientID,
		token:     "custom-token",
		userAgent: "foxglove-cli/test-version",
	})
	cmd.SetArgs([]string{"--recording-id", "rec_123", "--format", "json"})

	assert.NoError(t, cmd.Execute())
}

func TestValidateTopicsRequest(t *testing.T) {
	assert.EqualError(t, validateTopicsRequest(api.TopicsRequest{}),
		"provide one of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key")
	assert.NoError(t, validateTopicsRequest(api.TopicsRequest{RecordingID: "rec_123"}))
}
