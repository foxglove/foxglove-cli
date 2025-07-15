package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/spf13/cobra"
)

func newListDevicesCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	var projectID string
	deviceListCmd := &cobra.Command{
		Use:   "list",
		Short: "List devices registered to your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				api.DevicesRequest{
					ProjectID: projectID,
				},
				client.Devices,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list devices: %s\n", err)
			}
		},
	}
	deviceListCmd.InheritedFlags()
	deviceListCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", "", "project ID")
	AddFormatFlag(deviceListCmd, &format)
	AddJsonFlag(deviceListCmd, &isJsonFormat)
	return deviceListCmd
}

func newAddDeviceCommand(params *baseParams) *cobra.Command {
	var name string
	var projectID string
	var serialNumber string
	var propertyPairs []string
	addDeviceCmd := &cobra.Command{
		Use:   "add",
		Short: "Add a device for your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			if serialNumber != "" {
				fmt.Fprintf(os.Stderr, "Warning: serial-number is deprecated and will be removed in the next release\n")
			}

			properties, err := util.DeviceProperties(propertyPairs, client)
			if err != nil {
				dief("Failed to create device: %s", err)
			}

			resp, err := client.CreateDevice(api.CreateDeviceRequest{
				Name:       name,
				ProjectID:  projectID,
				Properties: properties,
			})
			if err != nil {
				dief("Failed to create device: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Device created: %s\n", resp.ID)
		},
	}
	addDeviceCmd.InheritedFlags()
	addDeviceCmd.PersistentFlags().StringVarP(&name, "name", "", "", "name of the device")
	addDeviceCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", "", "project ID")
	addDeviceCmd.PersistentFlags().StringVarP(&serialNumber, "serial-number", "", "", "Deprecated. Value will be ignored.")
	addDeviceCmd.PersistentFlags().StringArrayVarP(&propertyPairs, "property", "p", []string{}, "Custom property colon-separated key value pair. Multiple may be specified.")
	return addDeviceCmd
}

func newEditDeviceCommand(params *baseParams) *cobra.Command {
	var name string
	var propertyPairs []string
	addDeviceCmd := &cobra.Command{
		Use:   "edit",
		Short: "Edit a device",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			properties, err := util.DeviceProperties(propertyPairs, client)
			if err != nil {
				dief("Failed to edit device: %s", err)
			}

			nameOrId := args[0]
			reqBody := api.CreateDeviceRequest{}
			if properties == nil && name == "" {
				dief("Nothing to update")
			}

			if name != "" {
				reqBody.Name = name
			}
			if properties != nil {
				reqBody.Properties = properties
			}

			resp, err := client.EditDevice(nameOrId, api.CreateDeviceRequest{
				Name:       name,
				Properties: properties,
			})
			if err != nil {
				dief("Failed to edit device: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Device updated: %s\n", resp.Name)
		},
	}
	addDeviceCmd.InheritedFlags()
	addDeviceCmd.PersistentFlags().StringVarP(&name, "name", "", "", "New name for the device")
	addDeviceCmd.PersistentFlags().StringArrayVarP(&propertyPairs, "property", "p", []string{}, "Custom property colon-separated key value pair. Multiple may be specified.")
	return addDeviceCmd
}
