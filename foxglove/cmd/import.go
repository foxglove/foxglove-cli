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

func newImportCommand(params *baseParams, commandName string) (*cobra.Command, error) {
	var deviceID string
	var deviceName string
	importCmd := &cobra.Command{
		Use:   fmt.Sprintf("%s [FILE]", commandName),
		Short: "Import a data file to the foxglove data platform",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			if deviceName == "" && deviceID == "" {
				dief("Must specify either --device-id or --device-name")
			}
			err := executeImport(
				*params.baseURL,
				*params.clientID,
				deviceID,
				deviceName,
				filename,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			if err != nil {
				fatalf("Failed to import %s: %s\n", filename, err)
			}
		},
	}
	importCmd.InheritedFlags()
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	importCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "device name")
	err := importCmd.RegisterFlagCompletionFunc(
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
