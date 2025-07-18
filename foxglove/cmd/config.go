package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newConfigCommand() *cobra.Command {
	configCmd := &cobra.Command{
		Use:   "config",
		Short: "Manage CLI configuration",
		Long: `Manage CLI configuration values.

Configuration is stored in the config file (default: $HOME/.foxgloverc).
Available configuration keys:
  - project-id: Default project ID for commands
  - api-key: API key for authentication`,
	}

	configCmd.AddCommand(newConfigGetCommand())
	configCmd.AddCommand(newConfigSetCommand())

	return configCmd
}

func newConfigGetCommand() *cobra.Command {
	getCmd := &cobra.Command{
		Use:   "get [KEY]",
		Short: "Get a configuration value",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			key := args[0]

			if !isValidConfigKey(key) {
				dief("Invalid configuration key '%s'. Valid keys are: project-id, api-key", key)
			}

			viperKey := mapConfigKeyToViperKey(key)
			value := viper.GetString(viperKey)

			if value == "" {
				if viper.IsSet(viperKey) {
					fmt.Println()
				} else {
					dief("Configuration key '%s' not found", key)
				}
			} else {
				fmt.Println(value)
			}
		},
	}
	return getCmd
}

func newConfigSetCommand() *cobra.Command {
	setCmd := &cobra.Command{
		Use:   "set [KEY] [VALUE]",
		Short: "Set a configuration value",
		Args:  cobra.MaximumNArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			if len(args) < 1 {
				dief("Configuration key is required")
			}

			key := args[0]
			if !isValidConfigKey(key) {
				dief("Invalid configuration key '%s'. Valid keys are: project-id, api-key", key)
			}

			var value string
			if len(args) == 2 {
				value = args[1]
			} else {
				// If the value is not provided, prompt for an input
				switch key {
				case "project-id":
					value = promptForInput(fmt.Sprintf("Enter project ID (will be written to %s):\n", viper.ConfigFileUsed()))
				case "api-key":
					value = promptForInput(fmt.Sprintf("Enter an API key (will be written to %s):\n", viper.ConfigFileUsed()))
				default:
					dief("Invalid configuration key '%s'. Valid keys are: project-id, api-key", key)
				}
			}

			viperKey := mapConfigKeyToViperKey(key)

			viper.Set(viperKey, value)

			// For api-key, also set auth_type to indicate it's an API key
			if key == "api-key" {
				viper.Set("auth_type", TokenApiKey)
			}

			err := viper.WriteConfigAs(viper.ConfigFileUsed())
			if err != nil {
				dief("Failed to write config: %s", err)
			}

			fmt.Fprintf(os.Stderr, "Configuration updated: %s = %s\n", key, value)
		},
	}
	return setCmd
}

func isValidConfigKey(key string) bool {
	validKeys := map[string]bool{
		"project-id": true,
		"api-key":    true,
	}
	return validKeys[key]
}

func mapConfigKeyToViperKey(key string) string {
	switch key {
	case "project-id":
		return "default_project_id"
	case "api-key":
		return "bearer_token"
	default:
		return key
	}
}
