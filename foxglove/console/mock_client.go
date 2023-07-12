package console

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func NewMockAuthedClient(t *testing.T, baseUrl string) *FoxgloveClient {
	client := NewRemoteFoxgloveClient(
		baseUrl,
		"client",
		"",
		"user-agent",
	)
	token, err := client.SignIn("client-id")
	assert.Nil(t, err)
	return NewRemoteFoxgloveClient(
		baseUrl,
		"client",
		token,
		"user-agent",
	)
}
