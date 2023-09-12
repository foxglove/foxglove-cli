package cmd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	roslib "github.com/foxglove/foxglove-cli/foxglove/util/ros"
	"github.com/foxglove/mcap/go/cli/mcap/utils/ros"
	"github.com/foxglove/mcap/go/mcap"
	"github.com/foxglove/mcap/go/mcap/readopts"
	"github.com/schollz/progressbar/v3"
	"github.com/spf13/cobra"
	"google.golang.org/protobuf/encoding/protojson"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protodesc"
	"google.golang.org/protobuf/reflect/protoreflect"
	"google.golang.org/protobuf/types/descriptorpb"
	"google.golang.org/protobuf/types/dynamicpb"
)

var (
	ErrRedirectStdout = errors.New("stdout unredirected")
	ErrInvalidFormat  = errors.New("invalid format: supply mcap0, bag1, or json")
)

type DecimalTime uint64

func digits(n uint64) int {
	if n == 0 {
		return 1
	}
	count := 0
	for n != 0 {
		n = n / 10
		count++
	}
	return count
}

func (t DecimalTime) MarshalJSON() ([]byte, error) {
	seconds := uint64(t) / 1e9
	nanoseconds := uint64(t) % 1e9
	requiredLength := digits(seconds) + 1 + 9
	buf := make([]byte, 0, requiredLength)
	buf = strconv.AppendInt(buf, int64(seconds), 10)
	buf = append(buf, '.')
	for i := 0; i < 9-digits(nanoseconds); i++ {
		buf = append(buf, '0')
	}
	buf = strconv.AppendInt(buf, int64(nanoseconds), 10)
	return buf, nil
}

func (t *DecimalTime) UnmarshalJSON(b []byte) error {
	parts := bytes.Split(b, []byte{'.'})
	secs, err := strconv.ParseInt(string(parts[0]), 10, 32)
	if err != nil {
		return fmt.Errorf("failed to parse seconds: %w", err)
	}
	nsecs, err := strconv.ParseInt(string(parts[1]), 10, 32)
	if err != nil {
		return fmt.Errorf("failed to parse nanoseconds: %w", err)
	}
	*t = DecimalTime(secs*1e9 + nsecs)
	return nil
}

type Message struct {
	Topic       string          `json:"topic"`
	Sequence    uint32          `json:"sequence"`
	LogTime     DecimalTime     `json:"log_time"`
	PublishTime DecimalTime     `json:"publish_time"`
	Data        json.RawMessage `json:"data"`
}

func mcap2JSON(
	w io.Writer,
	r io.Reader,
) error {
	msg := &bytes.Buffer{}
	msgReader := &bytes.Reader{}
	buf := make([]byte, 1024*1024)
	transcoders := make(map[uint16]*ros.JSONTranscoder)
	descriptors := make(map[uint16]protoreflect.MessageDescriptor)
	encoder := json.NewEncoder(w)
	reader, err := mcap.NewReader(r)
	if err != nil {
		return fmt.Errorf("failed to create reader: %w", err)
	}
	it, err := reader.Messages(readopts.UsingIndex(false))
	if err != nil {
		return fmt.Errorf("failed to build reader: %w", err)
	}
	target := Message{}
	for {
		schema, channel, message, err := it.Next(buf)
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			log.Fatalf("Failed to read next message: %s", err)
		}
		switch schema.Encoding {
		case "ros1msg":
			transcoder, ok := transcoders[channel.SchemaID]
			if !ok {
				packageName := strings.Split(schema.Name, "/")[0]
				transcoder, err = ros.NewJSONTranscoder(packageName, schema.Data)
				if err != nil {
					return fmt.Errorf("failed to build transcoder for %s: %w", channel.Topic, err)
				}
				transcoders[channel.SchemaID] = transcoder
			}
			msgReader.Reset(message.Data)
			err = transcoder.Transcode(msg, msgReader)
			if err != nil {
				return fmt.Errorf("failed to transcode %s record on %s: %w", schema.Name, channel.Topic, err)
			}
		case "protobuf":
			messageDescriptor, ok := descriptors[channel.SchemaID]
			if !ok {
				fileDescriptorSet := &descriptorpb.FileDescriptorSet{}
				if err := proto.Unmarshal(schema.Data, fileDescriptorSet); err != nil {
					return fmt.Errorf("failed to build file descriptor set: %w", err)
				}
				files, err := protodesc.FileOptions{}.NewFiles(fileDescriptorSet)
				if err != nil {
					return fmt.Errorf("failed to create file descriptor: %w", err)
				}
				descriptor, err := files.FindDescriptorByName(protoreflect.FullName(schema.Name))
				if err != nil {
					return fmt.Errorf("failed to find descriptor: %w", err)
				}
				messageDescriptor = descriptor.(protoreflect.MessageDescriptor)
				descriptors[channel.SchemaID] = messageDescriptor
			}
			protoMsg := dynamicpb.NewMessage(messageDescriptor)
			if err := proto.Unmarshal(message.Data, protoMsg); err != nil {
				return fmt.Errorf("failed to parse message: %w", err)
			}
			bytes, err := protojson.Marshal(protoMsg)
			if err != nil {
				return fmt.Errorf("failed to marshal message: %w", err)
			}
			if _, err = msg.Write(bytes); err != nil {
				return fmt.Errorf("failed to write message bytes: %w", err)
			}
		default:
			return fmt.Errorf("JSON output only supported for ros1msg and protobuf schemas")
		}
		target.Topic = channel.Topic
		target.Sequence = message.Sequence
		target.LogTime = DecimalTime(message.LogTime)
		target.PublishTime = DecimalTime(message.PublishTime)
		target.Data = msg.Bytes()
		err = encoder.Encode(target)
		if err != nil {
			return fmt.Errorf("failed to write encoded message")
		}
		msg.Reset()
	}
	return nil
}

func stdoutRedirected() bool {
	if fi, _ := os.Stdout.Stat(); (fi.Mode() & os.ModeCharDevice) == os.ModeCharDevice {
		return false
	}
	return true
}

func validOutputFormat(format string) bool {
	return map[string]bool{
		"mcap0": true,
		"bag1":  true,
		"json":  true,
	}[format]
}

type fileInfo struct {
	maxTime      uint64
	messageCount uint64
}

// reindexBagFile rewrites a bag file to a new output location, and properly
// closes it. If the input is corrupt, we simply close the output with what was
// successfully read.
func reindexBagFile(w io.Writer, r io.Reader) error {
	writer, err := roslib.NewBagWriter(w)
	if err != nil {
		return fmt.Errorf("failed to construct bag writer: %w", err)
	}
	lexer, err := roslib.NewBagLexer(r)
	if err != nil {
		return fmt.Errorf("failed to construct bag lexer: %w", err)
	}
	for {
		tokenType, token, err := lexer.Next()
		if err != nil {
			// We'll hit an EOF at the end of a "complete" bag export, so only
			// log if we get another error (e.g UnexpectedEOF).
			if !errors.Is(err, io.EOF) {
				debugf("partial bag export corrupt with error %s", err)
			}
			// No matter what we need to close the output writer so the indexes
			// are correctly written. Non-EOF errors are permissable above,
			// since we are explicitly parsing a corrupt file.
			return writer.Close()
		}
		switch tokenType {
		case roslib.OpConnection:
			connection, err := roslib.ParseConnection(token)
			if err != nil {
				return err
			}
			if err = writer.WriteConnection(connection); err != nil {
				return err
			}
		case roslib.OpMessageData:
			message, err := roslib.ParseMessage(token)
			if err != nil {
				return err
			}
			if err = writer.WriteMessage(message); err != nil {
				return err
			}
		}
	}
}

// reindexMCAPFile rewrites an MCAP file to a new output location, and properly
// closes it. If the input is corrupt, we simply close the output with what was
// successfully read.
func reindexMCAPFile(w io.Writer, r io.Reader) error {
	writer, err := mcap.NewWriter(w, &mcap.WriterOptions{
		Chunked:     true,
		ChunkSize:   1024 * 1024,
		Compression: mcap.CompressionLZ4,
	})
	if err != nil {
		return err
	}
	lexer, err := mcap.NewLexer(r, &mcap.LexerOptions{
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
		return err
	}
	for {
		tokenType, token, err := lexer.Next(nil)
		if err != nil {
			// if the lexer hits an error because the download was truncated, we
			// still attempt to close the writer and work with what we have.
			debugf("partial export is corrupt with error: %s. Proceeding.", err)
			return writer.Close()
		}
		switch tokenType {
		case mcap.TokenHeader:
			header, err := mcap.ParseHeader(token)
			if err != nil {
				return err
			}
			if err = writer.WriteHeader(header); err != nil {
				return err
			}
		case mcap.TokenSchema:
			schema, err := mcap.ParseSchema(token)
			if err != nil {
				return err
			}
			if err = writer.WriteSchema(schema); err != nil {
				return err
			}
		case mcap.TokenChannel:
			channel, err := mcap.ParseChannel(token)
			if err != nil {
				return err
			}
			if err = writer.WriteChannel(channel); err != nil {
				return err
			}
		case mcap.TokenMessage:
			msg, err := mcap.ParseMessage(token)
			if err != nil {
				return err
			}
			if err = writer.WriteMessage(msg); err != nil {
				return err
			}
		case mcap.TokenMetadata:
			metadata, err := mcap.ParseMetadata(token)
			if err != nil {
				return err
			}
			if err = writer.WriteMetadata(metadata); err != nil {
				return err
			}
		case mcap.TokenDataEnd:
			return writer.Close()
		}
	}
}

// reindex a file, staging the reindexed output in tmpdir prior to moving it to
// the final location (same as the input location) atomically.
func reindex(tmpdir string, filename string, format string) (bool, *fileInfo, error) {
	f, err := os.Open(filename)
	if err != nil {
		return false, nil, err
	}
	defer f.Close()
	switch format {
	case "bag1":
		reader, err := roslib.NewBagReader(f)
		if err != nil {
			return false, nil, err
		}
		info, err := reader.Info()
		if err == nil {
			debugf("file already indexed. Message end time: %d", info.MessageEndTime)
			// already indexed
			return false, &fileInfo{
				maxTime:      info.MessageEndTime,
				messageCount: info.MessageCount,
			}, nil
		}
		tmpfile, err := os.CreateTemp(tmpdir, "reindex")
		if err != nil {
			return false, nil, fmt.Errorf("failed to create temporary reindex target: %w", err)
		}
		_, err = f.Seek(0, 0)
		if err != nil {
			return false, nil, fmt.Errorf("failed to seek to beginning of file: %w", err)
		}
		err = reindexBagFile(tmpfile, f)
		if err != nil {
			return false, nil, fmt.Errorf("failed to reindex: %w", err)
		}
		err = tmpfile.Close()
		if err != nil {
			return false, nil, fmt.Errorf("failed to close tempfile: %w", err)
		}
		err = os.Rename(tmpfile.Name(), filename)
		if err != nil {
			return false, nil, fmt.Errorf("failed to rename reindexed file: %w", err)
		}
		if err = f.Close(); err != nil {
			return false, nil, fmt.Errorf("failed to close file: %w", err)
		}
		f, err := os.Open(filename)
		if err != nil {
			return false, nil, err
		}
		reader, err = roslib.NewBagReader(f)
		if err != nil {
			return false, nil, err
		}
		// now grab the info
		info, err = reader.Info()
		if err != nil {
			return false, nil, fmt.Errorf("failed to read file info: %w", err)
		}
		return true, &fileInfo{
			maxTime:      info.MessageEndTime,
			messageCount: info.MessageCount,
		}, nil
	case "mcap0":
		reader, err := mcap.NewReader(f)
		if err != nil {
			return false, nil, err
		}
		info, err := reader.Info()
		if err == nil {
			// already indexed
			return false, &fileInfo{
				maxTime:      info.Statistics.MessageEndTime,
				messageCount: info.Statistics.MessageCount,
			}, nil
		}
		_, err = f.Seek(0, 0)
		if err != nil {
			return false, nil, fmt.Errorf("failed to seek to file start: %w", err)
		}
		tmpfile, err := os.CreateTemp(tmpdir, "reindex")
		if err != nil {
			return false, nil, fmt.Errorf("failed to create temporary reindex target: %w", err)
		}
		err = reindexMCAPFile(tmpfile, f)
		if err != nil {
			return false, nil, fmt.Errorf("failed to reindex: %w", err)
		}
		err = os.Rename(tmpfile.Name(), filename)
		if err != nil {
			return false, nil, fmt.Errorf("failed to rename reindexed file: %w", err)
		}

		reader.Close()
		if err = f.Close(); err != nil {
			return false, nil, fmt.Errorf("failed to close file: %w", err)
		}
		f, err := os.Open(filename)
		if err != nil {
			return false, nil, err
		}
		reader, err = mcap.NewReader(f)
		if err != nil {
			return false, nil, err
		}

		// now grab the info
		info, err = reader.Info()
		if err != nil {
			return false, nil, fmt.Errorf("failed to read file info: %w", err)
		}
		return true, &fileInfo{
			maxTime:      info.Statistics.MessageEndTime,
			messageCount: info.Statistics.MessageCount,
		}, nil
	default:
		return false, nil, fmt.Errorf("unrecognized format: %s", format)
	}
}

type partialFile struct {
	name string
	rs   io.ReadSeeker
	info *fileInfo
}

func doExport(
	ctx context.Context,
	outputfile string,
	baseURL string,
	clientID string,
	bearerToken string,
	userAgent string,
	request *console.StreamRequest,
) error {
	tmpdir, err := os.MkdirTemp(".", "export")
	if err != nil {
		return fmt.Errorf("failed to create temporary output directory: %w", err)
	}
	defer os.RemoveAll(tmpdir)
	zeroMessageDownloadCount := 0
	repeatRequestCount := 0
	tmpfiles := []partialFile{}
	for {
		tmpfile, err := os.CreateTemp(tmpdir, "export")
		if err != nil {
			return err
		}
		defer tmpfile.Close()
		debugf("exporting to %s", tmpfile.Name())
		err = executeExport(ctx, tmpfile, baseURL, clientID, bearerToken, userAgent, request)
		if err != nil {
			fmt.Println("error executing export: ", err)
		}
		didReindex, info, err := reindex(tmpdir, tmpfile.Name(), request.OutputFormat)
		if err != nil {
			return fmt.Errorf("failed to reindex tmpfile %s: %w", tmpfile.Name(), err)
		}
		debugf("output %s was complete: %t. Message count %d. Max time %d", tmpfile.Name(), !didReindex, info.messageCount, info.maxTime)

		// add tmpfile name to structure, with the biggest timestamp to scan _through_
		tmpfiles = append(tmpfiles, partialFile{tmpfile.Name(), tmpfile, info})
		if !didReindex {
			// if we did not need to do any reindexing, the file was already
			// complete. That means quit looping. This can only happen on an
			// MCAP export, since MCAP files include the closing magic as an
			// indicator that the file was closed. Since bag files could get
			// truncated on a message boundary, we have no way of distinguishing
			// a complete file from a truncated one. For bags, we always need to
			// make a followup request.
			break
		}

		// If we reindexed but the message count we got is zero, bail here as well.
		// Since bags don't contain closing magic, there is no way of
		// distinguishing a legitimately empty file from one that is truncated
		// with no records. To account for this, we will bail if we get two
		// successive results with zero messages included.
		if info.messageCount == 0 {
			zeroMessageDownloadCount++
			if zeroMessageDownloadCount > 1 {
				debugf("got two successive empty downloads. Assuming EOF.")
				break
			}

			// if the message count for the last export is zero, we need to redo
			// it with the same parameters.
			continue
		}
		// otherwise, we need to do another request, starting at the end of the
		// just-completed fetch. The merging process will deal with the overlaps.

		// otherwise, do another request. Since we're here we got a nonempty
		// resultset, so set the empty download count back to zero.
		zeroMessageDownloadCount = 0

		// the start time of the new request will be the max time from the
		// request just received. Specifically, the timestamp of the last
		// message written to the output file. By starting the request with this
		// time, we will end up with some messages duplicated between the end of
		// the previous file and the start of the next one. The merging process
		// at the end will be in charge of accounting for these duplicates.
		newStart := time.Unix(int64(info.maxTime)/1e9, int64(info.maxTime)%1e9)

		if request.Start != nil && newStart == *request.Start {
			repeatRequestCount++
		} else {
			repeatRequestCount = 0
		}

		// It is possible that the previous request contained a set of messagges
		// on the same timestamp, right at the end of the response. In this
		// instance, the next start time is the same as the previous start time.
		// We can end up in a loop repeatedly writing the same messages out to
		// partial files, and rerequesting the same data. To detect this we bail
		// if we have requested the same data more than twice.
		if repeatRequestCount > 1 {
			debugf("got two successive requests with the same start time. Assuming EOF.")
			break
		}

		request.Start = &newStart
		if request.End == nil {
			end := time.Now()
			request.End = &end
		}
	}

	// Now we need to combine the messages from the tmpfiles, handling the
	// overlaps between them.

	// If we have just one file, execute a mv. This will be the typical case
	// when there is no failure.
	if len(tmpfiles) == 1 {
		debugf("single tmpfile - executing a rename")
		err := os.Rename(tmpfiles[0].name, outputfile)
		if err != nil {
			return fmt.Errorf("failed to rename tmpfile: %w", err)
		}
		return nil
	}
	// otherwise go through the files in order and write out the messages
	debugf("multiple tmpfiles - combining")
	output, err := os.Create(outputfile)
	if err != nil {
		return err
	}
	defer output.Close()

	switch request.OutputFormat {
	case "bag1":
		return combineBagTmpFiles(output, tmpfiles)
	case "mcap0":
		return combineMCAPTmpFiles(output, tmpfiles)
	default:
		return fmt.Errorf("unsupported format for resilient download: %s", request.OutputFormat)
	}
}

func combineBagTmpFiles(w io.Writer, tmpfiles []partialFile) error {
	debugf("combining %d bag files", len(tmpfiles))
	writer, err := roslib.NewBagWriter(w)
	if err != nil {
		return fmt.Errorf("failed to construct output writer: %w", err)
	}
	connectionsWritten := make(map[uint32]bool)
	var connIDIncrement, maxObservedConn uint32
	for i, tmpfile := range tmpfiles {
		if tmpfile.info.messageCount == 0 {
			debugf("omitting empty partial file %s", tmpfile.name)
			continue
		}
		_, err := tmpfile.rs.Seek(0, 0)
		if err != nil {
			return err
		}
		lexer, err := roslib.NewBagLexer(tmpfile.rs)
		if err != nil {
			return fmt.Errorf("failed to construct lexer: %w", err)
		}
		var scanThrough uint64
		if i == len(tmpfiles)-1 {
			scanThrough = tmpfile.info.maxTime
		} else {
			if tmpfile.info.maxTime == 0 {
				continue
			}
			scanThrough = tmpfile.info.maxTime - 1
		}
		connIDIncrement = maxObservedConn + 1
		debugf("combining %s with connID increment %d and maxObservedConn %d", tmpfile.name, connIDIncrement, maxObservedConn)
	Top:
		for {
			tokenType, token, err := lexer.Next()
			if err != nil {
				if errors.Is(err, io.EOF) {
					break
				}
				return fmt.Errorf("failed to read message: %w", err)
			}
			switch tokenType {
			case roslib.OpMessageData:
				message, err := roslib.ParseMessage(token)
				if err != nil {
					return fmt.Errorf("failed to parse message: %w", err)
				}
				if message.Time > scanThrough {
					break Top
				}
				message.Conn += connIDIncrement
				err = writer.WriteMessage(message)
				if err != nil {
					return fmt.Errorf("failed to write message: %w", err)
				}
			case roslib.OpConnection:
				conn, err := roslib.ParseConnection(token)
				if err != nil {
					return fmt.Errorf("failed to parse channel: %w", err)
				}
				conn.Conn += connIDIncrement
				if conn.Conn > maxObservedConn {
					maxObservedConn = conn.Conn
				}

				// deduplicate the written-out connections, so we don't rewrite
				// the ones at EOF. These will be written by the writer close.
				// Since bag format doesn't indicate the end of the "data
				// section" we have no other way to recognize these than if they
				// had already been written out.
				if !connectionsWritten[conn.Conn] {
					err = writer.WriteConnection(conn)
					if err != nil {
						return fmt.Errorf("failed to write channel: %w", err)
					}
					connectionsWritten[conn.Conn] = true
				}

			}
		}
	}
	return writer.Close()
}

func combineMCAPTmpFiles(w io.Writer, tmpfiles []partialFile) error {
	writer, err := mcap.NewWriter(w, &mcap.WriterOptions{
		Chunked:     true,
		ChunkSize:   4 * 1024 * 1024,
		Compression: mcap.CompressionLZ4,
	})
	if err != nil {
		return fmt.Errorf("failed to construct output writer: %w", err)
	}

	if err := writer.WriteHeader(&mcap.Header{}); err != nil {
		return fmt.Errorf("failed to write output header: %w", err)
	}

	var schemaIDIncrement, channelIDIncrement, maxObservedSchema, maxObservedChannel uint16
	for i, tmpfile := range tmpfiles {
		if tmpfile.info.messageCount == 0 {
			debugf("omitting empty partial file %s", tmpfile.name)
			continue
		}
		_, err := tmpfile.rs.Seek(0, 0)
		if err != nil {
			return err
		}
		lexer, err := mcap.NewLexer(tmpfile.rs, &mcap.LexerOptions{
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
			return fmt.Errorf("failed to construct lexer: %w", err)
		}
		var scanThrough uint64
		if i == len(tmpfiles)-1 {
			scanThrough = tmpfile.info.maxTime
		} else {
			if tmpfile.info.maxTime == 0 {
				continue
			}
			scanThrough = tmpfile.info.maxTime - 1
		}
		schemaIDIncrement = maxObservedSchema
		channelIDIncrement = maxObservedChannel + 1
	Top:
		for {
			tokenType, token, err := lexer.Next(nil)
			if err != nil {
				return fmt.Errorf("failed to read message: %w", err)
			}
			switch tokenType {
			case mcap.TokenMessage:
				message, err := mcap.ParseMessage(token)
				if err != nil {
					return fmt.Errorf("failed to parse message: %w", err)
				}
				if message.LogTime > scanThrough {
					break Top
				}
				message.ChannelID += channelIDIncrement
				err = writer.WriteMessage(message)
				if err != nil {
					return fmt.Errorf("failed to write message: %w", err)
				}
			case mcap.TokenChannel:
				channel, err := mcap.ParseChannel(token)
				if err != nil {
					return fmt.Errorf("failed to parse channel: %w", err)
				}
				channel.ID += channelIDIncrement
				channel.SchemaID += schemaIDIncrement
				if channel.ID > maxObservedChannel {
					maxObservedChannel = channel.ID
				}
				err = writer.WriteChannel(channel)
				if err != nil {
					return fmt.Errorf("failed to write channel: %w", err)
				}
			case mcap.TokenSchema:
				schema, err := mcap.ParseSchema(token)
				if err != nil {
					return fmt.Errorf("failed to parse schema: %w", err)
				}
				schema.ID += schemaIDIncrement
				if schema.ID > maxObservedSchema {
					maxObservedSchema = schema.ID
				}
				err = writer.WriteSchema(schema)
				if err != nil {
					return fmt.Errorf("failed to write schema: %w", err)
				}
			case mcap.TokenMetadata:
				metadata, err := mcap.ParseMetadata(token)
				if err != nil {
					return fmt.Errorf("failed to parse metadata: %w", err)
				}
				err = writer.WriteMetadata(metadata)
				if err != nil {
					return fmt.Errorf("failed to write metadata: %w", err)
				}
			case mcap.TokenDataEnd:
				break Top
			}
		}
	}
	return writer.Close()
}

func executeExport(
	ctx context.Context,
	w io.Writer,
	baseURL string,
	clientID string,
	bearerToken string,
	userAgent string,
	request *console.StreamRequest,
) error {
	debugf("exporting with request: %+v", request)
	if !validOutputFormat(request.OutputFormat) {
		return ErrInvalidFormat
	}
	client := console.NewRemoteFoxgloveClient(
		baseURL,
		clientID,
		bearerToken,
		userAgent,
	)
	writer := w
	if stdoutRedirected() {
		progressWriter := progressbar.DefaultBytes(-1, "exporting")
		writer = io.MultiWriter(w, progressWriter)
	}
	if request.OutputFormat == "json" {
		request.OutputFormat = "mcap0"
		pipeReader, pipeWriter, err := os.Pipe()
		if err != nil {
			return fmt.Errorf("failed to create pipe: %w", err)
		}
		errs := make(chan error, 1)
		done := make(chan bool, 1)
		go func() {
			err = console.Export(ctx, pipeWriter, client, request)
			if err != nil {
				errs <- err
				return
			}
			done <- true
			pipeWriter.Close()
		}()
		err = mcap2JSON(writer, pipeReader)
		if err != nil {
			return fmt.Errorf("JSON conversion error: %w", err)
		}
		select {
		case <-done:
			return nil
		case err := <-errs:
			return err
		}
	} else {
		return console.Export(ctx, writer, client, request)
	}
}

func createStreamRequest(
	recordingID string,
	importID string,
	deviceID string,
	deviceName string,
	start string,
	end string,
	outputFormat string,
	topicList string,
) (*console.StreamRequest, error) {
	var startTime, endTime *time.Time
	if start != "" {
		start, err := time.Parse(time.RFC3339, start)
		if err != nil {
			return nil, fmt.Errorf("failed to parse start time: %w", err)
		}
		startTime = &start
	}

	if end != "" {
		end, err := time.Parse(time.RFC3339, end)
		if err != nil {
			return nil, fmt.Errorf("failed to parse end time: %w", err)
		}
		endTime = &end
	}

	topics := strings.FieldsFunc(topicList, func(c rune) bool { return c == ',' })

	request := &console.StreamRequest{
		RecordingID:  recordingID,
		ImportID:     importID,
		DeviceName:   deviceName,
		DeviceID:     deviceID,
		Start:        startTime,
		End:          endTime,
		OutputFormat: outputFormat,
		Topics:       topics,
	}
	if err := request.Validate(); err != nil {
		return nil, err
	}

	return request, nil
}

func newExportCommand(params *baseParams) (*cobra.Command, error) {
	var recordingID string
	var importID string
	var deviceID string
	var deviceName string
	var start string
	var end string
	var outputFormat string
	var topicList string
	var outputFile string
	var isJsonOutput bool
	exportCmd := &cobra.Command{
		Use:   "export",
		Short: "Export a data selection from Foxglove Data Platform",
		Long:  "Export a data selection from Foxglove Data Platform by Recording ID, Import ID, or Device and time range",
		Run: func(cmd *cobra.Command, args []string) {
			startTime, err := maybeConvertToRFC3339(start)
			if err != nil {
				dief("failed to parse start time: %s", err)
			}
			endTime, err := maybeConvertToRFC3339(end)
			if err != nil {
				dief("failed to parse end time: %s", err)
			}
			request, err := createStreamRequest(
				recordingID,
				importID,
				deviceID,
				deviceName,
				startTime,
				endTime,
				outputFormat,
				topicList,
			)
			if err != nil {
				dief("Failed to build request: %s\n", err)
			}
			if isJsonOutput && outputFormat != "json" {
				dief("Export failed. Output format conflict: --json, --output-format ", outputFormat)
			}
			if outputFile != "" && outputFormat != "json" {
				err = doExport(
					cmd.Context(),
					outputFile,
					params.baseURL,
					*params.clientID,
					params.token,
					params.userAgent,
					request,
				)
				if err != nil {
					dief("Export failed: %s\n", err)
				}
				fmt.Fprint(os.Stderr, "\n")
				return
			}
			if !stdoutRedirected() && request.OutputFormat != "json" {
				dief("Binary output may screw up your terminal. Please redirect to a pipe or file.\n")
			}
			defer os.Stdout.Close()
			err = executeExport(
				cmd.Context(),
				os.Stdout,
				params.baseURL,
				*params.clientID,
				params.token,
				params.userAgent,
				request,
			)
			if err != nil {
				dief("Export failed: %s\n", err)
			}
		},
	}
	exportCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	exportCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "device name")
	exportCmd.PersistentFlags().StringVarP(&outputFile, "output-file", "o", "", "output file")
	exportCmd.PersistentFlags().StringVarP(&recordingID, "recording-id", "", "", "recording ID")
	exportCmd.PersistentFlags().StringVarP(&importID, "import-id", "", "", "import ID")
	exportCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start time (ISO8601 timestamp)")
	exportCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end time (ISO8601 timestamp")
	exportCmd.PersistentFlags().StringVarP(&outputFormat, "output-format", "", "mcap0", "output format (mcap0, bag1, or json)")
	exportCmd.PersistentFlags().StringVarP(&topicList, "topics", "", "", "comma separated list of topics")
	exportCmd.PersistentFlags().BoolVar(&isJsonOutput, "json", false, "alias for --output-format json")
	AddDeviceAutocompletion(exportCmd, params)
	return exportCmd, nil
}
