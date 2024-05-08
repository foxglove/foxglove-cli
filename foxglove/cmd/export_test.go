package cmd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/foxglove/go-rosbag"
	"github.com/foxglove/mcap/go/mcap"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
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

func TestCombineMCAPTmpFiles(t *testing.T) {
	parts := []*bytes.Buffer{}
	for i := 0; i < 3; i++ {
		part := &bytes.Buffer{}
		writer, err := mcap.NewWriter(part, &mcap.WriterOptions{
			Chunked:   true,
			ChunkSize: 1024 * 1024,
		})
		assert.Nil(t, err)
		assert.Nil(t, writer.WriteHeader(&mcap.Header{}))
		assert.Nil(t, writer.WriteSchema(&mcap.Schema{
			ID:       1,
			Name:     "s1",
			Encoding: "ros1msg",
			Data:     []byte{},
		}))
		assert.Nil(t, writer.WriteChannel(&mcap.Channel{
			ID:       0,
			SchemaID: 1,
			Topic:    "c0",
		}))
		assert.Nil(t, writer.WriteSchema(&mcap.Schema{
			ID:       2,
			Name:     "s2",
			Encoding: "ros1msg",
			Data:     []byte{},
		}))
		assert.Nil(t, writer.WriteChannel(&mcap.Channel{
			ID:       1,
			SchemaID: 2,
			Topic:    "c1",
		}))

		for j := 0; j < 1000; j++ {
			assert.Nil(t, writer.WriteMessage(&mcap.Message{
				ChannelID:   uint16(i % 2),
				Sequence:    0,
				LogTime:     uint64(i*1000 + j),
				PublishTime: 0,
				Data:        []byte{},
			}))
		}
		assert.Nil(t, writer.Close())
		parts = append(parts, part)
	}

	output := &bytes.Buffer{}
	tmpfiles := []partialFile{}
	for i, part := range parts {
		tmpfiles = append(tmpfiles, partialFile{
			rs:   bytes.NewReader(part.Bytes()),
			name: fmt.Sprintf("foo-%d", i),
			info: &fileInfo{
				maxTime:      uint64(1000 * (i + 1)),
				messageCount: 1000,
			},
		})
	}

	assert.Nil(t, combineMCAPTmpFiles(output, tmpfiles))

	reader, err := mcap.NewReader(bytes.NewReader(output.Bytes()))
	assert.Nil(t, err)
	info, err := reader.Info()
	assert.Nil(t, err)
	assert.Equal(t, 3000, int(info.Statistics.MessageCount))
	assert.Equal(t, 6, int(info.Statistics.ChannelCount))
	assert.Equal(t, 6, int(info.Statistics.SchemaCount))
}

func TestCombineBagTempfiles(t *testing.T) {
	parts := []*bytes.Buffer{}
	for i := 0; i < 3; i++ {
		part := &bytes.Buffer{}
		writer, err := rosbag.NewWriter(part)
		assert.Nil(t, err)
		assert.Nil(t, writer.WriteConnection(&rosbag.Connection{
			Conn:  0,
			Topic: "/foo",
			Data: rosbag.ConnectionHeader{
				Topic:             "/foo",
				Type:              "std_msgs/String",
				MD5Sum:            "abc",
				MessageDefinition: []byte{},
			},
		}))
		assert.Nil(t, writer.WriteConnection(&rosbag.Connection{
			Conn:  1,
			Topic: "/bar",
			Data: rosbag.ConnectionHeader{
				Topic:             "/foo",
				Type:              "std_msgs/String",
				MD5Sum:            "abc",
				MessageDefinition: []byte{},
			},
		}))
		for j := 0; j < 1000; j++ {
			assert.Nil(t, writer.WriteMessage(&rosbag.Message{
				Conn: uint32(j % 2),
				Time: uint64(i*1000 + j),
				Data: []byte{},
			}))
		}
		assert.Nil(t, writer.Close())
		parts = append(parts, part)
	}
	tmpfiles := []partialFile{}
	for i, part := range parts {
		tmpfiles = append(tmpfiles, partialFile{
			rs:   bytes.NewReader(part.Bytes()),
			name: fmt.Sprintf("foo-%d", i),
			info: &fileInfo{
				maxTime:      uint64(1000 * (i + 1)),
				messageCount: 1000,
			},
		})
	}
	output := util.NewBufWriteSeeker()
	assert.Nil(t, combineBagTmpFiles(output, tmpfiles))
	reader, err := rosbag.NewReader(bytes.NewReader(output.Bytes()))
	assert.Nil(t, err)
	info, err := reader.Info()
	assert.Nil(t, err)
	t.Run("has expected message count", func(t *testing.T) {
		assert.Equal(t, 3000, int(info.MessageCount))
	})
	t.Run("contains expected timestamps", func(t *testing.T) {
		assert.Equal(t, 2999, int(info.MessageEndTime))
	})
}

func TestDoExport(t *testing.T) {
	ctx := context.Background()
	end, err := time.Parse(time.RFC3339, "2021-01-01T00:00:00Z")
	assert.Nil(t, err)

	t.Run("single request success", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		start, err := time.Parse(time.RFC3339, "2001-01-01T00:00:00Z")
		assert.Nil(t, err)
		clientID := "client-id"
		deviceID := "test-device"
		err = executeImport(
			sv.BaseURL(),
			clientID,
			deviceID,
			"",
			"",
			"../testdata/gps.mcap",
			token,
			"user-agent",
		)
		assert.Nil(t, err)

		t.Cleanup(func() {
			os.Remove("output.mcap")
		})

		err = doExport(
			ctx,
			"output.mcap",
			sv.BaseURL(),
			"abc",
			token,
			"user-agent",
			&api.StreamRequest{
				DeviceID:     "test-device",
				Start:        &start,
				End:          &end,
				OutputFormat: "mcap0",
				Topics:       []string{"/diagnostics"},
			},
		)
		assert.Nil(t, err)
	})
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
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExport(
			ctx,
			buf,
			sv.BaseURL(),
			"",
			"abc",
			"user-agent",
			&api.StreamRequest{
				DeviceID:     "test-device",
				Start:        &start,
				End:          &end,
				OutputFormat: "mcap0",
				Topics:       []string{"/diagnostics"},
			},
		)
		assert.ErrorIs(t, err, api.ErrForbidden)
	})
	t.Run("returns error on invalid format", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		buf := &bytes.Buffer{}
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		err = executeExport(
			ctx,
			buf,
			sv.BaseURL(),
			"",
			"abc",
			"user-agent",
			&api.StreamRequest{
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
			sv, err := api.NewMockServer(ctx)
			assert.Nil(t, err)
			client := api.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
			token, err := client.SignIn("client-id")
			assert.Nil(t, err)
			err = executeExport(
				ctx,
				buf,
				sv.BaseURL(),
				"abc",
				token,
				"user-agent",
				&api.StreamRequest{
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
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		clientID := "client-id"
		deviceID := "test-device"
		err = executeImport(
			sv.BaseURL(),
			clientID,
			deviceID,
			"",
			"",
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
				&api.StreamRequest{
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
		sv, err := api.NewMockServer(ctx)
		assert.Nil(t, err)
		client := api.NewRemoteFoxgloveClient(sv.BaseURL(), "client-id", "", "test-app")
		token, err := client.SignIn("client-id")
		assert.Nil(t, err)
		clientID := "client-id"
		deviceID := "test-device"
		err = executeImport(
			sv.BaseURL(),
			clientID,
			deviceID,
			"",
			"",
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
				&api.StreamRequest{
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

func copyTo(t *testing.T, from, to string) {
	infile, err := os.Open(from)
	require.NoError(t, err)
	defer func() {
		require.NoError(t, infile.Close())
	}()
	outfile, err := os.Create(to)
	require.NoError(t, err)
	defer func() {
		require.NoError(t, outfile.Close())
	}()
	_, err = io.Copy(outfile, infile)
	require.NoError(t, err)
}

func TestReindexBag(t *testing.T) {
	workingPath := filepath.Join(t.TempDir(), "gps.bag.active")
	copyTo(t, "../testdata/gps.bag.active", workingPath)
	didReindex, info, err := reindex(t.TempDir(), workingPath, "bag1")
	require.NoError(t, err)
	require.True(t, didReindex)
	require.Equal(t, 30445, int(info.messageCount))

	// check that the new bag is indexed by retrieving Info() from it
	fd, err := os.Open(workingPath)
	require.NoError(t, err)
	defer func() {
		require.NoError(t, fd.Close())
	}()
	reader, err := rosbag.NewReader(fd)
	require.NoError(t, err)
	bagInfo, err := reader.Info()
	require.NoError(t, err)
	require.Equal(t, 30445, int(bagInfo.MessageCount))
}
