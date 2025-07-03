package cmd

import (
	"context"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestAddDeviceCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.Nil(t, err)

	t.Run("creates a device", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		dev, err := client.CreateDevice(api.CreateDeviceRequest{
			Name:       "new-device",
			ProjectID:  "prj_1234abcd",
			Properties: map[string]interface{}{"key": "val"},
		})
		assert.Nil(t, err)

		assert.Contains(t, sv.RegisteredDevices(), api.DevicesResponse{
			ID:         dev.ID,
			Name:       dev.Name,
			ProjectID:  dev.ProjectID,
			Properties: dev.Properties,
		})
	})
}

func TestEditDeviceCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.Nil(t, err)

	t.Run("creates a device", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		dev, err := client.EditDevice("test-device", api.CreateDeviceRequest{
			Name:       "new-name",
			Properties: map[string]interface{}{"key": "val"},
		})
		assert.Nil(t, err)

		assert.Equal(t, dev.Name, "new-name")
		assert.Equal(t, dev.Properties, map[string]interface{}{"key": "val"})
	})
}
