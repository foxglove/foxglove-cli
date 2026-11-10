<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove events edit`

[Command reference](README.md) · [Parent command](events.md)

```text
Edit an event's time range, event type, metadata, or properties

Usage: foxglove events edit [OPTIONS] <EVENT_ID>

Arguments:
  <EVENT_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --start <START>
          New start of the event (inclusive, ISO 8601)

      --config <CONFIG>
          Config file

      --end <END>
          New end of the event (inclusive, ISO 8601)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --event-type-id <EVENT_TYPE_ID>
          Event type ID; the event must conform to the event type's schema

      --remove-event-type
          Remove the event's event type

  -m, --metadata <METADATA>
          Metadata colon-separated key/value pair; replaces all of the event's metadata, repeat for more pairs

      --clear-metadata
          Remove all of the event's metadata

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

      --remove-property <REMOVE_PROPERTY>
          Remove a property assignment; repeat for more keys

  -h, --help
          Print help
```
