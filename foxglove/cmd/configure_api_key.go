package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func configureAPIKey() error {
	fmt.Printf("Enter an API key (will be written to %s):\n", viper.ConfigFileUsed())
	var bearerToken string
	fmt.Scanln(&bearerToken)
	viper.Set("bearer_token", bearerToken)
	err := viper.WriteConfigAs(viper.ConfigFileUsed())
	if err != nil {
		return fmt.Errorf("Failed to write config: %w", err)
	}
	return nil
}

func newConfigureAPIKeyCommand() *cobra.Command {
	configCmd := &cobra.Command{
		Use:   "configure-api-key",
		Short: "Configure an API key",
		Run: func(cmd *cobra.Command, args []string) {
			err := configureAPIKey()
			if err != nil {
				fatalf("Configuration failed: %s\n", err)
			}
		},
	}
	configCmd.InheritedFlags()
	return configCmd
}
