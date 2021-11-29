package svc

import (
	"bytes"
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func login(ctx context.Context, sv *MockFoxgloveServer) (string, error) {
	client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", sv.port), "abc", "")
	return Login(ctx, client)
}

func TestImport(t *testing.T) {
	ctx := context.Background()
	t.Run("returns error forbidden when not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		_, port := NewMockServer(ctx)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", "")
		err := Import(ctx, client, "test-device", "./testdata/gps.bag")
		assert.ErrorIs(t, err, ErrForbidden)
	})
	t.Run("successfully imports data after auth", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		srv, port := NewMockServer(ctx)
		token, err := login(ctx, srv)
		assert.Nil(t, err)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", token)
		err = Import(ctx, client, "test-device", "./testdata/gps.bag")
		assert.Nil(t, err)
		assert.Equal(t, 5324051, len(srv.Uploads["device_id=test-device/gps.bag"]))
	})
}

func TestExport(t *testing.T) {
	ctx := context.Background()
	t.Run("returns error forbidden when not authenticated", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		_, port := NewMockServer(ctx)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", "")
		buf := &bytes.Buffer{}
		err := Export(ctx, buf, client, "test-device", "2020-01-01T00:00:00Z", "2021-01-01T00:00:00Z", []string{}, "mcap")
		assert.ErrorIs(t, err, ErrForbidden)
	})
	t.Run("returns empty data when nothing matches", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		sv, port := NewMockServer(ctx)
		token, err := login(ctx, sv)
		assert.Nil(t, err)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", token)
		buf := &bytes.Buffer{}
		err = Export(ctx, buf, client, "test-device", "2020-01-01T00:00:00Z", "2021-01-01T00:00:00Z", []string{}, "mcap")
		assert.Nil(t, err)
		assert.Empty(t, buf.Bytes())
	})
	t.Run("returns data file when data exists", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		sv, port := NewMockServer(ctx)
		token, err := login(ctx, sv)
		assert.Nil(t, err)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", token)
		buf := &bytes.Buffer{}
		err = Import(ctx, client, "test-device", "./testdata/gps.bag")
		assert.Nil(t, err)
		err = Export(ctx, buf, client, "test-device", "2020-01-01T00:00:00Z", "2021-01-01T00:00:00Z", []string{}, "mcap")
		assert.Nil(t, err)
		assert.NotEmpty(t, buf.Bytes())
	})
}

func TestLogin(t *testing.T) {
	ctx := context.Background()
	t.Run("returns nonempty bearer token", func(t *testing.T) {
		ctx, cancel := context.WithCancel(ctx)
		defer cancel()
		_, port := NewMockServer(ctx)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", "")
		bearerToken, err := Login(ctx, client)
		assert.Nil(t, err)
		assert.NotEmpty(t, bearerToken)
	})
	t.Run("times out if token does not appear", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 600*time.Millisecond)
		defer cancel()
		_, port := NewMockServer(ctx)
		client := NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "abc", "")
		bearerToken, err := Login(ctx, client)
		assert.ErrorIs(t, context.Canceled, err)
		assert.Empty(t, bearerToken)
	})
}
