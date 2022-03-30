package cmd

import (
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
		"ImportTime",
		"Start",
		"End",
		"InputType",
		"OutputType",
		"InputSize",
		"TotalOutputSize",
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
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
}

func newListImportsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
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
					DeviceID: deviceID,
				},
			)
			if err != nil {
				fmt.Printf("Failed to list imports: %s\n", err)
			}
		},
	}
	importsListCmd.InheritedFlags()
	importsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	importsListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return importsListCmd
}
