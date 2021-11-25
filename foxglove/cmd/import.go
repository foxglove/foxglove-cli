package cmd

import (
	"context"
	"fmt"
	"os"
	"path"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/schollz/progressbar/v3"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func doImport(
	ctx context.Context,
	client svc.FoxgloveClient,
	deviceID string,
	filename string,
) error {
	f, err := os.Open(filename)
	if err != nil {
		return fmt.Errorf("failed to open input file: %w", err)
	}
	defer f.Close()
	stat, err := f.Stat()
	if err != nil {
		return fmt.Errorf("failed to stat input: %w", err)
	}
	_, name := path.Split(filename)
	bar := progressbar.DefaultBytes(stat.Size(), "uploading")
	defer bar.Close()
	reader := progressbar.NewReader(f, bar)
	err = client.Upload(&reader, svc.UploadRequest{
		Filename: name,
		DeviceID: deviceID,
	})
	if err != nil {
		return fmt.Errorf("upload failure: %w", err)
	}
	return nil
}

func newImportCommand(baseURL, clientID string) *cobra.Command {
	var deviceID string
	var filename string
	importCmd := &cobra.Command{
		Use:   "import",
		Short: "Import a data file to the foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			ctx := context.Background()
			client := svc.NewRemoteFoxgloveClient(
				baseURL,
				clientID,
				viper.GetString("id_token"),
			)
			err := doImport(ctx, client, deviceID, filename)
			if err != nil {
				fmt.Printf("Import failed: %s\n", err)
			}
		},
	}
	importCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	importCmd.PersistentFlags().StringVarP(&filename, "filename", "", "", "filename")
	return importCmd
}
