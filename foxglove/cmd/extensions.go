package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeExtensionUpload(baseURL, clientID, token, filename, userAgent string) (resp *console.ExtensionUploadResponse, err error) {
	ctx := context.Background()
	client := console.NewRemoteFoxgloveClient(
		baseURL,
		clientID,
		token,
		userAgent,
	)
	return console.UploadExtensionFile(ctx, client, filename)
}

func newUploadExtensionCommand(params *baseParams) *cobra.Command {
	uploadCmd := &cobra.Command{
		Use:   "upload [FILE]",
		Short: "Upload a Studio extension (.foxe) to your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			resp, err := executeExtensionUpload(
				*params.baseURL,
				*params.clientID,
				viper.GetString("bearer_token"),
				filename,
				params.userAgent,
			)
			if err != nil {
				fatalf("Extension upload failed: %s\n", err)
			}
			fmt.Printf("Extension uploaded: %s\n", *resp)
		},
	}
	uploadCmd.InheritedFlags()
	return uploadCmd
}
