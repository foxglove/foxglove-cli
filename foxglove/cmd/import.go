package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeImport(baseURL, clientID, deviceID, filename, token, userAgent string) error {
	ctx := context.Background()
	client := svc.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	err := svc.Import(ctx, client, deviceID, filename)
	if err != nil {
		return err
	}
	return nil
}

func newImportCommand(params *baseParams) (*cobra.Command, error) {
	var deviceID string
	var filename string
	importCmd := &cobra.Command{
		Use:   "import",
		Short: "Import a data file to the foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeImport(*params.baseURL, *params.clientID, deviceID, filename, viper.GetString("bearer_token"), params.userAgent)
			if err != nil {
				fmt.Printf("Import failed: %s\n", err)
			}
		},
	}
	importCmd.InheritedFlags()
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	importCmd.PersistentFlags().StringVarP(&filename, "filename", "", "", "filename")
	err := importCmd.MarkPersistentFlagRequired("device-id")
	if err != nil {
		return nil, err
	}
	err = importCmd.MarkPersistentFlagRequired("filename")
	if err != nil {
		return nil, err
	}
	return importCmd, nil
}
