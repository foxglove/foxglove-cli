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
	err := initConfig("./test-config.yaml")
	assert.Nil(t, err)
	err = executeLogin(fmt.Sprintf("http://localhost:%d", port), "client-id")
	assert.Nil(t, err)
	assert.NotEmpty(t, svc.BearerTokens)
	m := make(map[string]string)
	filename := "./test-config.yaml"
	f, err := os.Open(filename)
	assert.Nil(t, err)
	defer os.Remove(filename)
	defer f.Close()
	err = yaml.NewDecoder(f).Decode(&m)
	assert.Nil(t, err)
	assert.NotEmpty(t, m["bearer_token"])
}
