package util

import (
	"fmt"
	"strconv"

	"github.com/foxglove/foxglove-cli/foxglove/console"
)

type OrgCustomProperties map[string]PropertyDefinition

type PropertyDefinition struct {
	Key        string
	ValueType  string
	EnumValues map[string]struct{}
}

// Validate CLI properties input & convert to args for a device request.
// This requires downloading the available properties for the org.
func DeviceProperties(propertyPairs []string, client *console.FoxgloveClient) (map[string]interface{}, error) {
	if len(propertyPairs) == 0 {
		return nil, nil
	}

	propertyMap, err := fetchAvailableProperties(client)
	if err != nil {
		return nil, fmt.Errorf("%s", err)
	}

	properties := make(map[string]interface{})
	for _, kv := range propertyPairs {
		key, val, err := SplitPair(kv, ':')
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
