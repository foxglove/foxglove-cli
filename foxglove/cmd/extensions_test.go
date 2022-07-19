package cmd

import (
	"context"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/stretchr/testify/assert"
)

func TestUploadExtensionCommand(t *testing.T) {
	ctx := context.Background()
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExtensionUpload(
			sv.BaseURL(),
			"client",
			"token",
			"../testdata/fg.mock-0.0.0.foxe",
			"user-agent",
		)
		assert.ErrorIs(t, err, console.ErrForbidden)
	})
	t.Run("returns friendly error for unexpected file extension", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExtensionUpload(
			sv.BaseURL(),
			"client",
			"token",
			"../testdata/gps.bag",
			"user-agent",
		)
		assert.EqualError(t, err, "file should have a '.foxe' extension")
	})
}
