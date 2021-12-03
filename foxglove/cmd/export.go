package cmd

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

var (
	ErrRedirectStdout = errors.New("stdout unredirected")
)

func stdoutRedirected() bool {
	if fi, _ := os.Stdout.Stat(); (fi.Mode() & os.ModeCharDevice) == os.ModeCharDevice {
		return false
	}
	return true
}

func executeExport(w io.Writer, baseURL, clientID, deviceID, start, end, outputFormat, topicList, bearerToken, userAgent string) error {
	ctx := context.Background()
	client := svc.NewRemoteFoxgloveClient(
		baseURL,
		clientID,
		bearerToken,
		userAgent,
	)
	topics := strings.FieldsFunc(topicList, func(c rune) bool {
		return c == ','
	})
	if !stdoutRedirected() {
		return fmt.Errorf("Binary output may screw up your terminal. Please redirect to a pipe or file.\n")
	}
	err := svc.Export(ctx, w, client, deviceID, start, end, topics, outputFormat)
	if err != nil {
		return fmt.Errorf("Export failed: %s", err)
	}
	return nil
}

func newExportCommand(params *baseParams) (*cobra.Command, error) {
	var deviceID string
	var start string
	var end string
	var outputFormat string
	var topicList string
	exportCmd := &cobra.Command{
		Use:   "export",
		Short: "Export a data selection from foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeExport(
				os.Stdout,
				*params.baseURL,
				*params.clientID,
				deviceID,
				start,
				end,
				outputFormat,
				topicList,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			if err != nil {
				fmt.Printf("Export failed: %s\n", err)
			}
		},
	}
	exportCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	exportCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start time (RFC3339 timestamp)")
	exportCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end time (RFC3339 timestamp")
	exportCmd.PersistentFlags().StringVarP(&outputFormat, "output-format", "", "", "output format")
	exportCmd.PersistentFlags().StringVarP(&topicList, "topics", "", "", "comma separated list of topics")
	err := exportCmd.MarkPersistentFlagRequired("device-id")
	if err != nil {
		return nil, err
	}
	err = exportCmd.MarkPersistentFlagRequired("start")
	if err != nil {
		return nil, err
	}
	err = exportCmd.MarkPersistentFlagRequired("end")
	if err != nil {
		return nil, err
	}
	return exportCmd, nil
}
