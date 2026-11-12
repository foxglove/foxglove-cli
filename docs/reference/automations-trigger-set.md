<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations trigger set`

[Command reference](README.md) · [Parent command](automations-trigger.md)

```text
Replace an automation's trigger with a new definition.

A clock.updated trigger runs on a schedule and requires --schedule-cadence and --schedule-anchor; other event types do not accept a schedule. --event-type-id applies only to event.created and event.updated triggers.

Usage: foxglove automations trigger set [OPTIONS] --event-type <EVENT_TYPE> <AUTOMATION_ID>

Arguments:
  <AUTOMATION_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --enabled[=<ENABLED>]
          Enable or disable the trigger. When omitted, the trigger is enabled, even when it replaces a disabled trigger; pass --enabled=false to keep it disabled

      --config <CONFIG>
          Config file

      --event-type <EVENT_TYPE>
          Event that starts a run: recording.created, recording.imported, session.created, event.created, event.updated, or clock.updated (a schedule)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --event-type-id <EVENT_TYPE_ID>
          Start runs only for events of this event type (event.created and event.updated only)

      --schedule-anchor <SCHEDULE_ANCHOR>
          UTC time at a whole hour when the schedule starts (ISO 8601); its calendar fields set the recurrence, and monthly anchors use days 1-28

      --schedule-cadence <SCHEDULE_CADENCE>
          How often a clock.updated trigger runs: hourly, daily, weekly, or monthly

  -h, --help
          Print help (see a summary with '-h')
```
