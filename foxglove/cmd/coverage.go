package cmd

import (
	"os"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/relvacode/iso8601"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListCoverageCommand(params *baseParams) *cobra.Command {
	var deviceName string
	var deviceID string
	var format string
	var start string
	var end string
	var tolerance int
	var recordingID string
	var includeEdgeRecordings bool
	coverageListCmd := &cobra.Command{
		Use:   "list",
		Short: "List coverage ranges",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			var startTime, endTime string
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

			err := renderList(
				os.Stdout,
				&console.CoverageRequest{
					DeviceID:    deviceID,
					DeviceName:  deviceName,
					Start:       startTime,
					End:         endTime,
					RecordingID: recordingID,
				},
				client.Coverage,
				format,
			)
			if err != nil {
				fatalf("Failed to list coverage: %s\n", err)
			}
		},
	}
	coverageListCmd.InheritedFlags()
	coverageListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	coverageListCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	coverageListCmd.PersistentFlags().StringVarP(&recordingID, "recording-id", "", "", "Recording ID")
	coverageListCmd.PersistentFlags().IntVarP(&tolerance, "tolerance", "", 0,
		"Number of seconds by which ranges must be separated to be considered distinct")

	coverageListCmd.PersistentFlags().BoolVarP(&includeEdgeRecordings, "include-edge-recordings", "", false, "Include edge recordings")
	coverageListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of coverage time range")
	coverageListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of coverage time range")
	AddFormatFlag(coverageListCmd, &format)
	return coverageListCmd
}
