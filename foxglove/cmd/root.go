package cmd

import (
	"fmt"
	"os"
	"path"

	"github.com/spf13/cobra"

	"github.com/spf13/viper"
)

const (
	foxgloveClientID = "d51173be08ed4cf7a734aed9ac30afd0"
)

func configfile() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return path.Join(home, ".foxgloverc"), nil
}

func Execute(version string) {
	if version == "" {
		version = "dev"
	}
	var rootCmd = &cobra.Command{
		Use:   "foxglove",
		Short: "Command line client for the Foxglove data platform",
	}
	var baseURL, clientID, cfgFile string
	rootCmd.CompletionOptions.DisableDefaultCmd = true
	rootCmd.PersistentFlags().StringVarP(&cfgFile, "config", "", "", "config file (default is $HOME/.foxglove.yaml)")
	rootCmd.PersistentFlags().StringVarP(&clientID, "client-id", "", foxgloveClientID, "foxglove client ID")
	rootCmd.PersistentFlags().StringVarP(&baseURL, "baseurl", "", "https://api.foxglove.dev", "console API server")

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
	importCmd, err := newImportCommand(&baseURL, &clientID)
	if err != nil {
		fmt.Println(err)
		return
	}
	exportCmd, err := newExportCommand(&baseURL, &clientID)
	if err != nil {
		fmt.Println(err)
		return
	}
	loginCmd := newLoginCommand(&baseURL, &clientID)
	rootCmd.AddCommand(importCmd)
	rootCmd.AddCommand(exportCmd)
	rootCmd.AddCommand(loginCmd)
	rootCmd.AddCommand(newVersionCommand(version))
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
