package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
)

func newListRecordingsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var deviceName string
	var primarySiteID string
	var projectID string
	var edgeSiteID string
	var path string
	var start string
	var end string
	var importStatus string
	var limit int
	var offset int
	var sortBy string
	var sortOrder string
	var isJsonFormat bool
	recordingsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List recordings",
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
			format = ResolveFormat(format, isJsonFormat)
			err = renderList(
				os.Stdout,
				&api.RecordingsRequest{
					DeviceID:     deviceID,
					DeviceName:   deviceName,
					ProjectID:    projectID,
					Start:        startTime,
					End:          endTime,
					Path:         path,
					SiteID:       primarySiteID,
					EdgeSiteID:   edgeSiteID,
					ImportStatus: importStatus,
					Limit:        limit,
					Offset:       offset,
					SortBy:       sortBy,
					SortOrder:    sortOrder,
				},
				client.Recordings,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list recordings: %s\n", err)
				os.Exit(1)
			}
		},
	}
	recordingsListCmd.InheritedFlags()
	recordingsListCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", "", "project ID")
	recordingsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	recordingsListCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "device name")
	recordingsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of data range (ISO8601 format)")
	recordingsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of data range (ISO8601 format)")
	recordingsListCmd.PersistentFlags().StringVarP(&path, "path", "", "", "recording file path")
	recordingsListCmd.PersistentFlags().StringVarP(&primarySiteID, "site-id", "", "", "primary site ID")
	recordingsListCmd.PersistentFlags().StringVarP(&edgeSiteID, "edge-site-id", "", "", "edge site ID")
	recordingsListCmd.PersistentFlags().StringVarP(&importStatus, "import-status", "", "", "import status")
	recordingsListCmd.PersistentFlags().IntVarP(&limit, "limit", "", 2000, "max number of recordings to return")
	recordingsListCmd.PersistentFlags().IntVarP(&offset, "offset", "", 0, "number of recordings to skip")
	recordingsListCmd.PersistentFlags().StringVarP(&sortBy, "sort-by", "", "", "sort recordings by a field")
	recordingsListCmd.PersistentFlags().StringVarP(&sortOrder, "sort-order", "", "", "sort order: 'asc' 'desc'")
	AddFormatFlag(recordingsListCmd, &format)
	AddDeviceAutocompletion(recordingsListCmd, params)
	AddJsonFlag(recordingsListCmd, &isJsonFormat)
	return recordingsListCmd
}

func newDeleteRecordingCommand(params *baseParams) *cobra.Command {
	deleteCmd := &cobra.Command{
		Use:   "delete [ID]",
		Short: "Delete a recording from your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(params.baseURL, *params.clientID, params.token, params.userAgent)
			if err := client.DeleteRecording(args[0]); err != nil {
				dief("Failed to delete recording: %s", err)
			}
		},
	}
	deleteCmd.InheritedFlags()
	return deleteCmd
}
