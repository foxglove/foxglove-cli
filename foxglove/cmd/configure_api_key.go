package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newConfigureAPIKeyCommand() *cobra.Command {
	var token string
	var baseURL string
	configCmd := &cobra.Command{
		Use:   "configure-api-key",
		Short: "Configure an API key",
		Run: func(cmd *cobra.Command, args []string) {
			if token == "" {
				prompt := fmt.Sprintf("Enter an API key (will be written to %s):\n", viper.ConfigFileUsed())
				token = promptForInput(prompt)
			}
			err := configureAuth(token, defaultString(baseURL, defaultBaseURL))
			if err != nil {
				dief("Configuration failed: %s\n", err)
			}
		},
	}
	configCmd.PersistentFlags().StringVarP(&token, "api-key", "", "", "api key (for non-interactive use)")
	configCmd.PersistentFlags().StringVarP(&baseURL, "base-url", "", defaultBaseURL, "console API server")
	configCmd.InheritedFlags()
	return configCmd
}
