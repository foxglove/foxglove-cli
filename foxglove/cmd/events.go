package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/foxglove/foxglove-cli/foxglove/api"
	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func parseMetadataPairs(keyvals []string) (map[string]string, error) {
	metadata := make(map[string]string)
	for _, kv := range keyvals {
		key, val, err := util.SplitPair(kv, ':')
		if err != nil {
			return nil, fmt.Errorf("invalid metadata key/value pair: %s", kv)
		}
		metadata[key] = val
	}
	return metadata, nil
}

func parseEventProperties(
	client *api.FoxgloveClient,
	propertyPairs []string,
	removeKeys []string,
) (map[string]interface{}, error) {
	properties, err := util.EventProperties(propertyPairs, client)
	if err != nil {
		return nil, err
	}
	if len(removeKeys) == 0 {
		return properties, nil
	}
	if properties == nil {
		properties = make(map[string]interface{})
	}
	for _, key := range removeKeys {
		if key == "" {
			return nil, fmt.Errorf("property key to remove cannot be empty")
		}
		properties[key] = nil
	}
	return properties, nil
}

// joinQueryFields encodes --query-field values as the public queryFields
// parameter. The API uses OpenAPI explode: false (comma-joined), not repeated
// queryFields= params.
func joinQueryFields(queryFields []string) (string, error) {
	for _, qf := range queryFields {
		if qf != "metadata" && qf != "properties" {
			return "", fmt.Errorf("invalid --query-field value %q: must be \"metadata\" or \"properties\"", qf)
		}
	}
	return strings.Join(queryFields, ","), nil
}

func validateCreateEventData(eventTypeID string, metadataPairs, propertyPairs []string) error {
	if eventTypeID != "" && len(metadataPairs) > 0 {
		return fmt.Errorf("--metadata cannot be used with --event-type-id")
	}
	if len(metadataPairs) > 0 && len(propertyPairs) > 0 {
		return fmt.Errorf("--metadata cannot be used with --property")
	}
	return nil
}

func newAddEventCommand(params *baseParams) *cobra.Command {
	var deviceID string
	var deviceName string
	var projectID string
	var start string
	var end string
	var keyvals []string
	var eventTypeID string
	var propertyPairs []string
	addEventCmd := &cobra.Command{
		Use:   "add",
		Short: "Add an event",
		Run: func(cmd *cobra.Command, args []string) {
			if err := validateCreateEventData(eventTypeID, keyvals, propertyPairs); err != nil {
				dief("Failed to add event: %s", err)
			}
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			metadata, err := parseMetadataPairs(keyvals)
			if err != nil {
				dief("Failed to add event: %s", err)
			}

			properties, err := util.EventProperties(propertyPairs, client)
			if err != nil {
				dief("Failed to add event: %s", err)
			}

			startTime, err := maybeConvertToRFC3339(start)
			if err != nil {
				dief("failed to parse start time: %s", err)
			}
			endTime, err := maybeConvertToRFC3339(end)
			if err != nil {
				dief("failed to parse end time: %s", err)
			}

			response, err := client.CreateEvent(api.CreateEventRequest{
				DeviceID:    deviceID,
				DeviceName:  deviceName,
				ProjectID:   projectID,
				Start:       startTime,
				End:         endTime,
				Metadata:    metadata,
				EventTypeID: eventTypeID,
				Properties:  properties,
			})
			if err != nil {
				dief("Failed to add event: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Created event: %s\n", response.ID)
		},
	}
	addEventCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	addEventCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	addEventCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID used to disambiguate device-id and device-name")
	addEventCmd.PersistentFlags().StringVarP(&start, "start", "", "", "Start of event (inclusive), RFC 3339 or ISO 8601 format")
	addEventCmd.PersistentFlags().StringVarP(&end, "end", "", "", "End of event (inclusive), RFC 3339 or ISO 8601 format")
	addEventCmd.PersistentFlags().StringArrayVarP(&keyvals, "metadata", "m", []string{}, "Metadata colon-separated key value pair. Multiple may be specified.")
	addEventCmd.PersistentFlags().StringVarP(&eventTypeID, "event-type-id", "", "", "Event type ID to associate with this event (e.g. evtt_123)")
	addEventCmd.PersistentFlags().StringArrayVarP(&propertyPairs, "property", "p", []string{}, "Event property colon-separated key value pair. Multiple may be specified.")
	AddDeviceAutocompletion(addEventCmd, params)
	return addEventCmd
}

func newListEventsCommand(params *baseParams) *cobra.Command {
	var format string
	var deviceID string
	var deviceName string
	var projectID string
	var sortBy string
	var sortOrder string
	var limit int
	var offset int
	var start string
	var end string
	var createdAfter string
	var updatedAfter string
	var query string
	var eventID string
	var eventTypeID string
	var queryFields []string
	var isJsonFormat bool
	eventsListCmd := &cobra.Command{
		Use:   "list",
		Short: "List events",
		Run: func(cmd *cobra.Command, args []string) {
			queryFieldsValue, err := joinQueryFields(queryFields)
			if err != nil {
				dief("%s", err)
			}
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			startTime, err := maybeConvertToRFC3339(start)
			if err != nil {
				dief("failed to parse start time: %s", err)
			}
			endTime, err := maybeConvertToRFC3339(end)
			if err != nil {
				dief("failed to parse end time: %s", err)
			}
			createdAfterTime, err := maybeConvertToRFC3339(createdAfter)
			if err != nil {
				dief("failed to parse created-after time: %s", err)
			}
			updatedAfterTime, err := maybeConvertToRFC3339(updatedAfter)
			if err != nil {
				dief("failed to parse updated-after time: %s", err)
			}
			format = ResolveFormat(format, isJsonFormat)
			err = renderList(
				os.Stdout,
				&api.EventsRequest{
					CreatedAfter: createdAfterTime,
					DeviceID:     deviceID,
					DeviceName:   deviceName,
					End:          endTime,
					EventID:      eventID,
					EventTypeID:  eventTypeID,
					Limit:        limit,
					Offset:       offset,
					ProjectID:    projectID,
					Query:        query,
					QueryFields:  queryFieldsValue,
					SortBy:       sortBy,
					SortOrder:    sortOrder,
					Start:        startTime,
					UpdatedAfter: updatedAfterTime,
				},
				client.Events,
				format,
			)
			if err != nil {
				dief("Failed to list events: %s", err)
			}
		},
	}
	eventsListCmd.InheritedFlags()
	eventsListCmd.PersistentFlags().StringVarP(&deviceID, "device-id", "", "", "Device ID")
	eventsListCmd.PersistentFlags().StringVarP(&deviceName, "device-name", "", "", "Device name")
	eventsListCmd.PersistentFlags().StringVarP(&projectID, "project-id", "", viper.GetString("default_project_id"), "Project ID")
	eventsListCmd.PersistentFlags().StringVarP(&eventID, "event-id", "", "", "Filter by exact event ID")
	eventsListCmd.PersistentFlags().StringVarP(&sortBy, "sort-by", "", "", "Field to sort by (id, deviceId, deviceName, start, createdAt, updatedAt)")
	eventsListCmd.PersistentFlags().StringVarP(&sortOrder, "sort-order", "", "asc", "sort order")
	eventsListCmd.PersistentFlags().IntVarP(&limit, "limit", "", 100, "limit")
	eventsListCmd.PersistentFlags().IntVarP(&offset, "offset", "", 0, "offset")
	eventsListCmd.PersistentFlags().StringVarP(&start, "start", "", "", "Exclude events that do not intersect this start time, RFC 3339 or ISO 8601 format")
	eventsListCmd.PersistentFlags().StringVarP(&end, "end", "", "", "Exclude events that do not intersect this end time, RFC 3339 or ISO 8601 format")
	eventsListCmd.PersistentFlags().StringVarP(&createdAfter, "created-after", "", "", "Return events created after this time, RFC 3339 or ISO 8601 format")
	eventsListCmd.PersistentFlags().StringVarP(&updatedAfter, "updated-after", "", "", "Return events updated after this time, RFC 3339 or ISO 8601 format")
	eventsListCmd.PersistentFlags().StringVarP(&query, "query", "", "", "Filter by properties or metadata, e.g. \"$key:$value\". See API docs for query syntax.")
	eventsListCmd.PersistentFlags().StringVarP(&eventTypeID, "event-type-id", "", "", "Filter by event type ID (e.g. evtt_123)")
	eventsListCmd.PersistentFlags().StringArrayVarP(&queryFields, "query-field", "", []string{}, "Fields to query by (\"metadata\" or \"properties\"). Multiple may be specified. Defaults to \"metadata\".")
	AddDeviceAutocompletion(eventsListCmd, params)
	AddFormatFlag(eventsListCmd, &format)
	AddJsonFlag(eventsListCmd, &isJsonFormat)
	return eventsListCmd
}

func newGetEventCommand(params *baseParams) *cobra.Command {
	var format string
	var isJsonFormat bool
	getEventCmd := &cobra.Command{
		Use:   "get [ID]",
		Short: "Get an event",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			event, err := client.GetEvent(args[0])
			if err != nil {
				dief("Failed to get event: %s", err)
			}
			format = ResolveFormat(format, isJsonFormat)
			if err := renderRecord(os.Stdout, event, format); err != nil {
				dief("Failed to get event: %s", err)
			}
		},
	}
	getEventCmd.InheritedFlags()
	AddFormatFlag(getEventCmd, &format)
	AddJsonFlag(getEventCmd, &isJsonFormat)
	return getEventCmd
}

func newEditEventCommand(params *baseParams) *cobra.Command {
	var start string
	var end string
	var keyvals []string
	var eventTypeID string
	var propertyPairs []string
	var removeProperties []string
	var clearMetadata bool
	editEventCmd := &cobra.Command{
		Use:   "edit [ID]",
		Short: "Edit an event",
		Args:  cobra.ExactArgs(1),
		PreRunE: func(cmd *cobra.Command, args []string) error {
			if cmd.Flags().Changed("metadata") && clearMetadata {
				return fmt.Errorf("--metadata and --clear-metadata cannot be used together")
			}
			return nil
		},
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)

			req := api.UpdateEventRequest{}
			if cmd.Flags().Changed("start") {
				startTime, err := maybeConvertToRFC3339(start)
				if err != nil {
					dief("failed to parse start time: %s", err)
				}
				req.Start = startTime
			}
			if cmd.Flags().Changed("end") {
				endTime, err := maybeConvertToRFC3339(end)
				if err != nil {
					dief("failed to parse end time: %s", err)
				}
				req.End = endTime
			}
			if cmd.Flags().Changed("metadata") {
				metadata, err := parseMetadataPairs(keyvals)
				if err != nil {
					dief("Failed to edit event: %s", err)
				}
				req.Metadata = &metadata
			}
			if clearMetadata {
				metadata := map[string]string{}
				req.Metadata = &metadata
			}
			if cmd.Flags().Changed("event-type-id") {
				req.EventTypeID = &eventTypeID
			}
			if cmd.Flags().Changed("property") || cmd.Flags().Changed("remove-property") {
				properties, err := parseEventProperties(client, propertyPairs, removeProperties)
				if err != nil {
					dief("Failed to edit event: %s", err)
				}
				req.Properties = properties
			}
			if req.Start == "" && req.End == "" && req.Metadata == nil && req.Properties == nil && req.EventTypeID == nil {
				dief("Nothing to update")
			}

			resp, err := client.UpdateEvent(args[0], req)
			if err != nil {
				dief("Failed to edit event: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Event updated: %s\n", resp.ID)
		},
	}
	editEventCmd.InheritedFlags()
	editEventCmd.PersistentFlags().StringVarP(&start, "start", "", "", "Event start time (inclusive), RFC 3339 or ISO 8601 format")
	editEventCmd.PersistentFlags().StringVarP(&end, "end", "", "", "Event end time (inclusive), RFC 3339 or ISO 8601 format")
	editEventCmd.PersistentFlags().StringArrayVarP(&keyvals, "metadata", "m", []string{}, "Replace all event metadata with these colon-separated key value pairs. Multiple may be specified.")
	editEventCmd.PersistentFlags().BoolVarP(&clearMetadata, "clear-metadata", "", false, "Remove all event metadata.")
	editEventCmd.PersistentFlags().StringVarP(&eventTypeID, "event-type-id", "", "", "Event type ID. Pass an empty value to remove the event type.")
	editEventCmd.PersistentFlags().StringArrayVarP(&propertyPairs, "property", "p", []string{}, "Set these event property keys. Other keys stay unchanged. Multiple may be specified.")
	editEventCmd.PersistentFlags().StringArrayVarP(&removeProperties, "remove-property", "", []string{}, "Remove these event property keys. Multiple may be specified.")
	return editEventCmd
}

func newDeleteEventCommand(params *baseParams) *cobra.Command {
	deleteEventCmd := &cobra.Command{
		Use:   "delete [ID]",
		Short: "Delete an event",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			client := api.NewRemoteFoxgloveClient(
				params.baseURL, *params.clientID,
				params.token,
				params.userAgent,
			)
			if err := client.DeleteEvent(args[0]); err != nil {
				dief("Failed to delete event: %s", err)
			}
			fmt.Fprintf(os.Stderr, "Event deleted: %s\n", args[0])
		},
	}
	deleteEventCmd.InheritedFlags()
	return deleteEventCmd
}
