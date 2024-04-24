package util

import (
	"context"
	"fmt"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestDeviceProperties(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.Nil(t, err)
	propertyOfType := func(valueType string) *api.CustomPropertiesResponseItem {
		for _, prop := range sv.RegisteredProperties() {
			if prop.ValueType == valueType {
				return &prop
			}
		}
		return nil
	}

	t.Run("returns error on unknown keys", func(t *testing.T) {
		client := newAuthedClient(t, sv.BaseURL())

		input := []string{"foo:bar"}
		_, err = DeviceProperties(input, client)
		assert.Equal(t, fmt.Errorf("unknown key: foo"), err)
	})

	t.Run("returns error for invalid value type", func(t *testing.T) {
		client := newAuthedClient(t, sv.BaseURL())

		numProp := propertyOfType("number")

		input := []string{fmt.Sprintf("%s:foo", numProp.Key)}
		_, err := DeviceProperties(input, client)
		assert.Equal(t, err, fmt.Errorf("invalid value for number: foo"))
	})

	t.Run("converts values based on valueType", func(t *testing.T) {
		client := newAuthedClient(t, sv.BaseURL())

		strProp := propertyOfType("string")
		numProp := propertyOfType("number")
		boolProp := propertyOfType("boolean")
		enumProp := propertyOfType("enum")

		input := []string{
			fmt.Sprintf("%s:bar", strProp.Key),
			fmt.Sprintf("%s:true", boolProp.Key),
			fmt.Sprintf("%s:1.5", numProp.Key),
			fmt.Sprintf("%s:%s", enumProp.Key, enumProp.Values[0]),
		}
		properties, err := DeviceProperties(input, client)
		assert.Nil(t, err)
		assert.Equal(t, properties, map[string]interface{}{
			strProp.Key:  "bar",
			numProp.Key:  1.5,
			boolProp.Key: true,
			enumProp.Key: enumProp.Values[0],
		})
	})
}

func newAuthedClient(t *testing.T, baseUrl string) *api.FoxgloveClient {
	client := api.NewRemoteFoxgloveClient(
		baseUrl,
		"client",
		"",
		"user-agent",
	)
	token, err := client.SignIn("client-id")
	assert.Nil(t, err)
	return api.NewRemoteFoxgloveClient(
		baseUrl,
		"client",
		token,
		"user-agent",
	)
}
