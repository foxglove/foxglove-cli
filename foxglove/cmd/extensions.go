package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
)

func executeExtensionUpload(client *api.FoxgloveClient, filename string) error {
	ctx := context.Background()
	return api.UploadExtensionFile(ctx, client, filename)
}

func executeExtensionDelete(client *api.FoxgloveClient, extensionId string) error {
	return client.DeleteExtension(extensionId)
}

func newPublishExtensionCommand(params *baseParams) *cobra.Command {
	uploadCmd := &cobra.Command{
		Use:   "publish [FILE]",
		Short: "Publish a Studio extension (.foxe) to your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			filename := args[0] // guaranteed length 1 due to Args setting above
			client := api.NewRemoteFoxgloveClient(
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
				dief("Extension upload failed: %s", err)
			}
			fmt.Fprintln(os.Stderr, "Extension published")
		},
	}
	uploadCmd.InheritedFlags()
	return uploadCmd
}

func newListExtensionsCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	listCmd := &cobra.Command{
		Use:   "list",
		Short: "List Studio extensions created for your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				api.ExtensionsRequest{},
				client.Extensions,
				format,
			)

			if err != nil {
				dief("Failed to list extensions: %s", err)
			}
		},
	}
	listCmd.InheritedFlags()
	AddFormatFlag(listCmd, &format)
	AddJsonFlag(listCmd, &isJsonFormat)
	return listCmd
}

func newUnpublishExtensionCommand(params *baseParams) *cobra.Command {
	deleteCmd := &cobra.Command{
		Use:   "unpublish [ID]",
		Short: "Delete and unpublish a Studio extension from your organization",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			err := executeExtensionDelete(client, args[0])
			if err != nil {
				dief("Failed to delete extension: %s", err)
			}
			fmt.Fprintln(os.Stderr, "Extension deleted")
		},
	}
	deleteCmd.InheritedFlags()
	return deleteCmd
}
