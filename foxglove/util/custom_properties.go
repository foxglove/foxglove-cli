package util

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/foxglove/foxglove-cli/foxglove/api"
)

type OrgCustomProperties map[string]PropertyDefinition

type PropertyDefinition struct {
	Key        string
	ValueType  string
	EnumValues map[string]struct{}
}

// Validate CLI properties input & convert to args for a device request.
// This requires downloading the available properties for the org.
func DeviceProperties(propertyPairs []string, client *api.FoxgloveClient) (map[string]interface{}, error) {
	return ResourceProperties(propertyPairs, client, "device")
}

// EventProperties validates CLI properties input for event custom properties.
func EventProperties(propertyPairs []string, client *api.FoxgloveClient) (map[string]interface{}, error) {
	return ResourceProperties(propertyPairs, client, "event")
}

// ResourceProperties validates CLI properties input for a custom-property resource type.
func ResourceProperties(propertyPairs []string, client *api.FoxgloveClient, resourceType string) (map[string]interface{}, error) {
	if len(propertyPairs) == 0 {
		return nil, nil
	}

	propertyMap, err := fetchAvailableProperties(client, resourceType)
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

		parsed, err := parsePropertyValue(property, val)
		if err != nil {
			return nil, err
		}
		properties[key] = parsed
	}
	return properties, nil
}

func parsePropertyValue(property PropertyDefinition, val string) (interface{}, error) {
	switch property.ValueType {
	case "string", "multiline-string":
		return val, nil
	case "number":
		parsed, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid value for number: %s", val)
		}
		return parsed, nil
	case "enum":
		_, hasVal := property.EnumValues[val]
		if !hasVal {
			return nil, fmt.Errorf("invalid enum value: %s", val)
		}
		return val, nil
	case "multi-enum":
		parts := strings.Split(val, ",")
		values := make([]string, 0, len(parts))
		for _, part := range parts {
			part = strings.TrimSpace(part)
			if part == "" {
				continue
			}
			_, hasVal := property.EnumValues[part]
			if !hasVal {
				return nil, fmt.Errorf("invalid enum value: %s", part)
			}
			values = append(values, part)
		}
		return values, nil
	case "boolean":
		parsed, err := strconv.ParseBool(val)
		if err != nil {
			return nil, fmt.Errorf("invalid value for boolean: %s", val)
		}
		return parsed, nil
	default:
		return nil, fmt.Errorf("unsupported type: %s", property.ValueType)
	}
}

// Download custom properties for a resource type and convert to a lookup map
func fetchAvailableProperties(client *api.FoxgloveClient, resourceType string) (OrgCustomProperties, error) {
	propertiesResp, err := client.DeviceCustomProperties(api.CustomPropertiesRequest{
		ResourceType: resourceType,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to load custom properties: %w", err)
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
