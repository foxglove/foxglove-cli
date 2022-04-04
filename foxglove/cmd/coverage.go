package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListCoverageCommand(params *baseParams) *cobra.Command {
	var format string
	var start string
	var end string
	importsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List coverage ranges",
		Run: func(cmd *cobra.Command, args []string) {
			client := svc.NewRemoteFoxgloveClient(
				*params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&svc.CoverageRequest{
					Start: start,
					End:   end,
				},
				client.Coverage,
				format,
			)
			if err != nil {
				fmt.Printf("Failed to list imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	importsListCmd.InheritedFlags()
	importsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start of coverage time range")
	importsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end of coverage time range")
	importsListCmd.PersistentFlags().StringVarP(
		&format,
		"format",
		"",
		"table",
		"render output in specified format (table, json, csv)",
	)
	return importsListCmd
}
