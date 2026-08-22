package cmd

import (
	"fmt"
	"os"
	"strconv"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/spf13/cobra"
)

func parseEventTypeCustomProperties(pairs []string) ([]api.EventTypeCustomProperty, error) {
	customProperties := make([]api.EventTypeCustomProperty, 0, len(pairs))
	for _, pair := range pairs {
		id := pair
		required := false
		if key, val, err := util.SplitPair(pair, ':'); err == nil {
			id = key
			parsed, parseErr := strconv.ParseBool(val)
			if parseErr != nil {
				return nil, fmt.Errorf("invalid --custom-property value %q: required must be true or false", pair)
			}
			required = parsed
		}
		if id == "" {
			return nil, fmt.Errorf("invalid --custom-property value %q: custom property ID is required", pair)
		}
		customProperties = append(customProperties, api.EventTypeCustomProperty{
			ID:       id,
			Required: required,
		})
	}
	return customProperties, nil
}

func newListEventTypesCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	cmd := &cobra.Command{
		Use:   "list",
		Short: "List event types",
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			format = ResolveFormat(format, isJsonFormat)
			err := renderList(
				os.Stdout,
				&api.EventTypesRequest{},
				client.EventTypes,
				format,
			)
			if err != nil {
				dief("Failed to list event types: %s", err)
			}
		},
	}
	AddFormatFlag(cmd, &format)
	AddJsonFlag(cmd, &isJsonFormat)
	return cmd
}

func newAddEventTypeCommand(params *baseParams) *cobra.Command {
	var name string
	var colorName string
	var customPropertyPairs []string
	addCmd := &cobra.Command{
		Use:   "add",
		Short: "Add an event type",
		Run: func(cmd *cobra.Command, args []string) {
			if name == "" {
				dief("--name is required")
			}
			customProperties, err := parseEventTypeCustomProperties(customPropertyPairs)
			if err != nil {
				dief("Failed to add event type: %s", err)
			}
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			resp, err := client.CreateEventType(api.CreateEventTypeRequest{
				Name:             name,
				ColorName:        colorName,
				CustomProperties: customProperties,
			})
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to add event type: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Created event type: %s\n", resp.ID)
		},
	}
	addCmd.PersistentFlags().StringVarP(&name, "name", "", "", "Name of the event type")
	addCmd.PersistentFlags().StringVarP(&colorName, "color-name", "", "", "Color name of the event type, e.g. red")
	addCmd.PersistentFlags().StringArrayVarP(&customPropertyPairs, "custom-property", "", []string{}, "Custom property ID, or ID:true/false for required. Multiple may be specified.")
	return addCmd
}

func newGetEventTypeCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	getCmd := &cobra.Command{
		Use:   "get [ID]",
		Short: "Get an event type",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			eventType, err := client.GetEventType(args[0])
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				if err == api.ErrNotFound {
					dief("Event type not found: %s", args[0])
				}
				dief("Failed to get event type: %s", err)
			}
			format = ResolveFormat(format, isJsonFormat)
			if err := renderRecord(os.Stdout, eventType, format); err != nil {
				dief("Failed to get event type: %s", err)
			}
		},
	}
	AddFormatFlag(getCmd, &format)
	AddJsonFlag(getCmd, &isJsonFormat)
	return getCmd
}

func newEditEventTypeCommand(params *baseParams) *cobra.Command {
	var name string
	var colorName string
	var customPropertyPairs []string
	editCmd := &cobra.Command{
		Use:   "edit [ID]",
		Short: "Edit an event type",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			req := api.UpdateEventTypeRequest{}
			if cmd.Flags().Changed("name") {
				req.Name = name
			}
			if cmd.Flags().Changed("color-name") {
				req.ColorName = colorName
			}
			if cmd.Flags().Changed("custom-property") {
				customProperties, err := parseEventTypeCustomProperties(customPropertyPairs)
				if err != nil {
					dief("Failed to edit event type: %s", err)
				}
				req.CustomProperties = &customProperties
			}
			if req.Name == "" && req.ColorName == "" && req.CustomProperties == nil {
				dief("Nothing to update")
			}
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			resp, err := client.UpdateEventType(args[0], req)
			if err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				if err == api.ErrNotFound {
					dief("Event type not found: %s", args[0])
				}
				dief("Failed to edit event type: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Event type updated: %s\n", resp.ID)
		},
	}
	editCmd.PersistentFlags().StringVarP(&name, "name", "", "", "Name of the event type")
	editCmd.PersistentFlags().StringVarP(&colorName, "color-name", "", "", "Color name of the event type, e.g. red")
	editCmd.PersistentFlags().StringArrayVarP(&customPropertyPairs, "custom-property", "", []string{}, "Custom property ID, or ID:true/false for required. Replaces the current list. Multiple may be specified.")
	return editCmd
}

func newDeleteEventTypeCommand(params *baseParams) *cobra.Command {
	deleteCmd := &cobra.Command{
		Use:   "delete [ID]",
		Short: "Delete an event type",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			if err := client.DeleteEventType(args[0]); err != nil {
				if err == api.ErrForbidden {
					dief("Not authenticated. Run foxglove auth login.")
				}
				dief("Failed to delete event type: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Event type deleted: %s\n", args[0])
		},
	}
	return deleteCmd
}
