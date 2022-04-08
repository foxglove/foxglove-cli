package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/console"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeLogin(baseURL, clientID, userAgent string) error {
	ctx := context.Background()
	client := console.NewRemoteFoxgloveClient(baseURL, clientID, "", userAgent)
	bearerToken, err := console.Login(ctx, client)
	if err != nil {
		return err
	}
	viper.Set("bearer_token", bearerToken)
	err = viper.WriteConfigAs(viper.ConfigFileUsed())
	if err != nil {
		return fmt.Errorf("Failed to write config: %s\n", err)
	}
	return nil
}

func newLoginCommand(params *baseParams) *cobra.Command {
	loginCmd := &cobra.Command{
		Use:   "login",
		Short: "Log in to the foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeLogin(*params.baseURL, *params.clientID, params.userAgent)
			if err != nil {
				fmt.Printf("Login failed: %s\n", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	return loginCmd
}
