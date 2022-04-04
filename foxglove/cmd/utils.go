package cmd

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/olekukonko/tablewriter"
)

func renderList[RequestType svc.Request, ResponseType svc.Record](
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
		renderJSON(w, records)
	case "csv":
		renderCSV(w, records)
	default:
		return fmt.Errorf("unsupported format %s", format)
	}
	return nil
}

func renderTable[RecordType svc.Record](w io.Writer, records []RecordType) {
	table := tablewriter.NewWriter(w)
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

func renderJSON[RecordType svc.Record](w io.Writer, records []RecordType) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "    ")
	return encoder.Encode(records)
}

func renderCSV[RecordType svc.Record](w io.Writer, records []RecordType) error {
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
