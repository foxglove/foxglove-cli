package cmd

import (
	"context"
	"fmt"

	"github.com/foxglove/foxglove-cli/foxglove/svc"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func executeLogin(baseURL, clientID string) error {
	ctx := context.Background()
	client := svc.NewRemoteFoxgloveClient(baseURL, clientID, viper.GetString("bearer_token"))
	bearerToken, err := svc.Login(ctx, client)
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

func newLoginCommand(baseURL, clientID *string) *cobra.Command {
	loginCmd := &cobra.Command{
		Use:   "login",
		Short: "Log in to the foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			err := executeLogin(*baseURL, *clientID)
			if err != nil {
				fmt.Printf("Login failed: %s\n", err)
			}
		},
	}
	loginCmd.InheritedFlags()
	return loginCmd
}
