package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeImport(baseURL, clientID, projectID, deviceID, deviceName, key, filename, token, userAgent string) error {
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
	client := api.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	err = api.Import(ctx, client, projectID, deviceID, deviceName, key, filename)
	if err != nil {
		return err
	}

	return nil
}

func importFromEdge(baseURL, clientID, token, userAgent, edgeRecordingID string) error {
	client := api.NewRemoteFoxgloveClient(
		baseURL, clientID,
		token,
		userAgent,
	)
	_, err := client.ImportFromEdge(api.ImportFromEdgeRequest{}, edgeRecordingID)
	if err != nil {
		return err
	}

	return nil
}

func newImportCommand(params *baseParams, commandName string) (*cobra.Command, error) {
	var projectID string
	var deviceID string
	var deviceName string
	var edgeRecordingID string
	var key string
	importCmd := &cobra.Command{
		Use:   fmt.Sprintf("%s [FILE]", commandName),
		Short: "Import a data file to Foxglove Data Platform",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			if edgeRecordingID != "" {
				err := importFromEdge(params.baseURL, *params.clientID, params.token, params.userAgent, edgeRecordingID)
				if err != nil {
					dief("Failed to import edge recording: %s", err)
				}
				return
			}

			filename := args[0]
			err := executeImport(
				params.baseURL,
				*params.clientID,
				projectID,
				deviceID,
				deviceName,
				key,
				filename,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			if err != nil {
				dief("Failed to import %s: %s", filename, err)
			}
		},
	}
	importCmd.InheritedFlags()
	importCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "project ID")
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	importCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "device name")
	importCmd.PersistentFlags().StringVarP(&key, "key", "", "", "recording key")
	importCmd.PersistentFlags().StringVarP(&edgeRecordingID, "edge-recording-id", "", "", "edge recording ID")
	AddDeviceAutocompletion(importCmd, params)
	return importCmd, nil
}
