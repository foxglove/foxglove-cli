package cmd

import (
	"context"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/stretchr/testify/assert"
)

func TestAddDeviceCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := console.NewMockServer(ctx)
	assert.Nil(t, err)

	t.Run("creates a device", func(t *testing.T) {
		client := console.NewMockAuthedClient(t, sv.BaseURL())
		dev, err := client.CreateDevice(console.CreateDeviceRequest{
			Name:       "new-device",
			Properties: map[string]interface{}{"key": "val"},
		})
		assert.Nil(t, err)

		assert.Contains(t, sv.RegisteredDevices(), console.DevicesResponse{
			ID:         dev.ID,
			Name:       dev.Name,
			Properties: dev.Properties,
		})
	})
}
