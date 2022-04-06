package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListDevicesCommand(params *baseParams) *cobra.Command {
	var format string
	deviceListCmd := &cobra.Command{
		Use:   "list",
		Short: "List devices registered to your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				console.DevicesRequest{},
				client.Devices,
				format,
			)
			if err != nil {
				fmt.Printf("Failed to list devices: %s\n", err)
			}
		},
	}
	deviceListCmd.InheritedFlags()
	deviceListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return deviceListCmd
}
