package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newBetaAddEventCommand(params *baseParams) *cobra.Command {
	var deviceID string
	var deviceName string
	var timestamp string
	var durationNanos string
	var keyvals []string
	addEventCmd := &cobra.Command{
		Use:   "add",
		Short: "Add an event",
		Run: func(cmd *cobra.Command, args []string) {
			printDeprecation()
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)

			metadata := make(map[string]string)
			for _, kv := range keyvals {
				parts := strings.FieldsFunc(kv, func(c rune) bool { return c == ':' })
				if len(parts) != 2 {
					fmt.Fprintf(os.Stderr, "Invalid key/value pair: %s\n", kv)
					os.Exit(1)
				}
				metadata[parts[0]] = parts[1]
			}
			response, err := client.BetaCreateEvent(console.BetaCreateEventRequest{
				DeviceID:      deviceID,
				DeviceName:    deviceName,
				Timestamp:     timestamp,
				DurationNanos: durationNanos,
				Metadata:      metadata,
			})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to add event: %s\n", err)
				os.Exit(1)
			}
			fmt.Fprintf(os.Stderr, "Created event: %s\n", response.ID)
		},
	}
	addEventCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	addEventCmd.PersistentFlags().StringVarP(&timestamp, "timestamp", "", "", "Timestamp of event (RFC3339 format)")
	addEventCmd.PersistentFlags().StringVarP(&durationNanos, "duration-nanos", "", "", "Duration of event in nanoseconds")
	addEventCmd.PersistentFlags().StringArrayVarP(&keyvals, "metadata", "m", []string{}, "Metadata colon-separated key value pair. Multiple may be specified.")
	return addEventCmd
}

func newBetaListEventsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var deviceName string
	var sortBy string
	var sortOrder string
	var limit int
	var offset int
	var start string
	var end string
	var key string
	var value string
	eventsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List events",
		Run: func(cmd *cobra.Command, args []string) {
			printDeprecation()
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&console.BetaEventsRequest{
					DeviceID:   deviceID,
					DeviceName: deviceName,
					SortBy:     sortBy,
					SortOrder:  sortOrder,
					Limit:      limit,
					Offset:     offset,
					Start:      start,
					End:        end,
					Key:        key,
					Value:      value,
				},
				client.BetaEvents,
				format,
			)
			if err != nil {
				fatalf("Failed to list events: %s\n", err)
			}
		},
	}
	eventsListCmd.InheritedFlags()
	eventsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	eventsListCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "name of device")
	eventsListCmd.PersistentFlags().StringVarP(&sortBy, "sort-by", "", "", "name of sort column")
	eventsListCmd.PersistentFlags().StringVarP(&sortOrder, "sort-order", "", "asc", "sort order")
	eventsListCmd.PersistentFlags().IntVarP(&limit, "limit", "", 100, "limit")
	eventsListCmd.PersistentFlags().IntVarP(&offset, "offset", "", 0, "offset")
	eventsListCmd.PersistentFlags().StringVarP(&key, "key", "", "", "return events with matching metadata keys")
	eventsListCmd.PersistentFlags().StringVarP(&value, "value", "", "", "return events with matching metadata values")
	AddFormatFlag(eventsListCmd, &format)
	return eventsListCmd
}

func printDeprecation() {
	fmt.Fprintln(os.Stderr, "[Warning] this command is deprecated and will be removed in a future version. Use `foxglove events`.")
}
