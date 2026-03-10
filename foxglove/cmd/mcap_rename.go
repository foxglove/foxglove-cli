package cmd

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/foxglove/mcap/go/mcap"
	"github.com/spf13/cobra"
)

func rewriteMCAPTopics(dst io.Writer, src io.Reader, fromTopic string, toTopic string) (int, error) {
	writer, err := mcap.NewWriter(dst, &mcap.WriterOptions{
		Chunked:     true,
		ChunkSize:   1024 * 1024,
		Compression: mcap.CompressionLZ4,
	})
	if err != nil {
		return 0, fmt.Errorf("failed to construct output writer: %w", err)
	}
	lexer, err := mcap.NewLexer(src, &mcap.LexerOptions{
		AttachmentCallback: func(ar *mcap.AttachmentReader) error {
			return writer.WriteAttachment(&mcap.Attachment{
				LogTime:    ar.LogTime,
				CreateTime: ar.CreateTime,
				Name:       ar.Name,
				MediaType:  ar.MediaType,
				DataSize:   ar.DataSize,
				Data:       ar.Data(),
			})
		},
	})
	if err != nil {
		return 0, fmt.Errorf("failed to construct input lexer: %w", err)
	}

	renamed := 0
	for {
		tokenType, token, err := lexer.Next(nil)
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return renamed, fmt.Errorf("failed to read input MCAP: %w", err)
		}
		switch tokenType {
		case mcap.TokenHeader:
			header, err := mcap.ParseHeader(token)
			if err != nil {
				return renamed, fmt.Errorf("failed to parse header: %w", err)
			}
			if err := writer.WriteHeader(header); err != nil {
				return renamed, fmt.Errorf("failed to write header: %w", err)
			}
		case mcap.TokenSchema:
			schema, err := mcap.ParseSchema(token)
			if err != nil {
				return renamed, fmt.Errorf("failed to parse schema: %w", err)
			}
			if err := writer.WriteSchema(schema); err != nil {
				return renamed, fmt.Errorf("failed to write schema: %w", err)
			}
		case mcap.TokenChannel:
			channel, err := mcap.ParseChannel(token)
			if err != nil {
				return renamed, fmt.Errorf("failed to parse channel: %w", err)
			}
			if channel.Topic == fromTopic {
				channel.Topic = toTopic
				renamed++
			}
			if err := writer.WriteChannel(channel); err != nil {
				return renamed, fmt.Errorf("failed to write channel: %w", err)
			}
		case mcap.TokenMessage:
			message, err := mcap.ParseMessage(token)
			if err != nil {
				return renamed, fmt.Errorf("failed to parse message: %w", err)
			}
			if err := writer.WriteMessage(message); err != nil {
				return renamed, fmt.Errorf("failed to write message: %w", err)
			}
		case mcap.TokenMetadata:
			metadata, err := mcap.ParseMetadata(token)
			if err != nil {
				return renamed, fmt.Errorf("failed to parse metadata: %w", err)
			}
			if err := writer.WriteMetadata(metadata); err != nil {
				return renamed, fmt.Errorf("failed to write metadata: %w", err)
			}
		case mcap.TokenDataEnd:
			if err := writer.Close(); err != nil {
				return renamed, fmt.Errorf("failed to finalize output: %w", err)
			}
			return renamed, nil
		}
	}
	if err := writer.Close(); err != nil {
		return renamed, fmt.Errorf("failed to finalize output: %w", err)
	}
	return renamed, nil
}

func renameMCAPFile(inputPath string, outputPath string, fromTopic string, toTopic string) error {
	input, err := os.Open(inputPath)
	if err != nil {
		return fmt.Errorf("failed to open input file: %w", err)
	}
	defer input.Close()

	var targetPath string
	if outputPath == "" {
		targetPath = inputPath
	} else {
		targetPath = outputPath
	}

	inPlace := targetPath == inputPath
	if inPlace {
		tmpfile, err := os.CreateTemp(filepath.Dir(inputPath), "mcap-rename-*")
		if err != nil {
			return fmt.Errorf("failed to create temporary file: %w", err)
		}
		tmpname := tmpfile.Name()
		renamed, rewriteErr := rewriteMCAPTopics(tmpfile, input, fromTopic, toTopic)
		closeErr := tmpfile.Close()
		if rewriteErr != nil {
			_ = os.Remove(tmpname)
			return rewriteErr
		}
		if closeErr != nil {
			_ = os.Remove(tmpname)
			return fmt.Errorf("failed to close temporary file: %w", closeErr)
		}
		if renamed == 0 {
			_ = os.Remove(tmpname)
			return fmt.Errorf("topic %q was not found", fromTopic)
		}
		if err := os.Rename(tmpname, inputPath); err != nil {
			_ = os.Remove(tmpname)
			return fmt.Errorf("failed to replace input file: %w", err)
		}
		return nil
	}

	output, err := os.Create(targetPath)
	if err != nil {
		return fmt.Errorf("failed to create output file: %w", err)
	}
	renamed, rewriteErr := rewriteMCAPTopics(output, input, fromTopic, toTopic)
	closeErr := output.Close()
	if rewriteErr != nil {
		_ = os.Remove(targetPath)
		return rewriteErr
	}
	if closeErr != nil {
		return fmt.Errorf("failed to close output file: %w", closeErr)
	}
	if renamed == 0 {
		_ = os.Remove(targetPath)
		return fmt.Errorf("topic %q was not found", fromTopic)
	}
	return nil
}

func newMcapRenameCommand() *cobra.Command {
	var fromTopic string
	var toTopic string
	var outputFile string

	renameCmd := &cobra.Command{
		Use:   "rename [INPUT_FILE]",
		Short: "Rename a topic inside an MCAP file",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			if fromTopic == "" {
				return fmt.Errorf("required flag \"from\" not set")
			}
			if toTopic == "" {
				return fmt.Errorf("required flag \"to\" not set")
			}
			if fromTopic == toTopic {
				return fmt.Errorf("--from and --to must be different")
			}
			return renameMCAPFile(args[0], outputFile, fromTopic, toTopic)
		},
	}
	renameCmd.Flags().StringVar(&fromTopic, "from", "", "Existing topic name to rename")
	renameCmd.Flags().StringVar(&toTopic, "to", "", "New topic name")
	renameCmd.Flags().StringVarP(&outputFile, "output", "o", "", "Write renamed MCAP to a new file (default: in-place)")
	return renameCmd
}
