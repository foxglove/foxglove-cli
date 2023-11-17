package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/console"

	"github.com/spf13/cobra"
)

func executeLogin(baseURL, clientID, userAgent string, authDelegate console.AuthDelegate) error {
	ctx := context.Background()
	client := console.NewRemoteFoxgloveClient(baseURL, clientID, "", userAgent)
	bearerToken, err := console.Login(ctx, client, authDelegate)
	if err != nil {
		return err
	}
	err = configureAuth(bearerToken, baseURL, TokenSession)
	if err != nil {
		return fmt.Errorf("Failed to configure auth: %w", err)
	}
	return nil
}

func newLoginCommand(params *baseParams) *cobra.Command {
	var baseURL string
	loginCmd := &cobra.Command{
		Use:   "login",
		Short: "Log in to Foxglove Data Platform",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeLogin(baseURL, *params.clientID, params.userAgent, &console.PlatformAuthDelegate{})
			if err != nil {
				dief("Login failed: %s", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	loginCmd.PersistentFlags().StringVarP(&baseURL, "base-url", "", defaultBaseURL, "console API server")
	return loginCmd
}
