package cmd

import (
	"context"
	"errors"
	"fmt"
	"os/exec"
	"runtime"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/svc"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func openBrowser(url string) (*exec.Cmd, error) {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "linux":
		cmd = exec.Command("xdg-open", url)
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", url)
	case "darwin":
		cmd = exec.Command("open", url)
	default:
		return nil, fmt.Errorf("unsupported platform")
	}
	return cmd, cmd.Start()
}

// login initializes a browser-based login flow for foxglove studio.
func login(ctx context.Context, client svc.FoxgloveClient, baseurl string, clientID string) error {
	info, err := client.DeviceCode()
	if err != nil {
		return fmt.Errorf("failed to fetch device code: %w", err)
	}
	fmt.Println("Enter this code in your browser: ", info.UserCode)
	browser, err := openBrowser(info.VerificationUriComplete)
	if err != nil {
		return fmt.Errorf("failed to open browser: %w", err)
	}
	defer browser.Process.Kill()

	// now poll the token endpoint until the token for the device code appears.
	// When the device code has not yet appeared, the endpoint returns a 403.
	var token string
	for {
		token, err = client.Token(info.DeviceCode)
		if errors.Is(err, svc.ErrForbidden) {
			time.Sleep(500 * time.Millisecond)
			continue
		}
		if err != nil {
			return fmt.Errorf("failed to request token: %w", err)
		}
		break
	}
	bearerToken, err := client.SignIn(token)
	if err != nil {
		return fmt.Errorf("failed to sign in: %w", err)
	}
	viper.Set("id_token", bearerToken)
	err = viper.WriteConfig()
	if err != nil {
		return fmt.Errorf("failed to write config: %w", err)
	}
	return nil
}

func newLoginCommand(baseURL, clientID string) *cobra.Command {
	loginCmd := &cobra.Command{
		Use:   "login",
		Short: "Log in to the foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			ctx := context.Background()
			client := svc.NewRemoteFoxgloveClient(baseURL, clientID, viper.GetString("id_token"))
			err := login(ctx, client, baseURL, clientID)
			time.Sleep(1 * time.Second)
			if err != nil {
				fmt.Printf("Login failure: %s\n", err)
			}
		},
	}
	return loginCmd
}
