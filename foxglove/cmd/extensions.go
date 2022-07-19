package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeExtensionUpload(baseURL, clientID, token, filename, userAgent string) error {
	ctx := context.Background()
	client := console.NewRemoteFoxgloveClient(
		baseURL,
		clientID,
		token,
		userAgent,
	)
	return console.UploadExtensionFile(ctx, client, filename)
}

func newPublishExtensionCommand(params *baseParams) *cobra.Command {
	uploadCmd := &cobra.Command{
		Use:   "publish [FILE]",
		Short: "Publish a Studio extension (.foxe) to your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			err := executeExtensionUpload(
				*params.baseURL,
				*params.clientID,
				viper.GetString("bearer_token"),
				filename,
				params.userAgent,
			)
			if err != nil {
				fatalf("Extension upload failed: %s\n", err)
			}
			fmt.Println("Extension published")
		},
	}
	uploadCmd.InheritedFlags()
	return uploadCmd
}
