package cmd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"os"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/stretchr/testify/assert"
)

func withStdoutRedirected(output io.Writer, f func()) error {
	stdout := os.Stdout
	r, w, err := os.Pipe()
	if err != nil {
		return err
	}
	os.Stdout = w

	f()
	w.Close()

	defer func() {
		os.Stdout = stdout
	}()

	_, err = io.Copy(output, r)
	if err != nil {
		return err
	}
	return nil
}

func TestExportCommand(t *testing.T) {
	ctx := context.Background()
	start, err := time.Parse(time.RFC3339, "2020-01-01T00:00:00Z")
	assert.Nil(t, err)
	end, err := time.Parse(time.RFC3339, "2021-01-01T00:00:00Z")
	assert.Nil(t, err)
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		buf := &bytes.Buffer{}
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExport(
			ctx,
			buf,
			sv.BaseURL(),
			"",
			"abc",
			"user-agent",
			&console.StreamRequest{
				DeviceID:     "test-device",
				Start:        &start,
				End:          &end,
				OutputFormat: "mcap0",
				Topics:       []string{"/diagnostics"},
			},
		)
		assert.ErrorIs(t, err, console.ErrForbidden)
	})
	t.Run("returns error on invalid format", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		buf := &bytes.Buffer{}
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExport(
			ctx,
			buf,
			sv.BaseURL(),
			"",
			"abc",
			"user-agent",
			&console.StreamRequest{
				DeviceID:     "test-device",
				Start:        &start,
				End:          &end,
				OutputFormat: "mcap",
				Topics:       []string{"/diagnostics"},
			},
		)
		assert.ErrorIs(t, err, ErrInvalidFormat)
	})
	t.Run("returns empty data when requesting data that does not exist", func(t *testing.T) {
		buf := &bytes.Buffer{}
		err := withStdoutRedirected(buf, func() {
			ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
			defer cancel()
			sv, err := console.NewMockServer(ctx)
			assert.Nil(t, err)
			client := console.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
			token, err := client.SignIn("client-id")
			assert.Nil(t, err)
			err = executeExport(
				ctx,
				buf,
				sv.BaseURL(),
				"abc",
				token,
				"user-agent",
				&console.StreamRequest{
					DeviceID:     "test-device",
					Start:        &start,
					End:          &end,
					OutputFormat: "mcap0",
					Topics:       []string{"/diagnostics"},
				},
			)
			assert.Nil(t, err)
		})
		assert.Nil(t, err)
		assert.Equal(t, 0, buf.Len())
	})
	t.Run("returns valid bytes when target data exists", func(t *testing.T) {
		buf := &bytes.Buffer{}
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		client := console.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		clientID := "client-id"
		deviceID := "test-device"
		err = executeImport(
			sv.BaseURL(),
			clientID,
			deviceID,
			"../testdata/gps.bag",
			token,
			"user-agent",
		)
		assert.Nil(t, err)
		start, err := time.Parse(time.RFC3339, "2001-01-01T00:00:00Z")
		assert.Nil(t, err)
		err = withStdoutRedirected(buf, func() {
			err := executeExport(
				ctx,
				buf,
				sv.BaseURL(),
				"abc",
				token,
				"user-agent",
				&console.StreamRequest{
					DeviceID:     "test-device",
					Start:        &start,
					End:          &end,
					OutputFormat: "bag1",
					Topics:       []string{"/diagnostics"},
				},
			)
			assert.Nil(t, err)
		})
		assert.Nil(t, err)
		assert.Equal(t, 5324051, buf.Len())
	})
	t.Run("returns JSON data when requested", func(t *testing.T) {
		buf := &bytes.Buffer{}
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := console.NewMockServer(ctx)
		assert.Nil(t, err)
		client := console.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		clientID := "client-id"
		deviceID := "test-device"
		err = executeImport(
			sv.BaseURL(),
			clientID,
			deviceID,
			"../testdata/gps.mcap",
			token,
			"user-agent",
		)
		assert.Nil(t, err)
		start, err := time.Parse(time.RFC3339, "2001-01-01T00:00:00Z")
		assert.Nil(t, err)
		err = withStdoutRedirected(buf, func() {
			err := executeExport(
				ctx,
				buf,
				sv.BaseURL(),
				"abc",
				token,
				"user-agent",
				&console.StreamRequest{
					DeviceID:     "test-device",
					Start:        &start,
					End:          &end,
					OutputFormat: "json",
					Topics:       []string{"/diagnostics"},
				},
			)
			assert.Nil(t, err)
		})
		assert.Nil(t, err)
		count := 0
		d := json.NewDecoder(buf)
		for {
			var msg Message
			err := d.Decode(&msg)
			if errors.Is(err, io.EOF) {
				break
			}
			assert.Nil(t, err)
			count++
		}
		assert.Equal(t, 30445, count)
	})
}
