package cmd

import (
	"fmt"
	"os"
	"path"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"

	"github.com/spf13/viper"
)

const (
	foxgloveClientID = "d51173be08ed4cf7a734aed9ac30afd0"
	appname          = "foxglove-cli"
	defaultBaseURL   = "https://api.foxglove.dev"
)

func configfile() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return path.Join(home, ".foxgloverc"), nil
}

func configureAuth(token, baseURL string) error {
	viper.Set("bearer_token", token)
	viper.Set("base_url", baseURL)
	err := viper.WriteConfigAs(viper.ConfigFileUsed())
	if err != nil {
		return fmt.Errorf("Failed to write config: %w", err)
	}
	return nil
}

var logDebug bool

func debugMode() bool {
	return logDebug
}

func dief(s string, args ...any) {
	fmt.Fprintf(os.Stderr, s+"\n", args...)
	os.Exit(1)
}

type baseParams struct {
	clientID  *string
	cfgFile   *string
	baseURL   string
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
		client := console.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
		devices, err := client.Devices(console.DevicesRequest{})
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

func listDevicesByNameAutocompletionFunc(
	baseURL string,
	clientID string,
	token string,
	userAgent string,
) func(*cobra.Command, []string, string) ([]string, cobra.ShellCompDirective) {
	return func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		client := console.NewRemoteFoxgloveClient(baseURL, clientID, token, userAgent)
		devices, err := client.Devices(console.DevicesRequest{})
		if err != nil {
			return []string{}, cobra.ShellCompDirectiveDefault
		}
		var candidates []string
		for _, device := range devices {
			candidates = append(candidates, device.Name)
		}
		return candidates, cobra.ShellCompDirectiveDefault
	}
}

func defaultString(a, b string) string {
	if a != "" {
		return a
	}
	return b
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
	attachmentsCmd := &cobra.Command{
		Use:   "attachments",
		Short: "Query and modify data attachments",
	}
	recordingsCmd := &cobra.Command{
		Use:   "recordings",
		Short: "Query recordings",
	}
	devicesCmd := &cobra.Command{
		Use:   "devices",
		Short: "List and manage devices",
	}
	eventsCmd := &cobra.Command{
		Use:   "events",
		Short: "List and manage events",
	}
	coverageCmd := &cobra.Command{
		Use:   "coverage",
		Short: "List coverage ranges",
	}
	extensionsCmd := &cobra.Command{
		Use:   "extensions",
		Short: "List and publish Studio extensions",
	}

	var clientID, cfgFile string
	rootCmd.PersistentFlags().StringVarP(&cfgFile, "config", "", "", "config file (default is $HOME/.foxglove.yaml)")
	rootCmd.PersistentFlags().StringVarP(&clientID, "client-id", "", foxgloveClientID, "foxglove client ID")
	rootCmd.PersistentFlags().BoolVarP(&logDebug, "debug", "", false, "enable debug logging")

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

	useragent := fmt.Sprintf("%s/%s", appname, version)
	params := &baseParams{
		userAgent: useragent,
		cfgFile:   &cfgFile,
		clientID:  &clientID,
		token:     viper.GetString("bearer_token"),
		baseURL:   defaultString(viper.GetString("base_url"), defaultBaseURL),
	}

	addImportCmd, err := newImportCommand(params, "add")
	if err != nil {
		fmt.Println(err)
		return
	}
	importShortcut, err := newImportCommand(params, "import")
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
	configureAPIKey := newConfigureAPIKeyCommand()
	authCmd.AddCommand(loginCmd)
	authCmd.AddCommand(configureAPIKey)
	recordingsCmd.AddCommand(newListRecordingsCommand(params))
	importsCmd.AddCommand(newListImportsCommand(params), addImportCmd)
	attachmentsCmd.AddCommand(newListAttachmentsCommand(params))
	attachmentsCmd.AddCommand(newDownloadAttachmentCmd(params))
	coverageCmd.AddCommand(newListCoverageCommand(params))
	dataCmd.AddCommand(
		exportCmd,
		importsCmd,
		coverageCmd,
		importShortcut,
	)
	devicesCmd.AddCommand(newListDevicesCommand(params), newAddDeviceCommand(params), newEditDeviceCommand(params))
	eventsCmd.AddCommand(newListEventsCommand(params), newAddEventCommand(params))
	extensionsCmd.AddCommand(newListExtensionsCommand(params))
	extensionsCmd.AddCommand(newPublishExtensionCommand(params))
	extensionsCmd.AddCommand(newUnpublishExtensionCommand(params))

	rootCmd.AddCommand(
		authCmd,
		dataCmd,
		newVersionCommand(version),
		devicesCmd,
		extensionsCmd,
		attachmentsCmd,
		recordingsCmd,
		eventsCmd,
	)

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
