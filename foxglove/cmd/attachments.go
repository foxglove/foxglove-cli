package cmd

import (
	"fmt"
	"io"
	"os"

	"github.com/foxglove/foxglove-cli/foxglove/console"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListAttachmentsCommand(params *baseParams) *cobra.Command {
	var format string
	var importID string
	var recordingID string
	attachmentsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List MCAP attachments",
		Run: func(cmd *cobra.Command, args []string) {
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			err := renderList(
				os.Stdout,
				&console.AttachmentsRequest{
					ImportID:    importID,
					RecordingID: recordingID,
				},
				client.Attachments,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list imports: %s\n", err)
				os.Exit(1)
			}
		},
	}
	attachmentsListCmd.InheritedFlags()
	attachmentsListCmd.PersistentFlags().StringVarP(&importID, "import-id", "", "", "Import ID")
	attachmentsListCmd.PersistentFlags().StringVarP(&recordingID, "recording-id", "", "", "Recording ID")
	AddFormatFlag(attachmentsListCmd, &format)
	return attachmentsListCmd
}

func newDownloadAttachmentCmd(params *baseParams) *cobra.Command {
	attachmentsDownloadCmd := &cobra.Command{
		Use:   "download",
		Short: "Download an MCAP attachment by ID",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			attachmentID := args[0]
			client := console.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				viper.GetString("bearer_token"),
				params.userAgent,
			)
			rc, err := client.Attachment(attachmentID)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to fetch attachment: %s\n", err)
				os.Exit(1)
			}
			defer rc.Close()
			_, err = io.Copy(os.Stdout, rc)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to fetch attachment: %s\n", err)
				os.Exit(1)
			}
		},
	}
	attachmentsDownloadCmd.InheritedFlags()
	return attachmentsDownloadCmd
}
