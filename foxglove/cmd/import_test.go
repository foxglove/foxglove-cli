package cmd

import (
	"context"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/stretchr/testify/assert"
)

func TestImportCommand(t *testing.T) {
	ctx := context.Background()
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		sv := svc.NewMockServer(ctx)
		err := executeImport(
			sv.BaseURL(),
			"abc",
			"test-device",
			"../testdata/gps.bag",
			"",
			"user-agent",
		)
		assert.Equal(t, "Forbidden. Have you signed in with `foxglove login`?", err.Error())
	})
	t.Run("returns friendly message when device is not registered", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		sv := svc.NewMockServer(ctx)
		client := svc.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		err = executeImport(
			sv.BaseURL(),
			"abc",
			"unregistered-device",
			"../testdata/gps.bag",
			token,
			"user-agent",
		)
		assert.Equal(t, "Device not registered with this organization", err.Error())
	})
}
