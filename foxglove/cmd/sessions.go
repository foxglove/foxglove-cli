package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func newListSessionsCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	var projectID string
	sessionsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List sessions in your organization",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				api.SessionsRequest{
					ProjectID: projectID,
				},
				client.Sessions,
				format,
			)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to list sessions: %s\n", err)
				os.Exit(1)
			}
		},
	}
	sessionsListCmd.InheritedFlags()
	sessionsListCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (optional filter)")
	AddFormatFlag(sessionsListCmd, &format)
	AddJsonFlag(sessionsListCmd, &isJsonFormat)
	return sessionsListCmd
}

func newGetSessionCommand(params *baseParams) *cobra.Command {
	var projectID string
	getSessionCmd := &cobra.Command{
		Use:   "get [session-id-or-key]",
		Short: "Get a session by ID or key",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			keyOrID := args[0]
			session, err := client.GetSession(keyOrID, projectID)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				if err == api.ErrNotFound {
					dief("Session not found: %s", keyOrID)
				}
				dief("Failed to get session: %s", err)
			}
			fmt.Printf("ID:         %s\n", session.ID)
			fmt.Printf("Name:       %s\n", session.Name)
			fmt.Printf("Key:        %s\n", session.Key)
			fmt.Printf("Project ID: %s\n", session.ProjectID)
			fmt.Printf("Created At: %s\n", session.CreatedAt.Format("2006-01-02T15:04:05Z07:00"))
			fmt.Printf("Updated At: %s\n", session.UpdatedAt.Format("2006-01-02T15:04:05Z07:00"))
			recordingIDs := session.RecordingIDs
			if len(recordingIDs) == 0 {
				recs, err := client.ListSessionRecordings(keyOrID, projectID)
				if err == nil {
					recordingIDs = recs.RecordingIDs
				}
			}
			if len(recordingIDs) > 0 {
				fmt.Printf("Recordings: %s\n", strings.Join(recordingIDs, ", "))
			} else {
				fmt.Printf("Recordings: (none)\n")
			}
		},
	}
	getSessionCmd.InheritedFlags()
	getSessionCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when session-id-or-key is a session key)")
	return getSessionCmd
}

func newAddSessionCommand(params *baseParams) *cobra.Command {
	var name string
	var projectID string
	var deviceID string
	addSessionCmd := &cobra.Command{
		Use:   "add",
		Short: "Create a session",
		Run: func(cmd *cobra.Command, args []string) {
			if deviceID == "" {
				dief("--device-id is required when creating a session")
			}
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			resp, err := client.CreateSession(api.CreateSessionRequest{
				Name:      name,
				ProjectID: projectID,
				DeviceID:  deviceID,
			})
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to create session: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Session created: %s\n", resp.ID)
			if resp.Key != "" {
				fmt.Fprintf(os.Stderr, "Session key: %s\n", resp.Key)
			}
		},
	}
	addSessionCmd.InheritedFlags()
	addSessionCmd.PersistentFlags().StringVarP(&name, "name", "", "", "Name of the session")
	addSessionCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID")
	addSessionCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID (required)")
	AddDeviceIDAutocompletion(addSessionCmd, params)
	return addSessionCmd
}

func newSessionRecordingsCommand(params *baseParams) *cobra.Command {
	recordingsCmd := &cobra.Command{
		Use:   "recordings",
		Short: "List, add, or remove recordings in a session",
	}
	recordingsCmd.AddCommand(
		newSessionRecordingsListCommand(params),
		newSessionRecordingsAddCommand(params),
		newSessionRecordingsRemoveCommand(params),
	)
	return recordingsCmd
}

func newSessionRecordingsListCommand(params *baseParams) *cobra.Command {
	var projectID string
	listCmd := &cobra.Command{
		Use:   "list [session-id-or-key]",
		Short: "List recording IDs in a session",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			keyOrID := args[0]
			recs, err := client.ListSessionRecordings(keyOrID, projectID)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to list session recordings: %s", err)
			}
			for _, id := range recs.RecordingIDs {
				fmt.Println(id)
			}
		},
	}
	listCmd.InheritedFlags()
	listCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when session-id-or-key is a session key)")
	return listCmd
}

func newSessionRecordingsAddCommand(params *baseParams) *cobra.Command {
	var projectID string
	addCmd := &cobra.Command{
		Use:   "add [session-id-or-key] [recording-id]",
		Short: "Assign a recording to a session",
		Args:  cobra.ExactArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			keyOrID := args[0]
			recordingID := args[1]
			err := client.AddRecordingToSession(keyOrID, projectID, recordingID)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to add recording to session: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Recording %s added to session\n", recordingID)
		},
	}
	addCmd.InheritedFlags()
	addCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when session-id-or-key is a session key)")
	return addCmd
}

func newSessionRecordingsRemoveCommand(params *baseParams) *cobra.Command {
	var projectID string
	removeCmd := &cobra.Command{
		Use:   "remove [session-id-or-key] [recording-id]",
		Short: "Remove a recording from a session",
		Args:  cobra.ExactArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			keyOrID := args[0]
			recordingID := args[1]
			err := client.RemoveRecordingFromSession(keyOrID, projectID, recordingID)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to remove recording from session: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Recording %s removed from session\n", recordingID)
		},
	}
	removeCmd.InheritedFlags()
	removeCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when session-id-or-key is a session key)")
	return removeCmd
}

func newDeleteSessionCommand(params *baseParams) *cobra.Command {
	var projectID string
	deleteSessionCmd := &cobra.Command{
		Use:   "delete [session-id-or-key]",
		Short: "Delete a session",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			keyOrID := args[0]
			err := client.DeleteSession(keyOrID, projectID)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to delete session: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Session deleted: %s\n", keyOrID)
		},
	}
	deleteSessionCmd.InheritedFlags()
	deleteSessionCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID (required when session-id-or-key is a session key)")
	return deleteSessionCmd
}
