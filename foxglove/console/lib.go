package console

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"runtime"
	"time"

	"github.com/foxglove/mcap/go/mcap"
	"github.com/schollz/progressbar/v3"
)

const (
	tokenRetryInterval = 500 * time.Millisecond
)

type AuthDelegate interface {
	openBrowser(url string) (*exec.Cmd, error)
}

type PlatformAuthDelegate struct{}

func (_ *PlatformAuthDelegate) openBrowser(url string) (*exec.Cmd, error) {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "linux":
		cmd = exec.Command("xdg-open", url)
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", url)
	case "darwin":
		cmd = exec.Command("open", url)
	default:
		return nil, fmt.Errorf("unsupported platform")
	}
	return cmd, cmd.Start()
}

func Export(
	ctx context.Context,
	w io.Writer,
	client *FoxgloveClient,
	request *StreamRequest,
) error {
	writer, err := mcap.NewWriter(w, &mcap.WriterOptions{
		IncludeCRC:  true,
		Chunked:     true,
		Compression: mcap.CompressionZSTD,
		ChunkSize:   4 * 1024 * 1024,
	})
	if err != nil {
		return err
	}

	buf := make([]byte, 0)
	lastMessageTimestamp := uint64(0)
	bufferedMessages := []*mcap.Message{}
Top:
	for {
		rc, err := client.Stream(request)
		if err != nil {
			return err
		}
		defer rc.Close()

		lexer, err := mcap.NewLexer(rc, &mcap.LexerOptions{})
		if err != nil {
			return err
		}

		for {
			tokenType, token, err := lexer.Next(buf)
			if err != nil {
				request = &StreamRequest{
					ImportID:     request.ImportID,
					DeviceID:     request.DeviceID,
					Start:        &time.Time{}, // start at the last serialized
					End:          request.End,
					OutputFormat: request.OutputFormat,
					Topics:       request.Topics,
				}
				return err
			}
			if len(buf) < len(token) {
				buf = token
			}
			switch tokenType {
			case mcap.TokenChannel:
				channel, err := mcap.ParseChannel(token)
				if err != nil {
					return err
				}
				err = writer.WriteChannel(channel)
				if err != nil {
					return err
				}
			case mcap.TokenMessage:
				message, err := mcap.ParseMessage(token)
				if err != nil {
					return err
				}

				if message.LogTime == lastMessageTimestamp {
					bufferedMessages = append(bufferedMessages, message)
				}

				err = writer.WriteMessage(message)
				if err != nil {
					return err
				}
			case mcap.TokenSchema:
				schema, err := mcap.ParseSchema(token)
				if err != nil {
					return err
				}
				err = writer.WriteSchema(schema)
				if err != nil {
					return err
				}
			case mcap.TokenMetadata:
				metadata, err := mcap.ParseMetadata(token)
				if err != nil {
					return err
				}
				err = writer.WriteMetadata(metadata)
				if err != nil {
					return err
				}
			case mcap.TokenDataEnd:
				break Top
			default:
				continue
			}
		}
	}
	return nil
}

func Import(
	ctx context.Context,
	client *FoxgloveClient,
	deviceID string,
	filename string,
) error {
	f, err := os.Open(filename)
	if err != nil {
		return fmt.Errorf("failed to open input file: %w", err)
	}
	defer f.Close()
	stat, err := f.Stat()
	if err != nil {
		return fmt.Errorf("failed to stat input: %w", err)
	}
	_, name := path.Split(filename)
	bar := progressbar.DefaultBytes(stat.Size(), "uploading")
	defer bar.Close()
	reader := progressbar.NewReader(f, bar)
	err = client.Upload(&reader, UploadRequest{
		Filename: name,
		DeviceID: deviceID,
	})
	if err != nil {
		return err
	}
	return nil
}

func UploadExtensionFile(
	ctx context.Context,
	client *FoxgloveClient,
	filename string,
) error {
	f, err := os.Open(filename)
	if err != nil {
		return fmt.Errorf("failed to open input file: %w", err)
	}
	defer f.Close()
	stat, err := f.Stat()
	if err != nil {
		return fmt.Errorf("failed to stat input: %w", err)
	}

	if filepath.Ext(stat.Name()) != ".foxe" {
		return fmt.Errorf("file should have a '.foxe' extension")
	}

	if stat.Size() > 30*1024*1024 {
		return fmt.Errorf("file size may not exceed 30mb")
	}

	bar := progressbar.DefaultBytes(stat.Size(), "uploading")
	defer bar.Close()

	if err != nil {
		return fmt.Errorf("cannot upload extension: %w", err)
	}
	reader := progressbar.NewReader(f, bar)
	return client.UploadExtension(&reader)
}

// Login initializes a browser-based login flow for foxglove studio.
func Login(ctx context.Context, client *FoxgloveClient, authDelegate AuthDelegate) (string, error) {
	info, err := client.DeviceCode()
	if err != nil {
		return "", fmt.Errorf("failed to fetch device code: %w", err)
	}
	browser, err := authDelegate.openBrowser(info.VerificationUriComplete)
	// There's no way to tell for sure whether the browser actually opened the link, even if the
	// openBrowser command succeeds.
	if err == nil {
		defer func() {
			_ = browser.Process.Kill()
		}()
		fmt.Println("If no window opens, copy/paste the following link into your browser:")
	} else {
		fmt.Println("copy/paste the following link into your browser:")
	}
	fmt.Println("")
	fmt.Println(info.VerificationUriComplete)
	fmt.Println("")
	fmt.Println("Verify this code and click 'Authorize' to complete login: ", info.UserCode)

	// now poll the token endpoint until the token for the device code appears.
	// When the device code has not yet appeared, the endpoint returns a 403.
	var token string
	for {

		select {
		case <-ctx.Done():
			return "", context.Canceled
		default:
		}

		token, err = client.Token(info.DeviceCode)
		if errors.Is(err, ErrForbidden) {
			time.Sleep(tokenRetryInterval)
			continue
		}
		if err != nil {
			return "", fmt.Errorf("failed to request token: %w", err)
		}
		break
	}
	bearerToken, err := client.SignIn(token)
	if err != nil {
		return "", fmt.Errorf("failed to sign in: %w", err)
	}
	return bearerToken, nil
}
