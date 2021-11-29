package cmd

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
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
	t.Run("returns forbidden if not authenticated", func(t *testing.T) {
		_, port := svc.NewMockServer(ctx)
		err := executeExport(
			fmt.Sprintf("http://localhost:%d", port),
			"abc",
			"test-device",
			"2020-01-01T00:00:00Z",
			"2021-01-01T00:00:00Z",
			"mcap",
			"/diagnostics",
			"",
		)
		assert.Equal(t, "Export failed: streaming request failure: forbidden", err.Error())
	})
	t.Run("returns empty data when requesting data that does not exist", func(t *testing.T) {
		buf := &bytes.Buffer{}
		err := withStdoutRedirected(buf, func() {
			_, port := svc.NewMockServer(ctx)
			client := svc.NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "client-id", "")
			token, err := client.SignIn("client-id")
			assert.Nil(t, err)
			err = executeExport(
				fmt.Sprintf("http://localhost:%d", port),
				"abc",
				"test-device",
				"2020-01-01T00:00:00Z",
				"2021-01-01T00:00:00Z",
				"mcap",
				"/diagnostics",
				token,
			)
			assert.Nil(t, err)
			assert.Nil(t, err)
		})
		assert.Nil(t, err)
		assert.Equal(t, 0, buf.Len())
	})
	//t.Run("returns valid bytes when target data exists", func(t *testing.T) {
	//	buf := &bytes.Buffer{}
	//	_, port := svc.NewMockServer(ctx)
	//	client := svc.NewRemoteFoxgloveClient(fmt.Sprintf("http://localhost:%d", port), "client-id", "")
	//	token, err := client.SignIn("client-id")
	//	assert.Nil(t, err)
	//	baseurl := fmt.Sprintf("http://localhost:%d", port)
	//	clientID := "client-id"
	//	deviceID := "test-device"
	//	err = executeImport(
	//		baseurl,
	//		clientID,
	//		deviceID,
	//		"../svc/testdata/gps.bag",
	//		token,
	//	)
	//	assert.Nil(t, err)
	//	err = withStdoutRedirected(buf, func() {
	//		err := executeExport(
	//			fmt.Sprintf("http://localhost:%d", port),
	//			clientID,
	//			deviceID,
	//			"2001-01-01T00:00:00Z",
	//			"2021-01-01T00:00:00Z",
	//			"bag1",
	//			"/diagnostics",
	//			token,
	//		)
	//		assert.Nil(t, err)
	//	})
	//	assert.Nil(t, err)
	//	assert.Equal(t, 0, buf.Len())
	//})
}
