package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/api"

	"github.com/spf13/cobra"
)

func executeLogin(baseURL, clientID, userAgent string, authDelegate api.AuthDelegate) error {
	ctx := context.Background()
	client := api.NewRemoteFoxgloveClient(baseURL, clientID, "", userAgent)
	bearerToken, err := api.Login(ctx, client, authDelegate)
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
			err := executeLogin(baseURL, *params.clientID, params.userAgent, &api.PlatformAuthDelegate{})
			if err != nil {
				dief("Login failed: %s", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	loginCmd.PersistentFlags().StringVarP(&baseURL, "base-url", "", defaultBaseURL, "API server")
	return loginCmd
}
