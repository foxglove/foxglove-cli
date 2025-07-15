package cmd

import (
	"context"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/stretchr/testify/assert"
)

func TestListProjectsCommand(t *testing.T) {
	ctx := context.Background()
	sv, err := api.NewMockServer(ctx)
	assert.Nil(t, err)

	t.Run("lists projects", func(t *testing.T) {
		client := api.NewMockAuthedClient(t, sv.BaseURL())
		projects, err := client.Projects(api.ProjectsRequest{})
		assert.Nil(t, err)

		assert.Contains(t, projects, api.ProjectsResponse{
			ID:             "prj_1234abcd",
			Name:           "My First Project",
			OrgMemberCount: 11,
			LastSeenAt:     nil,
		})
	})
}
