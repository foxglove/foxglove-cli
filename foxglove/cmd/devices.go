package cmd

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func renderDevicesJSON(w io.Writer, devices []svc.DeviceResponse) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "    ")
	return encoder.Encode(devices)
}

func renderDevicesCSV(w io.Writer, devices []svc.DeviceResponse) error {
	writer := csv.NewWriter(w)
	err := writer.Write([]string{
		"id",
		"name",
		"createdAt",
		"updatedAt",
	})
	if err != nil {
		return err
	}
	for _, device := range devices {
		err := writer.Write([]string{
			device.ID,
			device.Name,
			device.CreatedAt.Format(time.RFC3339),
			device.UpdatedAt.Format(time.RFC3339),
		})
		if err != nil {
			return err
		}
	}
	writer.Flush()
	return writer.Error()
}

func renderDevicesTable(w io.Writer, devices []svc.DeviceResponse) {
	table := tablewriter.NewWriter(w)
	table.SetHeader([]string{
		"ID",
		"Name",
		"Created At",
		"Updated At",
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
	for _, device := range devices {
		data = append(data, []string{
			device.ID,
			device.Name,
			device.CreatedAt.Format(time.RFC3339),
			device.UpdatedAt.Format(time.RFC3339),
		})
	}
	table.AppendBulk(data)
	table.Render()
}

func listDevices(
	w io.Writer,
	baseURL,
	clientID,
	format, token, userAgent string,
) error {
	client := svc.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	devices, err := client.Devices()
	if err != nil {
		return err
	}
	switch format {
	case "table":
		renderDevicesTable(w, devices)
	case "json":
		err := renderDevicesJSON(w, devices)
		if err != nil {
			return err
		}
	case "csv":
		err := renderDevicesCSV(w, devices)
		if err != nil {
			return err
		}
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
}

func newListDevicesCommand(params *baseParams) *cobra.Command {
	var format string
	deviceListCmd := &cobra.Command{
		Use:   "list",
		Short: "List devices registered to your organization",
		Run: func(cmd *cobra.Command, args []string) {
			err := listDevices(
				os.Stdout,
				*params.baseURL,
				*params.clientID,
				format,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			if err != nil {
				fmt.Printf("Failed to list devices: %s\n", err)
			}
		},
	}
	deviceListCmd.InheritedFlags()
	deviceListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return deviceListCmd
}
