package cmd

import (
	"fmt"
	"os"
	"strconv"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
)

type PropertyDefinition struct {
	Key        string
	ValueType  string
	EnumValues map[string]struct{}
}

type OrgCustomProperties map[string]PropertyDefinition

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

			properties, err := deviceProperties(propertyPairs, client)
			if err != nil {
				fatalf("Failed to create device: %s\n", err)
			}

			resp, err := client.CreateDevice(console.CreateDeviceRequest{
				Name:       name,
				Properties: properties,
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

// Validate CLI properties input & convert to args for a device request.
// This requires downloading the available properties for the org.
func deviceProperties(propertyPairs []string, client *console.FoxgloveClient) (map[string]interface{}, error) {
	if len(propertyPairs) == 0 {
		return nil, nil
	}

	propertyMap, err := fetchAvailableProperties(client)
	if err != nil {
		return nil, fmt.Errorf("%s", err)
	}

	properties := make(map[string]interface{})
	for _, kv := range propertyPairs {
		key, val, err := splitPair(kv, ':')
		if err != nil {
			return nil, err
		}

		property, hasKey := propertyMap[key]
		if !hasKey {
			return nil, fmt.Errorf("unknown key: %s", key)
		}

		switch property.ValueType {
		case "string":
			properties[key] = val
		case "number":
			properties[key], err = strconv.ParseFloat(val, 64)
			if err != nil {
				return nil, fmt.Errorf("invalid value for number: %s", val)
			}
		case "enum":
			_, hasVal := property.EnumValues[val]
			if !hasVal {
				return nil, fmt.Errorf("invalid enum value: %s", val)
			}
			properties[key] = val
		case "boolean":
			properties[key], err = strconv.ParseBool(val)
			if err != nil {
				return nil, fmt.Errorf("invalid value for boolean: %s", val)
			}
		default:
			return nil, fmt.Errorf("unsupported type: %s", property.ValueType)
		}
	}
	return properties, nil
}

// Reduce response items into a map keyed by the property's key
func propertyDefinitions(props []console.CustomPropertiesResponseItem) (map[string]console.CustomPropertiesResponseItem, error) {
	properties := make(map[string]console.CustomPropertiesResponseItem)
	for _, prop := range props {
		properties[prop.Key] = prop
	}
	return properties, nil
}

// Download device custom properties and convert to a lookup map
func fetchAvailableProperties(client *console.FoxgloveClient) (OrgCustomProperties, error) {
	propertiesResp, err := client.DeviceCustomProperties(console.CustomPropertiesRequest{
		ResourceType: "device",
	})
	if err != nil {
		return nil, fmt.Errorf("failed to load custom properties: %s\n", err)
	}

	properties := make(map[string]PropertyDefinition)
	for _, prop := range propertiesResp {
		properties[prop.Key] = PropertyDefinition{
			Key:        prop.Key,
			ValueType:  prop.ValueType,
			EnumValues: valueSet(prop.Values),
		}
	}

	return properties, nil
}

// Reduce a slice of strings into a map with empty values
func valueSet(values []string) map[string]struct{} {
	var present struct{}
	valSet := make(map[string]struct{})
	for _, val := range values {
		valSet[val] = present
	}
	return valSet
}
