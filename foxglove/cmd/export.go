package cmd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"math"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/foxglove/mcap/go/cli/mcap/utils/ros"
	"github.com/foxglove/mcap/go/mcap"
	"github.com/schollz/progressbar/v3"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
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
	it, err := reader.Messages(0, math.MaxInt64, nil, false)
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

func executeExport(
	ctx context.Context,
	w io.Writer,
	baseURL string,
	clientID string,
	bearerToken string,
	userAgent string,
	request *console.StreamRequest,
) error {
	if !validOutputFormat(request.OutputFormat) {
		return ErrInvalidFormat
	}
	client := console.NewRemoteFoxgloveClient(
		baseURL,
		clientID,
		bearerToken,
		userAgent,
	)
	if !stdoutRedirected() && request.OutputFormat != "json" {
		return fmt.Errorf("Binary output may screw up your terminal. Please redirect to a pipe or file.\n")
	}
	progressWriter := progressbar.DefaultBytes(-1, "exporting")
	writer := io.MultiWriter(w, progressWriter)
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
	importID string,
	deviceID string,
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
		ImportID:     importID,
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
	var importID string
	var deviceID string
	var start string
	var end string
	var outputFormat string
	var topicList string
	exportCmd := &cobra.Command{
		Use:   "export",
		Short: "Export a data selection from foxglove data platform",
		Run: func(cmd *cobra.Command, args []string) {
			defer os.Stdout.Close()

			request, err := createStreamRequest(importID, deviceID, start, end, outputFormat, topicList)
			if err != nil {
				fatalf("Failed to build request: %s\n", err)
			}

			err = executeExport(
				cmd.Context(),
				os.Stdout,
				*params.baseURL,
				*params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
				request,
			)
			if err != nil {
				fatalf("Export failed: %s\n", err)
			}
		},
	}
	exportCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "device ID")
	exportCmd.PersistentFlags().StringVarP(&importID, "import-id", "", "", "import ID")
	exportCmd.PersistentFlags().StringVarP(&start, "start", "", "", "start time (RFC3339 timestamp)")
	exportCmd.PersistentFlags().StringVarP(&end, "end", "", "", "end time (RFC3339 timestamp")
	exportCmd.PersistentFlags().StringVarP(&outputFormat, "output-format", "", "mcap0", "output format (mcap0, bag1, or json)")
	exportCmd.PersistentFlags().StringVarP(&topicList, "topics", "", "", "comma separated list of topics")
	err := exportCmd.RegisterFlagCompletionFunc(
		"device-id",
		listDevicesAutocompletionFunc(
			*params.baseURL,
			*params.clientID,
			viper.GetString("bearer_token"),
			params.userAgent,
		),
	)
	if err != nil {
		return nil, err
	}
	return exportCmd, nil
}
