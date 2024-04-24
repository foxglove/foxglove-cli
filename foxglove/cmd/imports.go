package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
)

func newListImportsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var start string
	var end string
	var dataStart string
	var includeDeleted bool
	var dataEnd string
	var isJsonFormat bool
	importsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List imports for a device",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			startTime, err := maybeConvertToRFC3339(start)
			if err != nil {
				dief("failed to parse start time: %s", err)
			}
			endTime, err := maybeConvertToRFC3339(end)
			if err != nil {
				dief("failed to parse end time: %s", err)
			}
			dataStartTime, err := maybeConvertToRFC3339(dataStart)
			if err != nil {
				dief("failed to parse data start time: %s", err)
			}
			dataEndTime, err := maybeConvertToRFC3339(dataEnd)
			if err != nil {
				dief("failed to parse data end time: %s", err)
			}
			format = ResolveFormat(format, isJsonFormat)
			err = renderList(
				os.Stdout,
				&api.ImportsRequest{
					DeviceID:       deviceID,
					Start:          startTime,
					End:            endTime,
					DataStart:      dataStartTime,
					DataEnd:        dataEndTime,
					IncludeDeleted: includeDeleted,
				},
				client.Imports,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	importsListCmd.InheritedFlags()
	importsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	importsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of import time range (ISO8601)")
	importsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of import time range (ISO8601)")
	importsListCmd.PersistentFlags().StringVarP(&dataStart, "data-start", "", "", "start of data time range (ISO8601)")
	importsListCmd.PersistentFlags().StringVarP(&dataEnd, "data-end", "", "", "end of data time range (ISO8601)")
	importsListCmd.PersistentFlags().BoolVarP(&includeDeleted, "include-deleted", "", false, "end of data time range")
	AddFormatFlag(importsListCmd, &format)
	AddJsonFlag(importsListCmd, &isJsonFormat)
	return importsListCmd
}
