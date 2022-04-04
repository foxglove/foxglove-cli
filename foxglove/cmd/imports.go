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

func renderImportsTable(w io.Writer, imports []svc.ImportsResponse) {
	table := tablewriter.NewWriter(w)
	table.SetHeader([]string{
		"Import ID",
		"Device ID",
		"Filename",
		"Import Time",
		"Start",
		"End",
		"Input Type",
		"Output Type",
		"Input Size",
		"Total Output Size",
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
	for _, imp := range imports {
		data = append(data, []string{
			imp.ImportID,
			imp.DeviceID,
			imp.Filename,
			imp.ImportTime.Format(time.RFC3339),
			imp.Start.Format(time.RFC3339),
			imp.End.Format(time.RFC3339),
			imp.InputType,
			imp.OutputType,
			fmt.Sprintf("%d", imp.InputSize),
			fmt.Sprintf("%d", imp.TotalOutputSize),
		})
	}
	table.AppendBulk(data)
	table.Render()
}

func renderImportsCSV(w io.Writer, imports []svc.ImportsResponse) error {
	writer := csv.NewWriter(w)
	err := writer.Write([]string{
		"importId",
		"deviceId",
		"filename",
		"importTime",
		"start",
		"end",
		"inputType",
		"outputType",
		"inputSize",
		"totalOutputSize",
	})
	if err != nil {
		return err
	}
	for _, imp := range imports {
		err := writer.Write([]string{
			imp.ImportID,
			imp.DeviceID,
			imp.Filename,
			imp.ImportTime.Format(time.RFC3339),
			imp.Start.Format(time.RFC3339),
			imp.End.Format(time.RFC3339),
			imp.InputType,
			imp.OutputType,
			fmt.Sprintf("%d", imp.InputSize),
			fmt.Sprintf("%d", imp.TotalOutputSize),
		})
		if err != nil {
			return err
		}
	}
	writer.Flush()
	return writer.Error()
}

func renderImportsJSON(w io.Writer, imports []svc.ImportsResponse) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "    ")
	return encoder.Encode(imports)
}

func listImports(
	w io.Writer,
	baseURL,
	clientID,
	format, token, userAgent string,
	req *svc.ImportsRequest,
) error {
	client := svc.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	imports, err := client.Imports(req)
	if err != nil {
		return err
	}
	switch format {
	case "table":
		renderImportsTable(w, imports)
	case "json":
		err := renderImportsJSON(w, imports)
		if err != nil {
			return err
		}
	case "csv":
		err := renderImportsCSV(w, imports)
		if err != nil {
			return err
		}
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
}

func newListImportsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var start string
	var end string
	var dataStart string
	var includeDeleted bool
	var dataEnd string
	importsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List imports for a device",
		Run: func(cmd *cobra.Command, args []string) {
			err := listImports(
				os.Stdout,
				*params.baseURL,
				*params.clientID,
				format,
				viper.GetString("bearer_token"),
				params.userAgent,
				&svc.ImportsRequest{
					DeviceID:       deviceID,
					Start:          start,
					End:            end,
					DataStart:      dataStart,
					DataEnd:        dataEnd,
					IncludeDeleted: includeDeleted,
				},
			)
			if err != nil {
				fmt.Printf("Failed to list imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	importsListCmd.InheritedFlags()
	importsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	importsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of import time range")
	importsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of import time range")
	importsListCmd.PersistentFlags().StringVarP(&dataStart, "data-start", "", "", "start of data time range")
	importsListCmd.PersistentFlags().StringVarP(&dataEnd, "data-end", "", "", "end of data time range")
	importsListCmd.PersistentFlags().BoolVarP(&includeDeleted, "include-deleted", "", false, "end of data time range")
	importsListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return importsListCmd
}
