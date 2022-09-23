package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// versionCmd represents the version command
func newVersionCommand(version string) *cobra.Command {
	versionCmd := &cobra.Command{
		Use:   "version",
		Short: "print Foxglove CLI version",
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println(version)
		},
	}
	return versionCmd
}
