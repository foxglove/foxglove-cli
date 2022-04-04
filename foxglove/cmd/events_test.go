package cmd

import (
	"bytes"
	"context"
	"encoding/csv"
	"encoding/json"
	"io"
	"strings"
	"testing"

	"github.com/foxglove/foxglove-cli/foxglove/svc"
	"github.com/stretchr/testify/assert"
)

func TestListEvents(t *testing.T) {
	ctx := context.Background()
	sv, err := svc.NewMockServer(ctx)
	assert.Nil(t, err)
	client := svc.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
	token, err := client.SignIn("client-id")
	assert.Nil(t, err)
	upload := &bytes.Buffer{}
	err = client.Upload(upload, svc.UploadRequest{
		Filename: "upload.txt",
		DeviceID: "test-device",
	})
	assert.Nil(t, err)

	cases := []struct {
		assertion string
		format    string
		parseFn   func(io.Reader) error
	}{
		{
			"json",
			"json",
			func(r io.Reader) error {
				var v []svc.ImportsResponse
				err := json.NewDecoder(r).Decode(&v)
				assert.Equal(t, 1, len(v))
				return err
			},
		},
		{
			"csv",
			"csv",
			func(r io.Reader) error {
				reader := csv.NewReader(r)
				result, err := reader.ReadAll()
				assert.Equal(t, 2, len(result)) // header included
				return err
			},
		},
		{
			"table",
			"table",
			func(r io.Reader) error {
				bytes, err := io.ReadAll(r)
				assert.Nil(t, err)
				lines := strings.FieldsFunc(string(bytes), func(c rune) bool { return c == '\n' })
				assert.Equal(t, 3, len(lines)) // header + separator + one data record
				return err
			},
		},
	}
	for _, c := range cases {
		t.Run(c.assertion, func(t *testing.T) {
			t.Run("returns error if unauthenticated", func(t *testing.T) {
				buf := &bytes.Buffer{}
				err = listImports(
					buf,
					sv.BaseURL(),
					"fake-client",
					c.format, "fake token", "user-agent",
					&svc.ImportsRequest{},
				)
				assert.ErrorIs(t, err, svc.ErrForbidden)
			})
		})
		t.Run("returns parsable data when authenticated", func(t *testing.T) {
			buf := &bytes.Buffer{}
			err = listImports(
				buf,
				sv.BaseURL(),
				"client-id",
				c.format,
				token,
				"user-agent",
				&svc.ImportsRequest{},
			)
			assert.Nil(t, err)
			err = c.parseFn(buf)
			assert.Nil(t, err)
		})
	}
}
