package cmd

import (
	"fmt"
	"os"
	"text/tabwriter"

	"github.com/foxglove/foxglove-cli/foxglove/console"

	"github.com/spf13/cobra"
)

func executeInfo(baseURL, clientID, token, userAgent string) error {
	client := console.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
	me, err := client.Me()
	if err != nil {
		return err
	}
	w := tabwriter.NewWriter(os.Stdout, 0, 1, 1, ' ', 0)
	fmt.Fprintln(w, "Email\t", me.Email)
	fmt.Fprintln(w, "Email verified\t", me.EmailVerified)
	fmt.Fprintln(w, "Org ID\t", me.OrgId)
	fmt.Fprintln(w, "Org Slug\t", me.OrgSlug)
	fmt.Fprintln(w, "Admin\t", me.Admin)
	fmt.Fprintln(w, "Token type\t", "session")
	w.Flush()

	return nil
}

func newInfoCommand(params *baseParams) *cobra.Command {
	loginCmd := &cobra.Command{
		Use:   "info",
		Short: "Display information about the currently logged-in user",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeInfo(params.baseURL, *params.clientID, params.token, params.userAgent)
			if err != nil {
				dief("Info command failed: %s", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	return loginCmd
}
