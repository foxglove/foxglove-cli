package svc

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"runtime"
	"time"

	"github.com/schollz/progressbar/v3"
)

const (
	tokenRetryInterval = 500 * time.Millisecond
)

func Export(
	ctx context.Context,
	w io.Writer,
	client *foxgloveClient,
	deviceID string,
	startstr string,
	endstr string,
	topics []string,
	outputFormat string,
) error {
	start, err := time.Parse(time.RFC3339, startstr)
	if err != nil {
		return fmt.Errorf("failed to parse start: %w", err)
	}
	end, err := time.Parse(time.RFC3339, endstr)
	if err != nil {
		return fmt.Errorf("failed to parse start: %w", err)
	}
	rc, err := client.Stream(StreamRequest{
		DeviceID:     deviceID,
		Start:        start,
		End:          end,
		OutputFormat: outputFormat,
		Topics:       topics,
	})
	if err != nil {
		return fmt.Errorf("streaming request failure: %w", err)
	}
	defer rc.Close()
	_, err = io.Copy(w, rc)
	if err != nil {
		return fmt.Errorf("copy failure: %w", err)
	}
	return nil
}

func Import(
	ctx context.Context,
	client *foxgloveClient,
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
		return fmt.Errorf("upload failure: %w", err)
	}
	return nil
}

// Login initializes a browser-based login flow for foxglove studio.
func Login(ctx context.Context, client *foxgloveClient) (string, error) {
	info, err := client.DeviceCode()
	if err != nil {
		return "", fmt.Errorf("failed to fetch device code: %w", err)
	}
	fmt.Println("Enter this code in your browser: ", info.UserCode)
	browser, err := openBrowser(info.VerificationUriComplete)
	if err != nil {
		return "", fmt.Errorf("failed to open browser: %w", err)
	}
	defer func() {
		_ = browser.Process.Kill()
	}()

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

func openBrowser(url string) (*exec.Cmd, error) {
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
