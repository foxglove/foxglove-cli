package cmd

import (
	"bytes"
	"encoding/csv"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/foxglove/mcap/go/mcap"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
)

var ErrTruncatedMCAP = errors.New("truncated mcap file")
var ErrInvalidInput = errors.New("magic bytes do not match bag or mcap format")

func fileLooksLikeMCAP(r io.ReadSeeker) (bool, error) {
	buf := make([]byte, len(mcap.Magic))
	_, err := r.Read(buf)
	if err != nil {
		return false, fmt.Errorf("failed to read magic bytes: %w", err)
	}
	if !bytes.Equal(buf, mcap.Magic) {
		return false, nil
	}
	// if the header bytes equal mcap magic, check the footer bytes
	_, err = r.Seek(-int64(len(mcap.Magic)), io.SeekEnd)
	if err != nil {
		return false, fmt.Errorf("failed to seek to file end: %w", err)
	}
	_, err = r.Read(buf)
	if err != nil {
		return false, fmt.Errorf("failed to read trailing mcap magic bytes: %w", err)
	}
	if !bytes.Equal(buf, mcap.Magic) {
		return false, ErrTruncatedMCAP
	}
	return true, nil
}

func fileLooksLikeBag(r io.ReadSeeker) (bool, error) {
	bagMagic := []byte("#ROSBAG V2.0\n")
	buf := make([]byte, len(bagMagic))
	_, err := r.Read(buf)
	if err != nil {
		return false, fmt.Errorf("failed to read magic bytes: %w", err)
	}
	if !bytes.Equal(buf, bagMagic) {
		return false, nil
	}
	return true, nil
}

func validateImportLooksLegal(r io.ReadSeeker) error {
	looksLikeMCAP, err := fileLooksLikeMCAP(r)
	if err != nil {
		return err
	}
	if looksLikeMCAP {
		return nil
	}
	_, err = r.Seek(0, io.SeekStart)
	if err != nil {
		return err
	}
	looksLikeBag, err := fileLooksLikeBag(r)
	if err != nil {
		return err
	}
	if looksLikeBag {
		return nil
	}
	return ErrInvalidInput
}

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
	fmt.Print(prompt)
	var value string
	fmt.Scanln(&value)
	return value
}
