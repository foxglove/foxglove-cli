package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newConfigureProjectIDCommand() *cobra.Command {
	var projectID string
	var clearProjectID bool
	configCmd := &cobra.Command{
		Use:   "configure-project-id",
		Short: "Set the default project ID",
		Run: func(cmd *cobra.Command, args []string) {
			if projectID == "" && !clearProjectID {
				prompt := fmt.Sprintf("Enter a project ID (will be written to %s):\n", viper.ConfigFileUsed())
				projectID = promptForInput(prompt)
			}
			if clearProjectID {
				projectID = ""
			}
			err := configureProjectID(projectID)
			if err != nil {
				dief("Configuration failed: %s", err)
			}
		},
	}
	configCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", "", "default project ID (for non-interactive use)")
	configCmd.PersistentFlags().BoolVarP(&clearProjectID, "clear-project-id", "", false, "clear the default project ID")
	configCmd.InheritedFlags()
	return configCmd
}
