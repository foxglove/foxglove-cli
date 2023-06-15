package cmd

import (
	"fmt"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
)

func newListDevicesCommand(params *baseParams) *cobra.Command {
	var format string
	deviceListCmd := &cobra.Command{
		Use:   "list",
		Short: "List devices registered to your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				console.DevicesRequest{},
				client.Devices,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list devices: %s\n", err)
			}
		},
	}
	deviceListCmd.InheritedFlags()
	AddFormatFlag(deviceListCmd, &format)
	return deviceListCmd
}

func newAddDeviceCommand(params *baseParams) *cobra.Command {
	var name string
	var serialNumber string
	addDeviceCmd := &cobra.Command{
		Use:   "add",
		Short: "Add a device for your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			if serialNumber != "" {
				fmt.Fprintf(os.Stderr, "Warning: serial-number is deprecated and will be removed in the next release\n")
			}

			resp, err := client.CreateDevice(console.CreateDeviceRequest{
				Name: name,
			})
			if err != nil {
				fatalf("Failed to create device: %s\n", err)
			}
			fmt.Fprintf(os.Stderr, "Device created: %s\n", resp.ID)
		},
	}
	addDeviceCmd.InheritedFlags()
	addDeviceCmd.PersistentFlags().StringVarP(&name, "name", "", "", "name of the device")
	addDeviceCmd.PersistentFlags().StringVarP(&serialNumber, "serial-number", "", "", "Deprecated. Value will be ignored.")
	return addDeviceCmd
}
