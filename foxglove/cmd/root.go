package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"

	"github.com/spf13/viper"
)

func Execute() {
	rootCmd := &cobra.Command{
		Use:   "foxglove",
		Short: "Command line client for the Foxglove data platform",
	}
	rootCmd.CompletionOptions.DisableDefaultCmd = true

	var cfgFile, baseURL, clientID string
	rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file (default is $HOME/.foxglove.yaml)")
	rootCmd.PersistentFlags().StringVarP(&clientID, "client-id", "", "oSJGEAQm16LNF09FSVTMYJO5aArQzq8o", "foxglove client ID")
	rootCmd.PersistentFlags().StringVarP(&baseURL, "baseurl", "", "https://api.foxglove.dev", "console API server")

	initConfig(cfgFile)

	rootCmd.AddCommand(newImportCommand(baseURL, clientID))
	rootCmd.AddCommand(newLoginCommand(baseURL, clientID))
	rootCmd.AddCommand(newExportCommand(baseURL, clientID))

	cobra.CheckErr(rootCmd.Execute())
}

// initConfig reads in config file and ENV variables if set.
func initConfig(cfgFile string) {
	if cfgFile != "" {
		// Use config file from the flag.
		viper.SetConfigFile(cfgFile)
	} else {
		// Find home directory.
		home, err := os.UserHomeDir()
		cobra.CheckErr(err)

		// Search config in home directory with name ".foxglove" (without extension).
		viper.AddConfigPath(home)
		viper.SetConfigType("yaml")
		viper.SetConfigName(".foxglove")
	}

	viper.AutomaticEnv() // read in environment variables that match

	// If a config file is found, read it in.
	if err := viper.ReadInConfig(); err == nil {
		fmt.Fprintln(os.Stderr, "Using config file:", viper.ConfigFileUsed())
	}
}
