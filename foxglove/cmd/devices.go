package cmd

import (
	"fmt"
	"os"
	"strings"

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
	var propertyPairs []string
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

			propertyVals := propertyMap(propertyPairs)
			resp, err := client.CreateDevice(console.CreateDeviceRequest{
				Name:       name,
				Properties: propertyVals,
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
	addDeviceCmd.PersistentFlags().StringArrayVarP(&propertyPairs, "property", "p", []string{}, "Custom property colon-separated key value pair. Multiple may be specified.")
	return addDeviceCmd
}

//	type PropertyValue interface {
//		 string | int64 | float64 | bool
//	}
//
// todo: need to consider value types and return generic
// https://stackoverflow.com/questions/71047848/how-to-assign-or-return-generic-t-that-is-constrained-by-union
// https://go.dev/play/p/JVBEZwCXRMW
// func properties[V PropertyValue](propertyPairs []string) map[string]V {
func propertyMap(propertyPairs []string) map[string]interface{} {
	properties := make(map[string]interface{})
	for _, kv := range propertyPairs {
		parts := strings.FieldsFunc(kv, func(c rune) bool { return c == ':' })
		if len(parts) != 2 {
			fmt.Fprintf(os.Stderr, "Invalid key/value pair: %s\n", kv)
			os.Exit(1)
		}
		properties[parts[0]] = parts[1]
	}
	return properties
}
