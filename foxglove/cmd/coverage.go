package cmd

import (
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListCoverageCommand(params *baseParams) *cobra.Command {
	var format string
	var start string
	var end string
	coverageListCmd := &cobra.Command{
		Use:   "list",
		Short: "List coverage ranges",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&console.CoverageRequest{
					Start: start,
					End:   end,
				},
				client.Coverage,
				format,
			)
			if err != nil {
				fatalf("Failed to list coverage: %s\n", err)
			}
		},
	}
	coverageListCmd.InheritedFlags()
	coverageListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of coverage time range")
	coverageListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of coverage time range")
	coverageListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return coverageListCmd
}
