# Foxglove CLI

Interact with Foxglove, directly from the command line.

Upgrading from v1? See the [v2 migration guide](docs/v2-migration.md) and
[release notes](https://github.com/foxglove/foxglove-cli/releases).

## Installation

Download the latest release for your OS and architecture:

| OS/Arch | Download command |
| --- | --- |
| macOS/arm64 (Apple Silicon) | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-arm64 -o foxglove && chmod +x foxglove` |
| macOS/amd64 (Intel) | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-amd64 -o foxglove && chmod +x foxglove` |
| Linux/amd64 | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-amd64 -o foxglove && chmod +x foxglove` |
| Linux/arm64 | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-arm64 -o foxglove && chmod +x foxglove` |
| Windows/amd64 | `curl.exe -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-amd64.exe -o foxglove.exe` |
| Windows/arm64 | `curl.exe -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-arm64.exe -o foxglove.exe` |

The download is saved in your current folder. Run `./foxglove --help` to try it,
or install it on macOS/Linux so you can run `foxglove` from any folder:

```sh
sudo mkdir -p /usr/local/bin
sudo install -m 755 ./foxglove /usr/local/bin/foxglove
foxglove --help
foxglove auth login
```

On Windows, run `.\foxglove.exe --help` in PowerShell, or move `foxglove.exe`
to a folder on your `Path` to use `foxglove` from any folder.

To install a specific release, see the [releases page](https://github.com/foxglove/foxglove-cli/releases).

Alternatively, build the Rust CLI from source. Rust 1.98.1 is pinned in
`rust-toolchain.toml`; [rustup](https://rustup.rs/) installs it automatically
when you run Cargo in this checkout:

    $ git clone git@github.com:foxglove/foxglove-cli.git
    $ cd foxglove-cli
    $ make build
    $ ./rust/target/release/foxglove-rust --help

`make install` copies that source build to `foxglove` in Cargo's bin directory
(`$CARGO_HOME/bin`, or `~/.cargo/bin` by default) on Unix-like systems.

## Getting started

Create a [free Foxglove account](https://app.foxglove.dev/signup) to start importing and interacting with data.

### Authentication

Log in to your Foxglove account from the command line:

```
$ foxglove auth login
```

Alternatively, you can configure the tool to use a [Foxglove API key](https://docs.foxglove.dev/docs/organization-setup/settings/#api-keys):

```
$ foxglove auth configure-api-key
```

This will overwrite any previously set credential. Use the [API key settings page](https://app.foxglove.dev/~/settings/apikeys)
to add the capabilities you intend to use (e.g. `data.upload` for importing data, `data.stream` for exporting, etc.).

In CI, set `FOXGLOVE_BEARER_TOKEN` (and optionally `FOXGLOVE_BASE_URL`) instead; they take precedence over the config file.
`BEARER_TOKEN`, `BASE_URL`, and `AUTH_TYPE` are no longer read. The base URL must use `https` unless the host is loopback.
`FOXGLOVE_DEFAULT_PROJECT_ID` sets the default project; `DEFAULT_PROJECT_ID` still works but is deprecated.

### Lists and pagination

List commands that accept `--limit` return 50 results by default; pass `--limit`
(1-2000) to change the page size. With `--format json`, list output is an object
whose `data` array holds the results:

```
$ foxglove devices list --format json
    {"data":[{"id":"dev_mHH1Cp4gPybCPR8y","name":"Adrian's Robot",...}]}
```

When more results may exist, a hint on stderr says how to get the next page.
Datasets, dataset episodes, dataset versions, episodes, episode versions, and
`datasets versions compare` page with cursors: their JSON output includes
`nextCursor`, which you pass to `--cursor`. The other commands that accept
`--limit` page with `--offset`.

### Devices

Before importing data, you must first create a device:

```
$ foxglove devices add --name RobotA
    Device created: dev_drpLqjBZYUzus3gv
```

List all devices:

```
$ foxglove devices list
    |          ID          |        NAME        |      CREATED AT      |      UPDATED AT      |
    |----------------------|--------------------|----------------------|----------------------|
    | dev_mHH1Cp4gPybCPR8y | Adrian's Robot     | 2021-10-28T17:20:55Z | 2021-10-28T17:20:55Z |
    | dev_WEJUVEOVApoIpe1M | GPS                | 2021-11-01T17:38:55Z | 2021-11-01T17:38:55Z |
    | dev_drpLqjBZYUzus3gv | updog              | 2021-11-25T02:22:45Z | 2021-11-25T02:22:45Z |
    | dev_lwjzOMxryMmP3yXg | nuScenes-v1.0-mini | 2021-12-09T21:45:51Z | 2021-12-09T21:45:51Z |
```

Filter devices by custom property, and sort them with `--sort-by` and `--sort-order`:

```
$ foxglove devices list --query 'properties.location:sf' --sort-by name --sort-order asc
```

Change a device's settings. Boolean flags take an optional `=true` or `=false`:

```
$ foxglove devices edit dev_drpLqjBZYUzus3gv --enabled=false --retain-recordings-seconds 604800 --auto-delete-after-import
    Device updated: RobotA
```

### Projects

Projects provide data isolation and access control to resources. See if your organization's
plan supports [Projects](https://docs.foxglove.dev/docs/projects).

List all projects:

```
$ foxglove projects list
    |          ID          |        NAME        | MEMBER COUNT | LAST RECORDING UPLOADED AT | INVITE ONLY |
    |----------------------|--------------------|--------------|----------------------------|-------------|
    | prj_mHH1Cp4gPybCPR8y |                    | 7            | 2021-10-28T17:20:55Z       | false       |
    | prj_WEJUVEOVApoIpe1M | My First Project   | 11           | 2021-11-01T17:38:55Z       | true        |
```

### Sites

List your organization's sites, and create, edit, or delete them:

```
$ foxglove sites list
$ foxglove sites add --name Garage --type edge --retain-recordings-seconds 604800
    Site created: site_drpLqjBZYUzus3gv
$ foxglove sites edit site_drpLqjBZYUzus3gv --name "Garage 2"
```

Save a Bring Your Own Storage site's ingest path policy, edit the copy, and save it back. `set`
sends the saved revision's `policy` and refuses to overwrite a newer save, because the file
records the revision it was edited from. A file holding only `schemaVersion` and `rules` works
too; pass `--previous-revision` with it to get the same check:

```
$ foxglove sites ingest-path-policy get site_drpLqjBZYUzus3gv > policy.json
$ foxglove sites ingest-path-policy set site_drpLqjBZYUzus3gv policy.json
    Ingest path policy for site site_drpLqjBZYUzus3gv is at revision 5
```

### Uploads and recordings

Upload ROS 1 bag and [MCAP](https://mcap.dev) files into the Foxglove platform:

```
$ foxglove upload ~/data/bags/gps.bag --device-name RobotA
```

List recordings:

```
$ foxglove recordings list
```

List only recordings that contain a topic:

```
$ foxglove recordings list --topic /camera/image
```

List sessions whose recordings overlap a time range, with each session's recording count and data range:

```
$ foxglove sessions list --key-matches drive --start 2026-04-19 --end 2026-04-20 --sort-by updatedAt
```

Request transfer of an existing edge recording to its configured Primary Site:

```sh
foxglove recordings transfer rec_123
```

### Exports

Retrieve data for a device, time range, and optional list of topics, and export it directly to the console (`--output-format json`) or to a new file (`--output-format mcap` or `--output-format bag1`):

```
# Output JSON (directly to console)
$ foxglove export --device-name RobotA --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --topics /tf --output-format json | head -n 5
    {"topic":"/tf","sequence":0,"log_time":1490149580.103843113,"publish_time":1490149580.103843113,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.117017840,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.113944947,"publish_time":1490149580.113944947,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.127078895,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.124028613,"publish_time":1490149580.124028613,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.137141823,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.134219155,"publish_time":1490149580.134219155,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.147199242,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.144292780,"publish_time":1490149580.144292780,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.157286100,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}

# Output MCAP file (output.mcap)
$ foxglove export --device-name RobotA --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format mcap --topics /gps/fix,/gps/fix_velocity > output.mcap

# Output ROS 1 bag file (output.bag)
$ foxglove export --device-name RobotA --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format bag1 --topics /gps/fix,/gps/fix_velocity > output.bag

# Export an episode to an MCAP file
$ foxglove export --episode-id <episode-id> --output-file episode.mcap

# Export version 2 of an episode to an MCAP file
$ foxglove export --episode-id <episode-id> --episode-version 2 --output-file episode.mcap
```

If you've output a file, inspect its contents:

```
# MCAP file
$ mcap info output.mcap
    profile: ros1
    messages: 6728
    duration: 5m39.304931438s
    start: 2021-03-22T08:03:38.473036858-07:00 (1616425418.473036858)
    end: 2021-03-22T08:09:17.777968296-07:00 (1616425757.777968296)
    compression:
            lz4: [1/1 chunks] (86.05%)
    channels:
            (1) /gps/fix           3364 msgs (9.91 Hz)   : sensor_msgs/NavSatFix [ros1msg]
            (2) /gps/fix_velocity  3364 msgs (9.91 Hz)   : geometry_msgs/TwistWithCovarianceStamped [ros1msg]
    attachments: 0

# ROS 1 bag file
$ rosbag reindex output.bag
$ rosbag info output.bag
    path:         output.bag
    version:      2.0
    duration:     5:39s (339s)
    start:        Mar 22 2021 08:03:38.47 (1616425418.47)
    end:          Mar 22 2021 08:09:17.78 (1616425757.78)
    size:         328.6 KB
    messages:     6728
    compression:  lz4 [1/1 chunks; 12.87%]
    uncompressed:   1.8 MB @ 5.5 KB/s
    compressed:   240.0 KB @ 0.7 KB/s (12.87%)
    types:        geometry_msgs/TwistWithCovarianceStamped [b00b6ce36bf21f646151de97da2c485c]
                  sensor_msgs/NavSatFix                    [7f6e605ad1e52d05162190ff17be80b6]
    topics:       /gps/fix            3364 msgs    : sensor_msgs/NavSatFix
                  /gps/fix_velocity   3364 msgs    : geometry_msgs/TwistWithCovarianceStamped
```

### Events

Create events to denote instances or time ranges of interest:

```
$ foxglove events add --device-id dev_mHH1Cp4gPybCPR8y \
    --start 2023-04-19T13:26:37.030302Z --end 2023-04-19T13:26:37.030302Z \
    --metadata requires-labeling:true
```

List all events:

```
$ foxglove events list
|          ID          |      DEVICE ID       |            START            |             END             |        CREATED AT        |        UPDATED AT        |           METADATA           |
|----------------------|----------------------|-----------------------------|-----------------------------|--------------------------|--------------------------|------------------------------|
| evt_N6doUtPYh8i7iZxf | dev_jCuXYeFwCkZowpHs | 2023-04-19T13:22:44.194041Z | 2023-04-19T13:22:44.194041Z | 2023-04-19T13:22:44.263Z | 2023-04-19T13:22:44.263Z | {}                           |
| evt_idMGJImlICYP4dcy | dev_mHH1Cp4gPybCPR8y | 2023-04-19T13:26:37.030302Z | 2023-04-19T13:26:37.030302Z | 2023-04-19T13:26:37.080Z | 2023-04-19T13:26:37.080Z | {"requires-labeling":"true"} |
```

Narrow the list to one event with `--event-id`, or to events created or updated
since a given time with `--created-after` and `--updated-after`:

```
$ foxglove events list --updated-after 2023-04-19T13:25:00Z
```

### Datasets and episodes

An episode is a time window over one or more recordings; a dataset is a named,
versioned collection of episodes. Both require a plan that supports
[datasets](https://docs.foxglove.dev/docs/datasets).

List datasets:

```
$ foxglove datasets list
ID | Name | Project ID | Description | Episode Count | Created At | Updated At
--- | --- | --- | --- | --- | --- | ---
ds_mHH1Cp4gPybCPR8y | Highway merges | prj_WEJUVEOVApoIpe1M | Curated merge maneuvers | 128 | 2026-04-19T13:22:44Z | 2026-05-02T09:11:03Z
```

List episodes, optionally narrowing to a time range, a member recording, or
episodes whose recordings are no longer all available:

```
$ foxglove episodes list --start 2026-04-19 --end 2026-04-20 --include-recordings
$ foxglove episodes list --recording-id rec_lwjzOMxryMmP3yXg --has-missing-recordings
```

Match an exact set of recording display IDs, with none missing or extra
(maximum 50 IDs). The flag accepts comma-separated values and may be repeated:

```
$ foxglove episodes list --exact-recording-ids rec_lwjzOMxryMmP3yXg,rec_4lVQ9mTp7vRqJY2s
```

Inspect an episode's version history, newest first. Use `--cursor` with the
returned `nextCursor` to request the next page:

```
$ foxglove episodes versions list ep_BCUw2zxpBnToJbOS --limit 20 --format json
```

Create an episode from one or more recordings. The window defaults to the span
of its recordings, and an existing episode whose latest version has the same
window, recordings, and custom properties is reused:

```
$ foxglove episodes add --recording-id rec_lwjzOMxryMmP3yXg \
    --start 2026-04-19T13:22:44Z --end 2026-04-19T13:23:14Z
Episode created: ep_BCUw2zxpBnToJbOS
```

Or create one from an event. The episode snapshots the event's current time range
and the completed recordings from its device that overlap it; later edits to the
event or new imports do not change it:

```
$ foxglove episodes add --event-id evt_Tn4kWq8sLx2BvR6p --metadata '{"label":"merge"}'
Episode created: ep_Rz6pXk3mWq9TfLbN
```

List the episodes in one dataset. The same filters apply, plus `addedAt` sorting
on when each episode joined the dataset:

```
$ foxglove datasets episodes list ds_mHH1Cp4gPybCPR8y --sort-by addedAt --sort-order desc
```

Member recordings are omitted unless you pass `--include-recordings`.

Episode custom properties are also omitted by default. Organizations with API
access to episode properties can set them during creation, request them on reads,
and filter by them:

```
$ foxglove episodes add --recording-id rec_lwjzOMxryMmP3yXg --property weather:rain
Episode created: ep_BCUw2zxpBnToJbOS
$ foxglove episodes list --properties-query 'weather:rain note:"night run"' --include-properties
$ foxglove episodes get ep_BCUw2zxpBnToJbOS --version 2 --include-properties --format json
$ foxglove datasets episodes list ds_mHH1Cp4gPybCPR8y --version 4 --properties-query 'weather:rain' --include-properties
```

`--property` (or `-p`) uses the same typed `key:value` syntax as device properties.
The CLI reads the organization's episode property definitions to convert strings,
numbers, booleans, enums, and multi-enums. Repeat a multi-enum key to set its full
list of values; it cannot set an empty list. Separate filter terms with spaces and
quote values that contain spaces. Filtering alone does not include properties in
the output; add `--include-properties` to see them.

Edit one episode's properties, time bounds, or recordings:

```
$ foxglove episodes edit ep_BCUw2zxpBnToJbOS --property weather:sun --remove-property reviewer --applies-to-version 2
Episode updated: ep_BCUw2zxpBnToJbOS (version 3)
$ foxglove episodes edit ep_BCUw2zxpBnToJbOS --end 2026-04-19T13:24:00Z --add-recording-id rec_lwjzOMxryMmP3yXg
```

Edits require `episodes.update`. Omitted fields keep their values, and
`--remove-property` removes a property. An edit that changes something creates a
new episode version; one that changes nothing keeps the current version. With
`--applies-to-version`, the edit fails instead of overwriting changes made since
that version. Existing datasets keep their pinned episode versions. `episodes get --version` reads an episode's
historical version; dataset membership reads return the version pinned by the
selected dataset version.

Apply edits to many episodes atomically from a JSON file: the API applies all of
them or none. Each entry requires `id` and can contain `appliesToVersion`,
`startTime`, `endTime`, `recordings` with `add`/`remove` arrays, and `properties`.
Property values merge by key; `null` removes a property. An ID-only entry changes
nothing and returns the current version. Timestamps are sent unchanged and must
use ISO 8601 at millisecond precision (for example `2026-04-19T13:22:44.123Z`).
A version conflict names each conflicting episode's current version.

```
$ cat edits.json
{"episodes":[{"id":"ep_BCUw2zxpBnToJbOS","appliesToVersion":3,"recordings":{"add":["rec_lwjzOMxryMmP3yXg"]}}]}
$ foxglove episodes edit-bulk edits.json --format json
```

Episode JSON includes `version` and `latestVersion`. Episode and dataset membership
table/CSV output append Version, Latest Version, and Custom Properties columns.
Comparison output also appends Previous Version and Previous Custom Properties;
JSON preserves the previous episode snapshot, and summaries report changed
members alongside additions and removals.

Property options and `episodes edit` return an error if your organization does
not have access to episode properties.

Create a dataset and change its episodes. Changes stay pending in the dataset's
draft until you commit them as a new version, or discard them with
`foxglove datasets discard`. `datasets episodes list --version draft` lists the
draft, pending changes included:

```
$ foxglove datasets add --name "Night merges" --episode-id ep_BCUw2zxpBnToJbOS
Dataset created: ds_Qp4sT7vKx2NbLm9W
Added 1 episode
Run foxglove datasets commit ds_Qp4sT7vKx2NbLm9W to commit the draft as a new version.
$ foxglove datasets episodes add ds_Qp4sT7vKx2NbLm9W ep_Vt3oQm8LkDs2PaXn ep_Hq7sWd1ZrNc4YbEe
$ foxglove datasets episodes remove ds_Qp4sT7vKx2NbLm9W ep_BCUw2zxpBnToJbOS
$ foxglove datasets episodes list ds_Qp4sT7vKx2NbLm9W --version draft
$ foxglove datasets commit ds_Qp4sT7vKx2NbLm9W --description "Merges after the lane change"
Committed version 1 with 2 episodes (2 added, 0 removed, 0 changed)
```

Without `--description`, the new version keeps the draft's description. Edit a
version's description later with `datasets versions edit`; pass an empty
`--description` to clear it:

```
$ foxglove datasets versions edit ds_Qp4sT7vKx2NbLm9W 1 --description "Merges at night"
Version 1 of dataset ds_Qp4sT7vKx2NbLm9W updated
```

Version output includes the description and who committed the version: an org
member ID, or an API key ID when an API key committed it.

Select an episode version when adding an episode or updating one in the draft.
Omit `--episode-version` to select the latest version. An already selected version
is reported as unchanged; the API rejects update IDs outside the draft.

```
$ foxglove datasets episodes add ds_Qp4sT7vKx2NbLm9W ep_BCUw2zxpBnToJbOS --episode-version 2
$ foxglove datasets episodes update ds_Qp4sT7vKx2NbLm9W ep_BCUw2zxpBnToJbOS --episode-version 3
Updated 1 episode
Run foxglove datasets commit ds_Qp4sT7vKx2NbLm9W to commit the draft as a new version.
```

List a dataset's versions, see which episodes changed between two of them, or
restore an earlier one into the draft. `datasets episodes list --version` lists
any version by number:

```
$ foxglove datasets versions list ds_mHH1Cp4gPybCPR8y
$ foxglove datasets versions compare ds_mHH1Cp4gPybCPR8y 3 5
$ foxglove datasets versions restore ds_mHH1Cp4gPybCPR8y 3
```

Restoring a dataset version also restores the selected version of each episode.
In the restore summary, `changed` counts episodes that stay in the dataset
but now use a different episode version. With `--force`, the discard summary includes
pending episode version updates.

Download a committed version of a dataset. The CLI writes one MCAP file per
episode, at the episode version selected by the dataset version, plus a
`manifest.json` that records each episode's version:

```
$ foxglove datasets download ds_mHH1Cp4gPybCPR8y --version 4
Episode 1 of 128: 41231 bytes
...
Downloaded 127 of 128 episodes to Highway-merges-v4 (1 failed)
```

If an earlier manifest does not record the same episode version, the CLI
downloads that episode again.

A version with recordings that are no longer available can't be downloaded.
Select another version, or remove the affected episodes from the dataset and
commit a new version. List them with
`foxglove datasets episodes list DATASET_ID --version N --has-missing-recordings`.

Use `--include-properties` to add episode custom properties to each manifest
entry, including empty `{}` values. This also works when resuming a download:

```
$ foxglove datasets download ds_mHH1Cp4gPybCPR8y --version 4 --include-properties
$ foxglove datasets versions compare ds_mHH1Cp4gPybCPR8y 3 4 --include-properties --format json
```

Changing `--include-properties` on a later run updates the manifest without
downloading completed MCAP files again. If your organization does not have
access to episode properties, the command fails and leaves earlier downloads
as they were.

### Extensions

With a Foxglove [Team plan](https://foxglove.dev/pricing), you can upload and share
[extensions](https://foxglove.dev/docs/studio/extensions/getting-started)
within your organization.

Create and package an extension with the
[`foxglove-extension`](https://github.com/foxglove/create-foxglove-extension/)
tool.

Publish an extension to install it for all Foxglove organization members:

```
$ foxglove extensions publish ./my-extension.1.0.0.foxe
```

Use this same command to update an existing extension with a newer version –
the last published version of an extension will be installed across your organization.

List all extensions:

```
$ foxglove extensions list
    [
        {
            "id": "ext_BsGXKGsZ9c4WQF1",
            "name": "my_new_panel",
            "publisher": "panel-publisher",
            "displayName": "My New Panel",
            "description": "Creates a panel",
            "activeVersion": "1.0.0",
            "sha256Sum": "395c3af8745ab104cd902d937366719a402bda4677ed3671cb38522c1ba13cbe"
        }
    ]
```

Unpublish an extension by its ID to **delete** its files and uninstall it for
all Foxglove organization members:

```
$ foxglove extensions unpublish ext_BsGXKGsZ9c4WQF1
```

## Shell autocompletion

Certain shells (bash, zsh, fish, and PowerShell) support generated
autocompletion for commands, flags, and file paths.

To enable this, consult your shell instructions under `$ foxglove completion <shell> -h`.

## Development

See the [Rust development guide](rust/README.md) for building and testing.

## Publishing a release

1. Open [GitHub Releases](https://github.com/foxglove/foxglove-cli/releases) and click
   **Draft a new release**. Under **Choose a tag**, enter the new version
   (for example, `v2.0.0`) and select **Create new tag** with **Target: main**.
   Tags must use `vMAJOR.MINOR.PATCH`, optionally followed by a prerelease suffix
   such as `v2.0.0-rc.1`. CI rejects tags without the `v` prefix or three version numbers.
2. Enter a title and click **Generate release notes**.
3. Click **Publish release** to start the CI workflow. Saving a draft or pushing a tag alone does not build a release.
4. Check the release's CI workflow in [Actions](https://github.com/foxglove/foxglove-cli/actions).
   Once it passes, the release will have all six binaries and `CHECKSUMS.txt`.

## Stay in touch

Join our [Discord community](https://foxglove.dev/chat) to ask questions, share feedback, and stay up to date on what our team is working on.
