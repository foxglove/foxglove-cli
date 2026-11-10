<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove event-types edit`

[Command reference](README.md) · [Parent command](event-types.md)

```text
Edit an event type's name, color, or custom properties

Usage: foxglove event-types edit [OPTIONS] <EVENT_TYPE_ID>

Arguments:
  <EVENT_TYPE_ID>


Options:
      --clear-custom-properties
          Remove every custom property from the event type

      --client-id <CLIENT_ID>
          Foxglove client ID

      --color <COLOR>
          New color name, such as red

      --config <CONFIG>
          Config file

      --custom-property <ID[:required]>
          Custom property ID, with a :required suffix if events must set it; replaces the existing list, so repeat for each property in display order

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --name <NAME>
          New name, unique within your organization

  -h, --help
          Print help
```
