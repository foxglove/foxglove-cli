package api

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestRecordingsFields(t *testing.T) {
	t.Run("sets empty values for optional fields", func(t *testing.T) {
		resp := RecordingsResponse{
			ID:           "some-id-1",
			Path:         "/file.mcap",
			Size:         1024,
			MessageCount: 1,
			CreatedAt:    "2023-03-02T15:00:00.000Z",
			ImportedAt:   "2023-03-03T15:00:00.000Z",
			Start:        "2023-03-01T15:00:00.000Z",
			End:          "2023-03-01T15:10:00.000Z",
			ImportStatus: "pending",
		}

		edgeSiteIdx := -1
		for i, header := range resp.Headers() {
			if header == "Edge Site ID" {
				edgeSiteIdx = i
			}
		}

		assert.Greater(t, edgeSiteIdx, -1)
		assert.Equal(t, "some-id-1", resp.Fields()[0])
		assert.Equal(t, "", resp.Fields()[edgeSiteIdx])
	})
}

func TestStreamRequestValidate(t *testing.T) {
	start := time.Now().Add(-time.Hour)
	end := time.Now()

	t.Run("validates with recording-id", func(t *testing.T) {
		req := &StreamRequest{
			RecordingID:  "rec-123",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("validates with recording key", func(t *testing.T) {
		req := &StreamRequest{
			Key:          "my-recording-key",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("validates with session-id", func(t *testing.T) {
		req := &StreamRequest{
			SessionID:    "session-123",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("validates with session-key and project-id", func(t *testing.T) {
		req := &StreamRequest{
			SessionKey:   "my-session-key",
			ProjectID:    "prj_123",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("fails with session-key without project-id", func(t *testing.T) {
		req := &StreamRequest{
			SessionKey:   "my-session-key",
			OutputFormat: "mcap0",
		}
		err := req.Validate()
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "project-id is required when using session-key")
	})

	t.Run("validates with device and start/end", func(t *testing.T) {
		req := &StreamRequest{
			DeviceID:     "device-123",
			DeviceName:   "My Device",
			Start:        &start,
			End:          &end,
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})
	t.Run("fails with device without start/end and no other source", func(t *testing.T) {
		req := &StreamRequest{
			DeviceID:     "device-123",
			DeviceName:   "My Device",
			OutputFormat: "mcap0",
		}
		err := req.Validate()
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "start/end are required if device is supplied")
	})

	t.Run("validates with device and session-id (no start/end required)", func(t *testing.T) {
		req := &StreamRequest{
			DeviceID:     "device-123",
			SessionID:    "session-123",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("validates with device and recording-id (no start/end required)", func(t *testing.T) {
		req := &StreamRequest{
			DeviceID:     "device-123",
			RecordingID:  "rec-123",
			OutputFormat: "mcap0",
		}
		assert.NoError(t, req.Validate())
	})

	t.Run("fails without any source", func(t *testing.T) {
		req := &StreamRequest{
			OutputFormat: "mcap0",
		}
		err := req.Validate()
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "either recording-id/key, session-id/session-key, import-id, or device-id/device-name")
	})

	t.Run("fails when end is before start", func(t *testing.T) {
		req := &StreamRequest{
			RecordingID:  "rec-123",
			Start:        &end,
			End:          &start,
			OutputFormat: "mcap0",
		}
		err := req.Validate()
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "end must be after or equal to start")
	})
}
