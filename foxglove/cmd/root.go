package cmd

import (
	"fmt"
	"os"
	"path"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/spf13/cobra"

	"github.com/spf13/viper"
)

const (
	foxgloveClientID = "d51173be08ed4cf7a734aed9ac30afd0"
	appname          = "foxglove-cli"
)

func configfile() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return path.Join(home, ".foxgloverc"), nil
}

type baseParams struct {
	clientID  *string
	cfgFile   *string
	baseURL   *string
	userAgent string
	token     string
}

func listDevicesAutocompletionFunc(
	baseURL string,
	clientID string,
	token string,
	userAgent string,
) func(*cobra.Command, []string, string) ([]string, cobra.ShellCompDirective) {
	return func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		client := svc.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
		devices, err := client.Devices(svc.DevicesRequest{})
		if err != nil {
			return []string{}, cobra.ShellCompDirectiveDefault
		}
		var candidates []string
		for _, device := range devices {
			candidates = append(candidates, fmt.Sprintf("%s\t%s", device.ID, device.Name))
		}
		return candidates, cobra.ShellCompDirectiveDefault
	}
}

func Execute(version string) {
	if version == "" {
		version = "dev"
	}
	rootCmd := &cobra.Command{
		Use:   "foxglove",
		Short: "Command line client for the Foxglove data platform",
	}
	authCmd := &cobra.Command{
		Use:   "auth",
		Short: "Manage authentication",
	}
	dataCmd := &cobra.Command{
		Use:   "data",
		Short: "Data access and management",
	}
	importsCmd := &cobra.Command{
		Use:   "imports",
		Short: "Query and modify data imports",
	}
	devicesCmd := &cobra.Command{
		Use:   "devices",
		Short: "List and manage devices",
	}
	eventsCmd := &cobra.Command{
		Use:   "events",
		Short: "List and manage events",
	}

	var baseURL, clientID, cfgFile string
	rootCmd.PersistentFlags().StringVarP(&cfgFile, "config", "", "", "config file (default is $HOME/.foxglove.yaml)")
	rootCmd.PersistentFlags().StringVarP(&clientID, "client-id", "", foxgloveClientID, "foxglove client ID")
	rootCmd.PersistentFlags().StringVarP(&baseURL, "baseurl", "", "https://api.foxglove.dev", "console API server")

	useragent := fmt.Sprintf("%s/%s", appname, version)
	params := &baseParams{
		userAgent: useragent,
		cfgFile:   &cfgFile,
		baseURL:   &baseURL,
		clientID:  &clientID,
		token:     viper.GetString("bearer_token"),
	}

	var err error
	if cfgFile == "" {
		cfgFile, err = configfile()
		if err != nil {
			fmt.Println(err)
			return
		}
	}
	err = initConfig(&cfgFile)
	if err != nil {
		fmt.Println(err)
		return
	}
	importCmd, err := newImportCommand(params)
	if err != nil {
		fmt.Println(err)
		return
	}
	exportCmd, err := newExportCommand(params)
	if err != nil {
		fmt.Println(err)
		return
	}
	loginCmd := newLoginCommand(params)
	rootCmd.AddCommand(authCmd, dataCmd, newVersionCommand(version), devicesCmd, eventsCmd)
	authCmd.AddCommand(loginCmd)
	importsCmd.AddCommand(newListImportsCommand(params))
	dataCmd.AddCommand(importCmd, exportCmd, importsCmd)
	devicesCmd.AddCommand(newListDevicesCommand(params))
	eventsCmd.AddCommand(newListEventsCommand(params))
	cobra.CheckErr(rootCmd.Execute())
}

// initConfig reads in config file and ENV variables if set.
func initConfig(cfgFile *string) error {
	if *cfgFile != "" {
		// Use config file from the flag.
		viper.SetConfigType("yaml")
		viper.SetConfigFile(*cfgFile)
	} else {
		// Find home directory.
		home, err := os.UserHomeDir()
		cobra.CheckErr(err)

		// Search config in home directory with name ".foxglove" (without extension).
		viper.AddConfigPath(home)
		viper.SetConfigType("yaml")
		viper.SetConfigName(".foxgloverc")
	}

	viper.AutomaticEnv() // read in environment variables that match

	// If a config file is found, read it in.
	_ = viper.ReadInConfig()
	return nil
}
