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
	pendingImportsCmd := &cobra.Command{
		Use:   "list",
		Short: "List the pending imports. These are in-progess import jobs for newly uploaded recordings.",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				console.PendingImportsRequest{
					RequestId:       requestId,
					DeviceId:        deviceId,
					DeviceName:      deviceName,
					Error:           error,
					Filename:        filename,
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
	pendingImportsCmd.PersistentFlags().BoolVarP(&showCompleted, "show-completed", "", false, "show completed requests")
	pendingImportsCmd.PersistentFlags().BoolVarP(&showQuarantined, "show-quarantined", "", false, "show quarantined requests")
	pendingImportsCmd.PersistentFlags().StringVarP(&siteId, "site-id", "", "", "Site ID")
	pendingImportsCmd.PersistentFlags().StringVarP(&error, "error", "", "", "filter based on error messages")
	AddFormatFlag(pendingImportsCmd, &format)
	return pendingImportsCmd
}
