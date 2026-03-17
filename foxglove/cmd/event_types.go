package cmd

import (
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
)

func newListEventTypesCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	cmd := &cobra.Command{
		Use:   "list",
		Short: "List event types",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				api.EventTypesRequest{},
				client.EventTypes,
				format,
			)
			if err != nil {
				dief("Failed to list event types: %s", err)
			}
		},
	}
	AddFormatFlag(cmd, &format)
	AddJsonFlag(cmd, &isJsonFormat)
	return cmd
}
