package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

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
	var key string
	var value string
	eventsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List events",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&console.EventsRequest{
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
				client.Events,
				format,
			)
			if err != nil {
				fmt.Printf("Failed to list events: %s\n", err)
				os.Exit(1)
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
	eventsListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return eventsListCmd
}
