package cmd

import (
	"fmt"
	"os"
	"strconv"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	tw "github.com/foxglove/foxglove-cli/foxglove/util/tablewriter"

	"github.com/spf13/cobra"
)

func executeInfo(baseURL, clientID, token, userAgent string) error {
	isUsingApiKey := TokenIsApiKey(token)
	if isUsingApiKey {
		fmt.Println("Authenticated with API key")
		return nil
	}

	client := api.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	me, err := client.Me()
	if err != nil {
		return err
	}

	headers := []string{
		"Email",
		"Email verified",
		"Org ID",
		"Org Slug",
		"Admin",
	}
	data := [][]string{{
		me.Email,
		strconv.FormatBool(me.EmailVerified),
		me.OrgId,
		me.OrgSlug,
		strconv.FormatBool(me.Admin),
	}}

	fmt.Println("Authenticated with session token")
	tw.PrintTable(os.Stdout, headers, data)

	return nil
}

func newInfoCommand(params *baseParams) *cobra.Command {
	loginCmd := &cobra.Command{
		Use:   "info",
		Short: "Display information about the currently authenticated user",
		Run: func(cmd *cobra.Command, args []string) {
			if !IsAuthenticated() {
				dief("Not signed in. Run `foxglove auth login` or `foxglove auth configure-api-key` to continue.")
			}
			err := executeInfo(params.baseURL, *params.clientID, params.token, params.userAgent)
			if err != nil {
				dief("Info command failed: %s", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	return loginCmd
}
