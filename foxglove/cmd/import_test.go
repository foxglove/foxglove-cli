package cmd

import (
	"context"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/stretchr/testify/assert"
)

func TestImportCommand(t *testing.T) {
	ctx := context.Background()
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeImport(
			sv.BaseURL(),
			"abc",
			"test-device",
			"",
			"../testdata/gps.bag",
			"",
			"user-agent",
		)
		assert.ErrorIs(t, err, console.ErrForbidden)
	})
	t.Run("returns friendly message when device is not registered", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		client := console.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		err = executeImport(
			sv.BaseURL(),
			"abc",
			"unregistered-device",
			"",
			"../testdata/gps.bag",
			token,
			"user-agent",
		)
		assert.Equal(t, "Device not registered with this organization", err.Error())
	})
}
