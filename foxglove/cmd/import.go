package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeImport(baseURL, clientID, deviceID, deviceName, filename, token, userAgent string) error {
	ctx := context.Background()
	f, err := os.Open(filename)
	if err != nil {
		return err
	}
	defer f.Close()
	err = validateImportLooksLegal(f)
	if err != nil {
		return err
	}
	client := console.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	err = console.Import(ctx, client, deviceID, deviceName, filename)
	if err != nil {
		return err
	}

	return nil
}

func importFromEdge(baseURL, clientID, token, userAgent, edgeRecordingID string) error {
	client := console.NewRemoteFoxgloveClient(
		baseURL, clientID,
		token,
		userAgent,
	)
	_, err := client.ImportFromEdge(console.ImportFromEdgeRequest{}, edgeRecordingID)
	if err != nil {
		return err
	}

	return nil
}

func newImportCommand(params *baseParams, commandName string) (*cobra.Command, error) {
	var deviceID string
	var deviceName string
	var edgeRecordingID string
	importCmd := &cobra.Command{
		Use:   fmt.Sprintf("%s [FILE]", commandName),
		Short: "Import a data file to Foxglove Data Platform",
		Run: func(cmd *cobra.Command, args []string) {
			if deviceName == "" && deviceID == "" && edgeRecordingID == "" {
				dief("Must specify either --device-id, --device-name, or --edge-recording-id")
			}
			if deviceName != "" || deviceID != "" {
				if len(args) == 0 {
					dief("Filename not specified")
				}
				filename := args[0]
				err := executeImport(
					params.baseURL,
					*params.clientID,
					deviceID,
					deviceName,
					filename,
					viper.GetString("bearer_token"),
					params.userAgent,
				)
				if err != nil {
					dief("Failed to import %s: %s\n", filename, err)
				}
			}
			if edgeRecordingID != "" {
				err := importFromEdge(params.baseURL, *params.clientID, params.token, params.userAgent, edgeRecordingID)
				if err != nil {
					dief("Failed to import edge recording: %s\n", err)
				}
			}
		},
	}
	importCmd.InheritedFlags()
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	importCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "device name")
	importCmd.PersistentFlags().StringVarP(&edgeRecordingID, "edge-recording-id", "", "", "edge recording ID")
	AddDeviceAutocompletion(importCmd, params)
	return importCmd, nil
}
