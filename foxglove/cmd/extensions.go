package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
)

func executeExtensionUpload(client *console.FoxgloveClient, filename string) error {
	ctx := context.Background()
	return console.UploadExtensionFile(ctx, client, filename)
}

func executeExtensionDelete(client *console.FoxgloveClient, extensionId string) error {
	return client.DeleteExtension(extensionId)
}

func newPublishExtensionCommand(params *baseParams) *cobra.Command {
	uploadCmd := &cobra.Command{
		Use:   "publish [FILE]",
		Short: "Publish a Studio extension (.foxe) to your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			client := console.NewRemoteFoxgloveClient(
				params.baseURL,
				*params.clientID,
				params.token,
				params.userAgent,
			)
			err := executeExtensionUpload(
				client,
				filename,
			)
			if err != nil {
				dief("Extension upload failed: %s\n", err)
			}
			fmt.Fprintln(os.Stderr, "Extension published")
		},
	}
	uploadCmd.InheritedFlags()
	return uploadCmd
}

func newListExtensionsCommand(params *baseParams) *cobra.Command {
	var format string
	listCmd := &cobra.Command{
		Use:   "list",
		Short: "List Studio extensions created for your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				console.ExtensionsRequest{},
				client.Extensions,
				format,
			)

			if err != nil {
				dief("Failed to list extensions: %s\n", err)
			}
		},
	}
	listCmd.InheritedFlags()
	AddFormatFlag(listCmd, &format)
	return listCmd
}

func newUnpublishExtensionCommand(params *baseParams) *cobra.Command {
	deleteCmd := &cobra.Command{
		Use:   "unpublish [ID]",
		Short: "Delete and unpublish a Studio extension from your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			err := executeExtensionDelete(client, args[0])
			if err != nil {
				dief("Failed to delete extension: %s\n", err)
			}
			fmt.Fprintln(os.Stderr, "Extension deleted")
		},
	}
	deleteCmd.InheritedFlags()
	return deleteCmd
}
