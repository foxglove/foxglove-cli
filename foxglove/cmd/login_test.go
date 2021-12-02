package cmd

import (
	"context"
	"fmt"
	"os"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/stretchr/testify/assert"
	"gopkg.in/yaml.v2"
)

func TestLoginCommand(t *testing.T) {
	ctx := context.Background()
	svc, port := svc.NewMockServer(ctx)
	configfile := "./test-config.yaml"
	err := initConfig(&configfile)
	assert.Nil(t, err)
	err = executeLogin(fmt.Sprintf("http://localhost:%d", port), "client-id")
	assert.Nil(t, err)
	assert.NotEmpty(t, svc.BearerTokens)
	m := make(map[string]string)
	f, err := os.Open(configfile)
	assert.Nil(t, err)
	defer os.Remove(configfile)
	defer f.Close()
	err = yaml.NewDecoder(f).Decode(&m)
	assert.Nil(t, err)
	assert.NotEmpty(t, m["bearer_token"])
}
