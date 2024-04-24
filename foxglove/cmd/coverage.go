package cmd

import (
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
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
	var isJsonFormat bool
	coverageListCmd := &cobra.Command{
		Use:   "list",
		Short: "List coverage ranges",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			// We accept ISO8601, which is a little more lenient than the API. Here
			// we convert to RFC3339.
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
				&api.CoverageRequest{
					DeviceID:              deviceID,
					DeviceName:            deviceName,
					Start:                 startTime,
					End:                   endTime,
					RecordingID:           recordingID,
					Tolerance:             tolerance,
					IncludeEdgeRecordings: includeEdgeRecordings,
				},
				client.Coverage,
				format,
			)
			if err != nil {
				dief("Failed to list coverage: %s", err)
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
	coverageListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of coverage time range (ISO8601)")
	coverageListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of coverage time range (ISO8601)")
	AddFormatFlag(coverageListCmd, &format)
	AddDeviceAutocompletion(coverageListCmd, params)
	AddJsonFlag(coverageListCmd, &isJsonFormat)
	return coverageListCmd
}
