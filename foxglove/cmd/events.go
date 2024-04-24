package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/spf13/cobra"
)

func newAddEventCommand(params *baseParams) *cobra.Command {
	var deviceID string
	var start string
	var end string
	var keyvals []string
	addEventCmd := &cobra.Command{
		Use:   "add",
		Short: "Add an event",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			metadata := make(map[string]string)
			for _, kv := range keyvals {
				key, val, err := util.SplitPair(kv, ':')
				if err != nil {
					fmt.Fprintf(os.Stderr, "Invalid key/value pair: %s\n", kv)
					os.Exit(1)
				}
				metadata[key] = val
			}
			response, err := client.CreateEvent(api.CreateEventRequest{
				DeviceID: deviceID,
				Start:    start,
				End:      end,
				Metadata: metadata,
			})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to add event: %s\n", err)
				os.Exit(1)
			}
			fmt.Fprintf(os.Stderr, "Created event: %s\n", response.ID)
		},
	}
	addEventCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	addEventCmd.PersistentFlags().StringVarP(&start, "start", "", "", "Start of event, RFC 3339 date-time format")
	addEventCmd.PersistentFlags().StringVarP(&end, "end", "", "", "End of event (inclusive), RFC 3339 date-time format")
	addEventCmd.PersistentFlags().StringArrayVarP(&keyvals, "metadata", "m", []string{}, "Metadata colon-separated key value pair. Multiple may be specified.")
	return addEventCmd
}

func newListEventsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var deviceName string
	var sortBy string
	var sortOrder string
	var limit int
	var offset int
	var start string
	var end string
	var query string
	var isJsonFormat bool
	eventsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List events",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				&api.EventsRequest{
					DeviceID:   deviceID,
					DeviceName: deviceName,
					SortBy:     sortBy,
					SortOrder:  sortOrder,
					Limit:      limit,
					Offset:     offset,
					Start:      start,
					End:        end,
					Query:      query,
				},
				client.Events,
				format,
			)
			if err != nil {
				dief("Failed to list events: %s", err)
			}
		},
	}
	eventsListCmd.InheritedFlags()
	eventsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	eventsListCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	eventsListCmd.PersistentFlags().StringVarP(&sortBy, "sort-by", "", "", "name of sort column")
	eventsListCmd.PersistentFlags().StringVarP(&sortOrder, "sort-order", "", "asc", "sort order")
	eventsListCmd.PersistentFlags().IntVarP(&limit, "limit", "", 100, "limit")
	eventsListCmd.PersistentFlags().IntVarP(&offset, "offset", "", 0, "offset")
	eventsListCmd.PersistentFlags().StringVarP(&query, "query", "", "", "Filter by metadata with keyword or \"$key:$value\"")
	AddDeviceAutocompletion(eventsListCmd, params)
	AddFormatFlag(eventsListCmd, &format)
	AddJsonFlag(eventsListCmd, &isJsonFormat)
	return eventsListCmd
}
