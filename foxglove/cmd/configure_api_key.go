package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func setBearerToken(token string) error {
	viper.Set("bearer_token", token)
	err := viper.WriteConfigAs(viper.ConfigFileUsed())
	if err != nil {
		return fmt.Errorf("Failed to write config: %w", err)
	}
	return nil
}

func newConfigureAPIKeyCommand() *cobra.Command {
	var token string
	configCmd := &cobra.Command{
		Use:   "configure-api-key",
		Short: "Configure an API key",
		Run: func(cmd *cobra.Command, args []string) {
			if token == "" {
				prompt := fmt.Sprintf("Enter an API key (will be written to %s):\n", viper.ConfigFileUsed())
				token = promptForInput(prompt)
			}
			err := setBearerToken(token)
			if err != nil {
				fatalf("Configuration failed: %s\n", err)
			}
		},
	}
	configCmd.PersistentFlags().StringVarP(&token, "api-key", "", "", "api key (for non-interactive use)")
	configCmd.InheritedFlags()
	return configCmd
}
