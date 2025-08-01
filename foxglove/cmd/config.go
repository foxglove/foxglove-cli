package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newConfigCommand() *cobra.Command {
	configCmd := &cobra.Command{
		Use:   "config",
		Short: "Manage CLI configuration",
		Long: `Manage CLI configuration values.
Available configuration keys:
  - project-id: Default project ID for commands`,
	}

	configCmd.AddCommand(newConfigGetCommand())
	configCmd.AddCommand(newConfigSetCommand())
	configCmd.AddCommand(newConfigUnsetCommand())

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
				dief("Invalid configuration key '%s'. Valid keys are: %s", key, strings.Join(validConfigKeys, ", "))
			}

			viperKey := mapConfigKeyToViperKey(key)
			value := viper.GetString(viperKey)

			if value == "" {
				if viper.IsSet(viperKey) {
					fmt.Println()
				} else {
					dief("No value set for key '%s'", key)
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
		Args:  cobra.ExactArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			if len(args) < 1 {
				dief("Configuration key is required")
			}

			key := args[0]
			value := args[1]

			if !isValidConfigKey(key) {
				dief("Invalid configuration key '%s'. Valid keys are: %s", key, strings.Join(validConfigKeys, ", "))
			}

			viperKey := mapConfigKeyToViperKey(key)

			viper.Set(viperKey, value)

			err := viper.WriteConfigAs(viper.ConfigFileUsed())
			if err != nil {
				dief("Failed to write config: %s", err)
			}

			fmt.Fprintf(os.Stderr, "Configuration updated: %s = %s\n", key, value)
		},
	}
	return setCmd
}

func newConfigUnsetCommand() *cobra.Command {
	unsetCmd := &cobra.Command{
		Use:   "unset [KEY]",
		Short: "Remove a configuration value",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			key := args[0]

			if !isValidConfigKey(key) {
				dief("Invalid configuration key '%s'. Valid keys are: %s", key, strings.Join(validConfigKeys, ", "))
			}
			viperKey := mapConfigKeyToViperKey(key)
			if !viper.IsSet(viperKey) {
				dief("No value set for key '%s'", key)
			}
			configFile := viper.ConfigFileUsed()
			settings := viper.AllSettings()
			delete(settings, viperKey)

			// Clear viper and reload with updated settings
			viper.Reset()
			viper.SetConfigType("yaml")
			viper.SetConfigFile(configFile)
			for k, v := range settings {
				viper.Set(k, v)
			}

			err := viper.WriteConfigAs(configFile)
			if err != nil {
				dief("Failed to write config: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Configuration removed: %s\n", key)
		},
	}
	return unsetCmd
}

var validConfigKeys = []string{
	"project-id",
}

func isValidConfigKey(key string) bool {
	for _, validKey := range validConfigKeys {
		if key == validKey {
			return true
		}
	}
	return false
}

func mapConfigKeyToViperKey(key string) string {
	switch key {
	case "project-id":
		return "default_project_id"
	default:
		return key
	}
}
