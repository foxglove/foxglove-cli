package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
)

func newPendingImportsCommand(params *baseParams) *cobra.Command {
	var format string
	var requestId string
	var deviceId string
	var deviceName string
	var filename string
	var error string
	var showCompleted bool
	var showQuarantined bool
	var siteId string
	var updatedSince string
	pendingImportsCmd := &cobra.Command{
		Use:   "list",
		Short: "List the pending and errored import jobs for uploaded recordings",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			parsedUpdatedSince, err := maybeConvertToRFC3339(updatedSince)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to parse value of --updated-since: %s\n", err)
				os.Exit(1)
			}
			err = renderList(
				os.Stdout,
				console.PendingImportsRequest{
					RequestId:       requestId,
					DeviceId:        deviceId,
					DeviceName:      deviceName,
					Error:           error,
					Filename:        filename,
					UpdatedSince:    parsedUpdatedSince,
					ShowCompleted:   showCompleted,
					ShowQuarantined: showQuarantined,
					SiteId:          siteId,
				},
				client.PendingImports,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list pending imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	pendingImportsCmd.InheritedFlags()
	pendingImportsCmd.PersistentFlags().StringVarP(&requestId, "request-id", "", "", "Request ID")
	pendingImportsCmd.PersistentFlags().StringVarP(&deviceId, "device-id", "", "", "Device ID")
	pendingImportsCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	pendingImportsCmd.PersistentFlags().StringVarP(&filename, "filename", "", "", "Filename")
	pendingImportsCmd.PersistentFlags().StringVarP(&error, "error", "", "", "Filter based on error messages")
	pendingImportsCmd.PersistentFlags().BoolVarP(&showCompleted, "show-completed", "", false, "Show completed requests")
	pendingImportsCmd.PersistentFlags().BoolVarP(&showQuarantined, "show-quarantined", "", false, "Show quarantined requests")
	pendingImportsCmd.PersistentFlags().StringVarP(&siteId, "site-id", "", "", "Site ID")
	pendingImportsCmd.PersistentFlags().StringVarP(&updatedSince, "updated-since", "", "", "Filter pending imports updated since this time (ISO8601)")
	AddFormatFlag(pendingImportsCmd, &format)
	return pendingImportsCmd
}
