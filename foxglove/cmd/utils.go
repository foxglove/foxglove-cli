package cmd

import (
	"bytes"
	"encoding/csv"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	tw "github.com/foxglove/foxglove-cli/foxglove/util/tablewriter"
	"github.com/foxglove/mcap/go/mcap"
	"github.com/relvacode/iso8601"
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
		if len(records) == 0 {
			fmt.Println("No records found")
			return nil
		}
		data := [][]string{}
		for _, record := range records {
			data = append(data, record.Fields())
		}
		tw.PrintTable(w, records[0].Headers(), data)
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

func debugf(format string, args ...any) {
	if debugMode() {
		fmt.Fprintf(os.Stderr, "[DEBUG] "+format+"\n", args...)
	}
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

// AddDeviceAutocompletion adds autocompletion for device-name and device-id
// parameters to the command.
func AddDeviceAutocompletion(cmd *cobra.Command, params *baseParams) {
	if err := cmd.RegisterFlagCompletionFunc(
		"device-id",
		listDevicesAutocompletionFunc(
			params.baseURL,
			*params.clientID,
			params.token,
			params.userAgent,
		),
	); err != nil {
		dief("failed to register device-id autocompletion: %v", err)
	}
	if err := cmd.RegisterFlagCompletionFunc(
		"device-name",
		listDevicesByNameAutocompletionFunc(
			params.baseURL,
			*params.clientID,
			params.token,
			params.userAgent,
		),
	); err != nil {
		dief("failed to register device-name autocompletion: %v", err)
	}
}

func promptForInput(prompt string) string {
	fmt.Print(prompt)
	var value string
	fmt.Scanln(&value)
	return value
}

// maybeConvertToRFC3339 converts an ISO8601 timestamp to RFC3339, if an input
// timestamp is supplied. If the input is empty, it returns an empty string and
// no error.
func maybeConvertToRFC3339(timestamp string) (string, error) {
	if timestamp == "" {
		return "", nil
	}
	parsed, err := iso8601.ParseString(timestamp)
	if err != nil {
		return "", err
	}
	return parsed.Format(time.RFC3339), nil
}
