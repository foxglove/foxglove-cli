package cmd

import (
	"context"
	"os"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/stretchr/testify/assert"
	"gopkg.in/yaml.v2"
)

func TestLoginCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := svc.NewMockServer(ctx)
	assert.Nil(t, err)
	configfile := "./test-config.yaml"
	err = initConfig(&configfile)
	assert.Nil(t, err)
	err = executeLogin(sv.BaseURL(), "client-id", "test-app")
	assert.Nil(t, err)
	assert.NotEmpty(t, sv.BearerTokens)
	m := make(map[string]string)
	f, err := os.Open(configfile)
	assert.Nil(t, err)
	defer os.Remove(configfile)
	defer f.Close()
	err = yaml.NewDecoder(f).Decode(&m)
	assert.Nil(t, err)
	assert.NotEmpty(t, m["bearer_token"])
}
