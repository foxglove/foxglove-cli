package cmd

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
)

func renderList[RequestType console.Request, ResponseType console.Record](
	w io.Writer,
	req RequestType,
	fn func(RequestType) ([]ResponseType, error),
	format string,
) error {
	records, err := fn(req)
	if err != nil {
		return err
	}
	switch format {
	case "table":
		renderTable(w, records)
	case "json":
		err := renderJSON(w, records)
		if err != nil {
			return fmt.Errorf("failed to render JSON: %w", err)
		}
	case "csv":
		err := renderCSV(w, records)
		if err != nil {
			return fmt.Errorf("failed to render CSV: %w", err)
		}
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
}

func renderTable[RecordType console.Record](w io.Writer, records []RecordType) {
	table := tablewriter.NewWriter(w)
	if len(records) == 0 {
		return
	}
	headers := records[0].Headers()
	table.SetHeader(headers)
	table.SetBorders(tablewriter.Border{
		Left:   true,
		Top:    false,
		Right:  true,
		Bottom: false,
	})
	table.SetAlignment(tablewriter.ALIGN_LEFT)
	table.SetCenterSeparator("|")
	data := [][]string{}
	for _, record := range records {
		data = append(data, record.Fields())
	}
	table.AppendBulk(data)
	table.Render()
}

func renderJSON[RecordType console.Record](w io.Writer, records []RecordType) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "    ")
	return encoder.Encode(records)
}

func renderCSV[RecordType console.Record](w io.Writer, records []RecordType) error {
	writer := csv.NewWriter(w)
	err := writer.Write(records[0].Headers())
	if err != nil {
		return err
	}
	for _, record := range records {
		err := writer.Write(record.Fields())
		if err != nil {
			return err
		}
	}
	writer.Flush()
	return writer.Error()
}

func fatalf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format, args...)
	os.Exit(1)
}

// Define a `format` flag on a command for one of the formats above
func AddFormatFlag(cmd *cobra.Command, format *string) {
	cmd.PersistentFlags().StringVarP(
		format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
}

func promptForInput(prompt string) string {
	fmt.Printf(prompt)
	var value string
	fmt.Scanln(&value)
	return value
}
