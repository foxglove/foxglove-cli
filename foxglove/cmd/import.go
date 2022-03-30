package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeImport(baseURL, clientID, deviceID, filename, token, userAgent string) error {
	ctx := context.Background()
	client := console.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	err := console.Import(ctx, client, deviceID, filename)
	if err != nil {
		return err
	}
	return nil
}

func newImportCommand(params *baseParams, commandName string) (*cobra.Command, error) {
	var deviceID string
	importCmd := &cobra.Command{
		Use:   fmt.Sprintf("%s [FILE]", commandName),
		Short: "Import a data file to the foxglove data platform",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			err := executeImport(
				*params.baseURL,
				*params.clientID,
				deviceID,
				filename,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			if err != nil {
				fmt.Printf("Import failed: %s\n", err)
			}
		},
	}
	importCmd.InheritedFlags()
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	err := importCmd.MarkPersistentFlagRequired("device-id")
	if err != nil {
		return nil, err
	}
	err = importCmd.RegisterFlagCompletionFunc(
		"device-id",
		listDevicesAutocompletionFunc(
			*params.baseURL,
			*params.clientID,
			viper.GetString("bearer_token"),
			params.userAgent,
		),
	)
	if err != nil {
		return nil, err
	}
	return importCmd, nil
}
