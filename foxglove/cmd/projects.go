package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
)

func newListProjectsCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	projectListCmd := &cobra.Command{
		Use:   "list",
		Short: "List projects",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				api.ProjectsRequest{},
				client.Projects,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list projects: %s\n", err)
			}
		},
	}
	projectListCmd.InheritedFlags()
	AddFormatFlag(projectListCmd, &format)
	AddJsonFlag(projectListCmd, &isJsonFormat)
	return projectListCmd
}
