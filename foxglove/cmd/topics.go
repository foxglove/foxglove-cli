package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListTopicsCommand(params *baseParams) *cobra.Command {
	var request api.TopicsRequest
	var format string
	var isJSONFormat bool
	topicCmd := &cobra.Command{
		Use:   "list",
		Short: "List topics",
		Long: `List topics.

One of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key is required.`,
		Run: func(cmd *cobra.Command, args []string) {
			if err := validateTopicsRequest(request); err != nil {
				dief("%s", err)
			}
			if err := validateSessionKeyRequiresProjectID(request.SessionKey, request.ProjectID); err != nil {
				dief("%s", err)
			}
			start, err := maybeConvertToRFC3339(request.Start)
			if err != nil {
				dief("failed to parse start time: %s", err)
			}
			end, err := maybeConvertToRFC3339(request.End)
			if err != nil {
				dief("failed to parse end time: %s", err)
			}
			request.Start = start
			request.End = end
			format = ResolveFormat(format, isJSONFormat)

			client := api.NewRemoteFoxgloveClient(
				params.baseURL,
				*params.clientID,
				params.token,
				params.userAgent,
			)
			if err := renderList(os.Stdout, &request, client.Topics, format); err != nil {
				dief("Failed to list topics: %s", err)
			}
		},
	}

	topicCmd.PersistentFlags().StringVar(&request.ProjectID, "project-id", viper.GetString("default_project_id"), "Project ID (required when using --session-key)")
	topicCmd.PersistentFlags().StringVar(&request.DeviceID, "device-id", "", "Device ID")
	topicCmd.PersistentFlags().StringVar(&request.DeviceName, "device-name", "", "Device name")
	topicCmd.PersistentFlags().StringVar(&request.RecordingID, "recording-id", "", "Recording ID")
	topicCmd.PersistentFlags().StringVar(&request.RecordingKey, "recording-key", "", "Recording key")
	topicCmd.PersistentFlags().StringVar(&request.SessionID, "session-id", "", "Session ID")
	topicCmd.PersistentFlags().StringVar(&request.SessionKey, "session-key", "", "Session key")
	topicCmd.PersistentFlags().StringVar(&request.Start, "start", "", "Start of topic time range (ISO8601)")
	topicCmd.PersistentFlags().StringVar(&request.End, "end", "", "End of topic time range (ISO8601)")
	topicCmd.PersistentFlags().BoolVar(&request.IncludeSchemas, "include-schemas", false, "Include full topic schemas")
	topicCmd.PersistentFlags().StringVar(&request.SortBy, "sort-by", "", "Sort by topic or version")
	topicCmd.PersistentFlags().StringVar(&request.SortOrder, "sort-order", "", "Sort order (asc or desc)")
	topicCmd.PersistentFlags().IntVar(&request.Limit, "limit", 0, "Maximum number of topics to return")
	topicCmd.PersistentFlags().IntVar(&request.Offset, "offset", 0, "Number of topics to skip")
	AddFormatFlag(topicCmd, &format)
	AddDeviceAutocompletion(topicCmd, params)
	AddJsonFlag(topicCmd, &isJSONFormat)
	return topicCmd
}

func validateTopicsRequest(request api.TopicsRequest) error {
	if request.DeviceID == "" && request.DeviceName == "" && request.RecordingID == "" &&
		request.RecordingKey == "" && request.SessionID == "" && request.SessionKey == "" {
		return fmt.Errorf("provide one of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key")
	}
	return nil
}
