package cmd

import (
	"context"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestEditSessionCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.Nil(t, err)

	t.Run("updates a session key", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		created, err := client.CreateSession(api.CreateSessionRequest{
			Name:      "test-session",
			ProjectID: "prj_1234abcd",
			DeviceID:  "dev_1234abcd",
		})
		assert.Nil(t, err)
		assert.NotEmpty(t, created.Key)

		updated, err := client.PatchSession(created.ID, created.ProjectID, api.PatchSessionRequest{
			Key: "new-session-key",
		})
		assert.Nil(t, err)
		assert.Equal(t, created.ID, updated.ID)
		assert.Equal(t, "new-session-key", updated.Key)

		fetched, err := client.GetSession("new-session-key", created.ProjectID)
		assert.Nil(t, err)
		assert.Equal(t, created.ID, fetched.ID)
		assert.Equal(t, "new-session-key", fetched.Key)

		_, err = client.GetSession(created.Key, created.ProjectID)
		assert.ErrorIs(t, err, api.ErrNotFound)

		fetchedByID, err := client.GetSession(created.ID, created.ProjectID)
		assert.Nil(t, err)
		assert.Equal(t, "new-session-key", fetchedByID.Key)
	})

	t.Run("adds recordings without changing the key", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		created, err := client.CreateSession(api.CreateSessionRequest{
			Name:      "recordings-session",
			ProjectID: "prj_1234abcd",
			DeviceID:  "dev_1234abcd",
		})
		assert.Nil(t, err)

		updated, err := client.PatchSession(created.ID, created.ProjectID, api.PatchSessionRequest{
			AddRecordingIDs: []string{"rec_1"},
		})
		assert.Nil(t, err)
		assert.Equal(t, created.Key, updated.Key)

		fetched, err := client.GetSession(created.ID, created.ProjectID)
		assert.Nil(t, err)
		assert.Equal(t, created.Key, fetched.Key)
		assert.Equal(t, []api.SessionRecordingSummary{{ID: "rec_1"}}, fetched.Recordings)
	})
}
