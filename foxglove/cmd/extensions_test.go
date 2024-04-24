package cmd

import (
	"context"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestPublishExtensionCommand(t *testing.T) {
	ctx := context.Background()
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewRemoteFoxgloveClient(
			sv.BaseURL(),
			"client",
			"token",
			"user-agent",
		)
		err = executeExtensionUpload(client, "../testdata/fg.mock-0.0.0.foxe")
		assert.ErrorIs(t, err, api.ErrForbidden)
	})
	t.Run("returns friendly error for unexpected file extension", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewRemoteFoxgloveClient(
			sv.BaseURL(),
			"client",
			"token",
			"user-agent",
		)
		err = executeExtensionUpload(client, "../testdata/gps.bag")
		assert.EqualError(t, err, "file should have a '.foxe' extension")
	})
}

func TestUnpublishExtensionCommand(t *testing.T) {
	ctx := context.Background()
	t.Run("returns ok if deleted", func(t *testing.T) {
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		err = executeExtensionDelete(
			client,
			sv.ValidExtensionId(),
		)
		assert.Nil(t, err)
	})
	t.Run("does not error if extension not found", func(t *testing.T) {
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		err = executeExtensionDelete(
			client,
			"nonexistent-extension-id",
		)
		assert.Nil(t, err)
	})
}
