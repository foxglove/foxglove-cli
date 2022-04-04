package cmd

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func renderEventsJSON(w io.Writer, events []svc.EventsResponse) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "    ")
	return encoder.Encode(events)
}

func renderEventsCSV(w io.Writer, events []svc.EventsResponse) error {
	writer := csv.NewWriter(w)
	err := writer.Write([]string{
		"id",
		"deviceId",
		"timestamp",
		"duration",
		"createdAt",
		"updatedAt",
		"metadata",
	})
	if err != nil {
		return err
	}
	for _, event := range events {
		metadata, err := json.Marshal(event.Metadata)
		if err != nil {
			return fmt.Errorf("failed to marshal metadata: %s", err)
		}
		err = writer.Write([]string{
			event.ID,
			event.DeviceID,
			event.TimestampNanos,
			event.DurationNanos,
			event.CreatedAt,
			event.UpdatedAt,
			string(metadata),
		})
		if err != nil {
			return err
		}
	}
	writer.Flush()
	return writer.Error()
}

func renderEventsTable(w io.Writer, events []svc.EventsResponse) error {
	table := tablewriter.NewWriter(w)
	table.SetHeader([]string{
		"ID",
		"Device ID",
		"Timestamp",
		"Duration",
		"Created At",
		"Updated At",
		"Metadata",
	})
	table.SetBorders(tablewriter.Border{
		Left:   true,
		Top:    false,
		Right:  true,
		Bottom: false,
	})
	table.SetAlignment(tablewriter.ALIGN_LEFT)
	table.SetCenterSeparator("|")
	data := [][]string{}
	for _, event := range events {
		metadata, err := json.Marshal(event.Metadata)
		if err != nil {
			return fmt.Errorf("failed to marshal metadata: %s", err)
		}
		data = append(data, []string{
			event.ID,
			event.DeviceID,
			event.TimestampNanos,
			event.DurationNanos,
			event.CreatedAt,
			event.UpdatedAt,
			string(metadata),
		})
	}
	table.AppendBulk(data)
	table.Render()
	return nil
}

func listEvents(
	w io.Writer,
	baseURL string,
	clientID,
	format, token, userAgent string,
	req *svc.EventsRequest,
) error {
	client := svc.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	events, err := client.Events(req)
	if err != nil {
		return err
	}
	switch format {
	case "table":
		err := renderEventsTable(w, events)
		if err != nil {
			return err
		}
	case "json":
		err := renderEventsJSON(w, events)
		if err != nil {
			return err
		}
	case "csv":
		err := renderEventsCSV(w, events)
		if err != nil {
			return err
		}
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
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
	var key string
	var value string
	eventsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List events",
		Run: func(cmd *cobra.Command, args []string) {
			err := listEvents(
				os.Stdout,
				*params.baseURL,
				*params.clientID,
				format,
				viper.GetString("bearer_token"),
				params.userAgent,
				&svc.EventsRequest{
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
