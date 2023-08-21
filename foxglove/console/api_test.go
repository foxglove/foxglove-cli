package console

import (
	"testing"

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
