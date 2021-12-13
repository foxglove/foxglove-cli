package cmd

import (
	"bytes"
	"context"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/stretchr/testify/assert"
)

func TestListDevices(t *testing.T) {
	ctx := context.Background()
	for _, format := range []string{"table", "json", "csv"} {
		t.Run(fmt.Sprintf(
			"format %s returns forbidden if not authenticated", format),
			func(t *testing.T) {
				ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
				defer cancel()
				sv, err := svc.NewMockServer(ctx)
				assert.Nil(t, err)
				buf := &bytes.Buffer{}
				err = listDevices(
					buf,
					sv.BaseURL(),
					"abc",
					format,
					"",
					"user-agent",
				)
				assert.ErrorIs(t, err, svc.ErrForbidden)
			})
		t.Run(fmt.Sprintf(
			"format %s authenticated requests succeed without error", format),
			func(t *testing.T) {
				ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
				defer cancel()
				sv, err := svc.NewMockServer(ctx)
				assert.Nil(t, err)
				buf := &bytes.Buffer{}
				client := svc.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
				token, err := client.SignIn("client-id")
				assert.Nil(t, err)
				err = listDevices(
					buf,
					sv.BaseURL(),
					"client-id",
					format,
					token,
					"user-agent",
				)
				assert.Nil(t, err)
			})
	}
	t.Run("authenticated csv request results in parseable data", func(t *testing.T) {
		buf := &bytes.Buffer{}
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := svc.NewMockServer(ctx)
		assert.Nil(t, err)
		client := svc.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		err = listDevices(
			buf,
			sv.BaseURL(),
			"client-id",
			"csv",
			token,
			"user-agent",
		)
		assert.Nil(t, err)
		reader := csv.NewReader(buf)
		devices, err := reader.ReadAll()
		assert.Nil(t, err)
		assert.Equal(t, 2, len(devices)) // header record + one data record
	})
	t.Run("authenticated json request results in parseable data", func(t *testing.T) {
		buf := &bytes.Buffer{}
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := svc.NewMockServer(ctx)
		assert.Nil(t, err)
		client := svc.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		err = listDevices(
			buf,
			sv.BaseURL(),
			"client-id",
			"json",
			token,
			"user-agent",
		)
		assert.Nil(t, err)
		devices := []svc.DeviceResponse{}
		err = json.NewDecoder(buf).Decode(&devices)
		assert.Nil(t, err)
		assert.Nil(t, err)
		assert.Equal(t, 1, len(devices))
	})
}
