## Foxglove CLI

The Foxglove CLI tool enables command line interaction with remotely stored
data in Foxglove Data Platform.

### Installation

Download the latest release for your OS and architecture:

| OS/Arch     |                                                                                                            |
|--------|------------------------------------------------------------------------------------------------------------|
| linux/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-amd64 -o foxglove |
| macos/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-amd64 -o foxglove |
| windows/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-amd64.exe -o foxglove.exe |
| linux/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-arm64 -o foxglove |
| macos/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-arm64 -o foxglove |
| windows/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-arm64.exe -o foxglove.exe |

To install a specific release, see the [releases page](https://github.com/foxglove/foxglove-cli/releases).

Alternatively, install the CLI tool from source (requires Go >= 1.21) – this will install it to `$GOPATH/bin`:

    $ git clone git@github.com:foxglove/foxglove-cli.git
    $ make install

### Usage

Run `foxglove -h` for complete usage documentation.

#### Authenticating

Before interacting with data, you must [create a free Foxglove account](https://console.foxglove.dev/signup) 
and authenticate from the command line:

```
$ foxglove auth login
```

Alternatively, you can configure the tool to use a [Foxglove API key](https://docs.foxglove.dev/docs/organization-setup/settings/#api-keys):

```
$ foxglove auth configure-api-key
```

This will overwrite any previously set credential. Use the [API key settings page](https://console.foxglove.dev/foxglovehq/settings/apikeys)
to add the capabilities you intend to use (e.g. `data.upload` for importing data, `data.stream` for exporting, etc.).

### Devices

Before importing data, you must first create a device:

```
$ foxglove devices add --name "my device"
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

#### Imported data

Import ROS 1 bag and MCAP files into the Foxglove platform:

```
$ foxglove data imports add ~/data/bags/gps.bag --device-id dev_drpLqjBZYUzus3gv

// Shorthand version
$ foxglove data import ~/data/bags/gps.bag --device-id dev_drpLqjBZYUzus3gv
```

List all imports:

```
$ foxglove data imports list
|              IMPORT ID               |      DEVICE ID       |              FILENAME              |     IMPORT TIME      |        START         |         END          | INPUT TYPE | OUTPUT TYPE | INPUT SIZE | TOTAL OUTPUT SIZE |
|--------------------------------------|----------------------|------------------------------------|----------------------|----------------------|----------------------|------------|-------------|------------|-------------------|
| 55c8480b-0fef-243a-ea74-f07063f51c6e | dev_flm75pLkfzUBX2DH | demo.bag                           | 2022-01-28T22:37:44Z | 2017-03-22T02:26:20Z | 2017-03-22T02:26:27Z | bag1       | mcap0       | 70311473   | 68833788          |
| 31285b3d-3f97-ea58-3751-a1597ae3f16f | dev_Wm1gvryKJmREqnVT | transbot_2022-02-21-21-58-17_1.bag | 2022-02-21T22:03:32Z | 2022-02-21T21:58:17Z | 2022-02-21T21:58:21Z | bag1       | mcap0       | 12823      | 8847              |
| 837cfa3e-541e-9540-1c7a-a1f1b3ef3694 | dev_jCuXYeFwCkZowpHs | gps4.bag                           | 2022-03-17T12:34:34Z | 2021-03-22T15:03:38Z | 2021-03-22T15:09:18Z | bag1       | mcap0       | 5321782    | 1619916           |
| 820e51d8-9f1d-8ec5-f586-41bbaa87d45d | dev_JOgi4YiCRgaoszKw | input.bag                          | 2021-11-02T16:34:49Z | 2016-11-18T23:46:10Z | 2016-11-18T23:51:25Z | bag1       | mcap0       | 1770886024 | 1571403963        |
| 8f563534-264d-ad79-b404-684c8639d4a0 | dev_Wm1gvryKJmREqnVT | transbot_2022-02-21-19-44-47_1.bag | 2022-02-21T20:12:54Z | 2022-02-21T19:44:47Z | 2022-02-21T19:44:47Z | bag1       | mcap0       | 10964      | 8196              |
| b8c45da6-2110-8c77-370f-a6ec482cdcf6 | dev_Wm1gvryKJmREqnVT | transbot_2022-02-21-22-49-04_0.bag | 2022-02-21T22:50:48Z | 2022-02-21T22:49:05Z | 2022-02-21T22:49:23Z | bag1       | mcap0       | 16734      | 10688             |
| 0a400ef1-3892-1c6c-3a38-c0a80ed85749 | dev_JtSXCGiM0RC2YHDO | nuscenes.bag                       | 2022-02-24T15:13:32Z | 2018-07-24T03:28:47Z | 2018-07-24T03:29:06Z | bag1       | mcap0       | 87397054   | 85923915          |
| 0a400ef1-3892-1c6c-3a38-c0a80ed85749 | dev_mHH1Cp4gPybCPR8y | nuscenes-0061-v1.bag               | 2022-01-11T22:05:09Z | 2018-07-24T03:28:47Z | 2018-07-24T03:29:06Z | bag1       | mcap0       | 87397054   | 85923915          |
| 5ad56d95-7dcc-f12a-9b09-0f4d4ec9e2e5 | dev_mHH1Cp4gPybCPR8y | input.bag                          | 2021-11-03T23:21:37Z | 2017-03-22T02:26:20Z | 2017-03-22T02:26:26Z | bag1       | mcap0       | 32761      | 38542             |
```

#### Exported data

Export data by providing a device, time range, and an optional list of topics to include. 

You can also specify the output format (`mcap0`, `bag1`, and `json`) and file name:

```
// Output MCAP file (output.mcap)
$ foxglove data export --device-id dev_drpLqjBZYUzus3gv --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format mcap0 --topics /gps/fix,/gps/fix_velocity > output.mcap

// Output ROS 1 bag file (output.bag)
$ foxglove data export --device-id dev_drpLqjBZYUzus3gv --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format bag1 --topics /gps/fix,/gps/fix_velocity > output.bag

// Output JSON (directly to console)
$ foxglove data export --device-id dev_flm75pLkfzUBX2DH --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --topics /tf --output-format json | head -n 5
    {"topic":"/tf","sequence":0,"log_time":1490149580.103843113,"publish_time":1490149580.103843113,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.117017840,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.113944947,"publish_time":1490149580.113944947,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.127078895,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.124028613,"publish_time":1490149580.124028613,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.137141823,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.134219155,"publish_time":1490149580.134219155,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.147199242,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
    {"topic":"/tf","sequence":0,"log_time":1490149580.144292780,"publish_time":1490149580.144292780,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.157286100,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
```

If you've output a file, you can inspect the exported data:

```
// MCAP file
$ mcap info output.mcap
    library: mcap go #(devel); fg-data-platform-db07abe7
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

// ROS 1 bag file
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

#### Events

Create events to denote instances or time ranges of interest:

```
$ foxglove events add --device-id dev_mHH1Cp4gPybCPR8y --start 2023-04-19T13:26:37.030302Z --end 2023-04-19T13:26:37.030302Z --metadata requires-labeling:true
```

List all events:

```
$ foxglove events list
|          ID          |      DEVICE ID       |            START            |             END             |        CREATED AT        |        UPDATED AT        |           METADATA           |
|----------------------|----------------------|-----------------------------|-----------------------------|--------------------------|--------------------------|------------------------------|
| evt_N6doUtPYh8i7iZxf | dev_jCuXYeFwCkZowpHs | 2023-04-19T13:22:44.194041Z | 2023-04-19T13:22:44.194041Z | 2023-04-19T13:22:44.263Z | 2023-04-19T13:22:44.263Z | {}                           |
| evt_idMGJImlICYP4dcy | dev_mHH1Cp4gPybCPR8y | 2023-04-19T13:26:37.030302Z | 2023-04-19T13:26:37.030302Z | 2023-04-19T13:26:37.080Z | 2023-04-19T13:26:37.080Z | {"requires-labeling":"true"} |
```

#### Foxglove extensions

With a Foxglove [Team plan](https://foxglove.dev/pricing), you can upload and share
[Foxglove extensions](https://foxglove.dev/docs/studio/extensions/getting-started)
within your organization.

Create and package an extension with the
[`foxglove-extension`](https://github.com/foxglove/create-foxglove-extension/)
tool. 

Publish an extension to install it for all Foxglove organization members:

```
$ foxglove extensions upload ./my-extension.1.0.0.foxe
```

You can use this same command to update an existing extension with a newer version. 
The last published version of an extension will be installed across your organization.

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

### Shell autocompletion

Certain shells (bash, zsh, fish, and PowerShell) support autocompletion for subcommands and certain parameters (like device IDs).

To enable this, consult your shell instructions under `$ foxglove completion <shell> -h`.
