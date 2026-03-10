package cmd

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/foxglove/mcap/go/mcap"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func writeTestMCAP(t *testing.T, path string) {
	t.Helper()
	f, err := os.Create(path)
	require.NoError(t, err)
	defer f.Close()

	writer, err := mcap.NewWriter(f, &mcap.WriterOptions{
		Chunked:   true,
		ChunkSize: 1024 * 1024,
	})
	require.NoError(t, err)
	require.NoError(t, writer.WriteHeader(&mcap.Header{}))
	require.NoError(t, writer.WriteSchema(&mcap.Schema{
		ID:       1,
		Name:     "s1",
		Encoding: "ros1msg",
		Data:     []byte{},
	}))
	require.NoError(t, writer.WriteSchema(&mcap.Schema{
		ID:       2,
		Name:     "s2",
		Encoding: "ros1msg",
		Data:     []byte{},
	}))
	require.NoError(t, writer.WriteChannel(&mcap.Channel{
		ID:       1,
		SchemaID: 1,
		Topic:    "/foo",
	}))
	require.NoError(t, writer.WriteChannel(&mcap.Channel{
		ID:       2,
		SchemaID: 2,
		Topic:    "/bar",
	}))
	require.NoError(t, writer.WriteMessage(&mcap.Message{
		ChannelID:   1,
		Sequence:    1,
		LogTime:     1,
		PublishTime: 1,
		Data:        []byte{1, 2, 3},
	}))
	require.NoError(t, writer.WriteMessage(&mcap.Message{
		ChannelID:   2,
		Sequence:    2,
		LogTime:     2,
		PublishTime: 2,
		Data:        []byte{4, 5, 6},
	}))
	require.NoError(t, writer.Close())
}

func readMCAPTopics(t *testing.T, path string) []string {
	t.Helper()
	f, err := os.Open(path)
	require.NoError(t, err)
	defer f.Close()

	lexer, err := mcap.NewLexer(f, &mcap.LexerOptions{})
	require.NoError(t, err)

	topics := []string{}
	for {
		tokenType, token, err := lexer.Next(nil)
		require.NoError(t, err)
		if tokenType == mcap.TokenChannel {
			channel, err := mcap.ParseChannel(token)
			require.NoError(t, err)
			topics = append(topics, channel.Topic)
		}
		if tokenType == mcap.TokenDataEnd {
			break
		}
	}
	return topics
}

func TestMCAPRenameCommand_OutputFile(t *testing.T) {
	tmp := t.TempDir()
	inputPath := filepath.Join(tmp, "input.mcap")
	outputPath := filepath.Join(tmp, "output.mcap")
	writeTestMCAP(t, inputPath)

	cmd := newMcapRenameCommand()
	cmd.SetArgs([]string{inputPath, "--from", "/foo", "--to", "/foo_renamed", "--output", outputPath})

	err := cmd.Execute()
	require.NoError(t, err)
	assert.Equal(t, []string{"/foo", "/bar"}, readMCAPTopics(t, inputPath))
	assert.Equal(t, []string{"/foo_renamed", "/bar"}, readMCAPTopics(t, outputPath))
}

func TestMCAPRenameCommand_InPlace(t *testing.T) {
	tmp := t.TempDir()
	inputPath := filepath.Join(tmp, "input.mcap")
	writeTestMCAP(t, inputPath)

	cmd := newMcapRenameCommand()
	cmd.SetArgs([]string{inputPath, "--from", "/foo", "--to", "/foo_renamed"})

	err := cmd.Execute()
	require.NoError(t, err)
	assert.Equal(t, []string{"/foo_renamed", "/bar"}, readMCAPTopics(t, inputPath))
}

func TestMCAPRenameCommand_MissingTopic(t *testing.T) {
	tmp := t.TempDir()
	inputPath := filepath.Join(tmp, "input.mcap")
	outputPath := filepath.Join(tmp, "output.mcap")
	writeTestMCAP(t, inputPath)

	cmd := newMcapRenameCommand()
	cmd.SetArgs([]string{inputPath, "--from", "/missing", "--to", "/newname", "--output", outputPath})

	err := cmd.Execute()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "was not found")
	_, statErr := os.Stat(outputPath)
	assert.Error(t, statErr)
	assert.True(t, os.IsNotExist(statErr))
}

func TestMCAPRenameCommand_FlagValidation(t *testing.T) {
	tmp := t.TempDir()
	inputPath := filepath.Join(tmp, "input.mcap")
	writeTestMCAP(t, inputPath)

	t.Run("requires from", func(t *testing.T) {
		cmd := newMcapRenameCommand()
		cmd.SetArgs([]string{inputPath, "--to", "/new"})
		err := cmd.Execute()
		require.Error(t, err)
		assert.Contains(t, err.Error(), "required flag \"from\" not set")
	})

	t.Run("requires to", func(t *testing.T) {
		cmd := newMcapRenameCommand()
		cmd.SetArgs([]string{inputPath, "--from", "/foo"})
		err := cmd.Execute()
		require.Error(t, err)
		assert.Contains(t, err.Error(), "required flag \"to\" not set")
	})

	t.Run("from and to must differ", func(t *testing.T) {
		cmd := newMcapRenameCommand()
		cmd.SetArgs([]string{inputPath, "--from", "/foo", "--to", "/foo"})
		err := cmd.Execute()
		require.Error(t, err)
		assert.Contains(t, err.Error(), "must be different")
	})
}
