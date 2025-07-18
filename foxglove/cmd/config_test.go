package cmd

import (
	"testing"

	"github.com/spf13/viper"
	"github.com/stretchr/testify/assert"
)

func TestConfigCommand(t *testing.T) {
	t.Run("setting api-key also sets auth_type", func(t *testing.T) {
		// Create a temporary config file for testing
		tempConfig := t.TempDir() + "/test-config.yaml"

		viper.Reset()
		viper.SetConfigType("yaml")
		viper.SetConfigFile(tempConfig)

		cmd := newConfigSetCommand()
		cmd.SetArgs([]string{"api-key", "test-api-key-123"})
		err := cmd.Execute()
		assert.Nil(t, err)

		err = viper.ReadInConfig()
		assert.Nil(t, err)

		assert.Equal(t, "test-api-key-123", viper.GetString("bearer_token"))
		// Check that the api key update also changes the auth_type to TokenApiKey
		settings := viper.AllSettings()
		assert.Equal(t, TokenApiKey, settings["auth_type"].(AuthType))
	})
}
