package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeImport(baseURL, clientID, projectID, deviceID, deviceName, key, sessionID, sessionKey, filename, token, userAgent string) error {
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
	err = api.Import(ctx, client, projectID, deviceID, deviceName, key, sessionID, sessionKey, filename)
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

func newImportCommand(params *baseParams, commandName string, deprecated *string) (*cobra.Command, error) {
	var projectID string
	var deviceID string
	var deviceName string
	var edgeRecordingID string
	var key string
	var sessionID string
	var sessionKey string
	var deprecatedMsg string
	if deprecated != nil {
		deprecatedMsg = *deprecated
	}
	importCmd := &cobra.Command{
		Use:        fmt.Sprintf("%s [FILE]", commandName),
		Short:      "Import a data file to Foxglove Data Platform",
		Args:       cobra.ExactArgs(1),
		Deprecated: deprecatedMsg,
		Run: func(cmd *cobra.Command, args []string) {
			if edgeRecordingID != "" {
				err := importFromEdge(params.baseURL, *params.clientID, params.token, params.userAgent, edgeRecordingID)
				if err != nil {
					dief("Failed to import edge recording: %s", err)
				}
				return
			}

			if err := validateSessionKeyRequiresProjectID(sessionKey, projectID); err != nil {
				dief("%s", err)
			}

			filename := args[0]
			err := executeImport(
				params.baseURL,
				*params.clientID,
				projectID,
				deviceID,
				deviceName,
				key,
				sessionID,
				sessionKey,
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
	importCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when using --session-key)")
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	importCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	importCmd.PersistentFlags().StringVarP(&key, "key", "", "", "Recording key")
	importCmd.PersistentFlags().StringVarP(&sessionID, "session-id", "", "", "Session ID")
	importCmd.PersistentFlags().StringVarP(&sessionKey, "session-key", "", "", "Session key (requires --project-id)")
	importCmd.PersistentFlags().StringVarP(&edgeRecordingID, "edge-recording-id", "", "", "Edge recording ID")
	AddDeviceAutocompletion(importCmd, params)
	return importCmd, nil
}
