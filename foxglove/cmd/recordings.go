package cmd

import (
	"fmt"
	"os"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/relvacode/iso8601"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListRecordingsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var primarySiteID string
	var edgeSiteID string
	var path string
	var start string
	var end string
	var importStatus string
	recordingsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List recordings",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			var err error
			var startTime string
			var endTime string
			if start != "" {
				parsed, err := iso8601.ParseString(start)
				if err != nil {
					dief("failed to parse start time: %s", err)
				}
				startTime = parsed.Format(time.RFC3339)
			}
			if end != "" {
				parsed, err := iso8601.ParseString(end)
				if err != nil {
					dief("failed to parse end time: %s", err)
				}
				endTime = parsed.Format(time.RFC3339)
			}
			err = renderList(
				os.Stdout,
				&console.RecordingsRequest{
					DeviceID:     deviceID,
					Start:        startTime,
					End:          endTime,
					Path:         path,
					SiteID:       primarySiteID,
					EdgeSiteID:   edgeSiteID,
					ImportStatus: importStatus,
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
	recordingsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	recordingsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of data range")
	recordingsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of data range")
	recordingsListCmd.PersistentFlags().StringVarP(&path, "path", "", "", "recording file path")
	recordingsListCmd.PersistentFlags().StringVarP(&primarySiteID, "site-id", "", "", "primary site ID")
	recordingsListCmd.PersistentFlags().StringVarP(&edgeSiteID, "edge-site-id", "", "", "edge site ID")
	recordingsListCmd.PersistentFlags().StringVarP(&importStatus, "import-status", "", "", "import status")
	AddFormatFlag(recordingsListCmd, &format)
	return recordingsListCmd
}
