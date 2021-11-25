package cmd

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

var (
	ErrRedirectStdout = errors.New("stdout unredirected")
)

type ExportRequest struct {
	DeviceID     string   `json:"deviceId"`
	Start        string   `json:"start"`
	End          string   `json:"end"`
	OutputFormat string   `json:"outputFormat"`
	Topics       []string `json:"topics"`
}

type ExportResponse struct {
	Link string `json:"link"`
}

func stdoutRedirected() bool {
	if fi, _ := os.Stdout.Stat(); (fi.Mode() & os.ModeCharDevice) != 0 {
		return false
	}
	return true
}

func export(
	ctx context.Context,
	client svc.FoxgloveClient,
	deviceID string,
	startstr string,
	endstr string,
	topics []string,
	outputFormat string,
) error {
	if !stdoutRedirected() {
		return ErrRedirectStdout
	}
	start, err := time.Parse(time.RFC3339, startstr)
	if err != nil {
		return fmt.Errorf("failed to parse start: %w", err)
	}
	end, err := time.Parse(time.RFC3339, endstr)
	if err != nil {
		return fmt.Errorf("failed to parse start: %w", err)
	}
	rc, err := client.Stream(svc.StreamRequest{
		DeviceID:     deviceID,
		Start:        start,
		End:          end,
		OutputFormat: outputFormat,
		Topics:       topics,
	})
	if err != nil {
		return fmt.Errorf("streaming request failure: %w", err)
	}
	defer rc.Close()
	_, err = io.Copy(os.Stdout, rc)
	if err != nil {
		return fmt.Errorf("copy failure: %w", err)
	}
	return nil
}

func newExportCommand(baseURL, clientID string) *cobra.Command {
	var deviceID string
	var start string
	var end string
	var outputFormat string
	var topicList string
	exportCmd := &cobra.Command{
		Use:   "export",
		Short: "Export a data selection from foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			ctx := context.Background()
			client := svc.NewRemoteFoxgloveClient(
				baseURL,
				clientID,
				viper.GetString("id_token"),
			)
			topics := strings.FieldsFunc(topicList, func(c rune) bool {
				return c == ','
			})
			err := export(ctx, client, deviceID, start, end, topics, outputFormat)
			if err != nil {
				if errors.Is(err, ErrRedirectStdout) {
					fmt.Printf("Binary output may screw up your terminal. Please redirect to a pipe or file.\n")
					return
				}
				fmt.Printf("Export failed: %s", err)
			}
		},
	}
	exportCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	exportCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start")
	exportCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end")
	exportCmd.PersistentFlags().StringVarP(&outputFormat, "output-format", "", "", "output format")
	exportCmd.PersistentFlags().StringVarP(&topicList, "topics", "", "", "comma separated list of topics")

	return exportCmd
}
