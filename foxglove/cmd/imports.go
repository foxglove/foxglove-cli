package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListImportsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var start string
	var end string
	var dataStart string
	var includeDeleted bool
	var dataEnd string
	importsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List imports for a device",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&console.ImportsRequest{
					DeviceID:       deviceID,
					Start:          start,
					End:            end,
					DataStart:      dataStart,
					DataEnd:        dataEnd,
					IncludeDeleted: includeDeleted,
				},
				client.Imports,
				format,
			)
			if err != nil {
				fmt.Printf("Failed to list imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	importsListCmd.InheritedFlags()
	importsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	importsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of import time range")
	importsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of import time range")
	importsListCmd.PersistentFlags().StringVarP(&dataStart, "data-start", "", "", "start of data time range")
	importsListCmd.PersistentFlags().StringVarP(&dataEnd, "data-end", "", "", "end of data time range")
	importsListCmd.PersistentFlags().BoolVarP(&includeDeleted, "include-deleted", "", false, "end of data time range")
	importsListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return importsListCmd
}
