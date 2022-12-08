## Foxglove CLI

The Foxglove CLI tool enables command line interaction with remotely stored
data in Foxglove Data Platform.

### Installation

#### Install a release

To download the latest release, use one of the following commands according to
your OS and architecture:

| OS/Arch     |                                                                                                            |
|--------|------------------------------------------------------------------------------------------------------------|
| linux/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-amd64 -o foxglove |
| darwin/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-darwin-amd64 -o foxglove |
| windows/amd64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-amd64.exe -o foxglove.exe |
| linux/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-arm64 -o foxglove |
| darwin/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-darwin-arm64 -o foxglove |
| windows/arm64    | curl -L https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-arm64.exe -o foxglove.exe |


To install a specific release, see the [releases
page](https://github.com/foxglove/foxglove-cli/releases).

#### Install from source
Installation from source requires the following:
* Go >= 1.18

To install from source, run

    git clone git@github.com:foxglove/foxglove-cli.git
    make install

This will install `foxglove` to `$GOPATH/bin`.

### Usage

See `foxglove -h` for complete usage documentation.

#### Authenticating

Before interacting with the data platform it is necessary to authenticate:

```
foxglove auth login
```

#### Configuration with an API key

As an alternative to interactive login, the tool can be configured to use a
foxglove API key:

```
foxglove auth configure-api-key
```

This will overwrite any previously set credential. The API key should be
configured in the Foxglove console with whatever capabilities you intend to
use, for instance `data.upload` (for importing data) or `data.stream` (for
exporting).

#### Importing data

Before importing data it is necessary to add a device:

```
foxglove devices add --name "my device"
Device created: dev_drpLqjBZYUzus3gv
```

To load data into the platform, use `foxglove data imports add`. The platform
accepts imports in ros1 bag or mcap format:

```
foxglove data imports add ~/data/bags/gps.bag --device-id dev_drpLqjBZYUzus3gv
```

This is equivalent to the shorthand, 
```
foxglove data import ~/data/bags/gps.bag --device-id dev_drpLqjBZYUzus3gv
```

Imports can be listed with the CLI as well:

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

#### Devices

The CLI tool supports listing devices for the org, and adding new ones:

```
$ foxglove devices list
|          ID          |        NAME        |      CREATED AT      |      UPDATED AT      |
|----------------------|--------------------|----------------------|----------------------|
| dev_mHH1Cp4gPybCPR8y | Adrian's Robot     | 2021-10-28T17:20:55Z | 2021-10-28T17:20:55Z |
| dev_WEJUVEOVApoIpe1M | GPS                | 2021-11-01T17:38:55Z | 2021-11-01T17:38:55Z |
| dev_drpLqjBZYUzus3gv | updog              | 2021-11-25T02:22:45Z | 2021-11-25T02:22:45Z |
| dev_lwjzOMxryMmP3yXg | nuScenes-v1.0-mini | 2021-12-09T21:45:51Z | 2021-12-09T21:45:51Z |
```

```
foxglove devices add --name "my device"
Device created: dev_drpLqjBZYUzus3gv
```

#### Events
The tool supports creating and searching events:

```
$ foxglove beta events list
|          ID          |      DEVICE ID       |      TIMESTAMP      |  DURATION   |        CREATED AT        |        UPDATED AT        |                                 METADATA                                 |
|----------------------|----------------------|---------------------|-------------|--------------------------|--------------------------|--------------------------------------------------------------------------|
| evt_PbkdOL8geyyXLIOr | dev_JtSXCGiM0RC2YHDO | 1532402927000000000 | 0           | 2022-02-24T15:22:25.154Z | 2022-02-24T15:22:25.154Z | {"position":"start"}                                                     |
| evt_U0TCTtRbDb631tmI | dev_mHH1Cp4gPybCPR8y | 1532402927000000000 | 2000000000  | 2022-02-09T21:11:52.637Z | 2022-02-09T21:11:52.637Z | {"nuscene":"0061","startup":"yes"}                                       |
| evt_9akLLRJRqp8jg1zq | dev_mHH1Cp4gPybCPR8y | 1532402927000000000 | 19000000000 | 2022-02-09T21:08:46.936Z | 2022-02-09T21:08:46.936Z | {"location":"singapore","nuscene":"0061","weather-laguna":"really-nice"} |
| evt_0A1LUzWBKkLeDoIa | dev_mHH1Cp4gPybCPR8y | 1532402937000000000 | 0           | 2022-02-09T21:19:18.488Z | 2022-02-09T21:19:18.488Z | {"nuscene":"0061","random":"yes"}                                        |
| evt_Nx6Nx9WoEMPhDeRb | dev_mHH1Cp4gPybCPR8y | 1532402937000000000 | 0           | 2022-02-09T22:08:51.249Z | 2022-02-09T22:08:51.249Z | {"location":"🇸🇬"}                                                         |
| evt_zZ8s6al5F3CUYVtJ | dev_mHH1Cp4gPybCPR8y | 1532402937000000000 | 0           | 2022-02-09T22:09:49.412Z | 2022-02-09T22:09:49.412Z | {"weather":"🌧"}                                                          |
| evt_zEVu8NABeHZdABML | dev_mHH1Cp4gPybCPR8y | 1644443695000000000 | 0           | 2022-02-09T21:57:29.064Z | 2022-02-09T21:57:29.064Z | {}                                                                       |
| evt_QJtL4x6701tFhKia | dev_mHH1Cp4gPybCPR8y | 1645047149000000000 | 0           | 2022-02-16T21:35:07.352Z | 2022-02-16T21:35:07.352Z | {"happy":"valley"}                                                       |
| evt_kRIWfP2GgjCbLejp | dev_Wm1gvryKJmREqnVT | 1645483936331000000 | 17000000000 | 2022-02-21T23:19:10.521Z | 2022-02-21T23:19:10.521Z | {"calibration":"camera"}                                                 |
| evt_J69qtmDyKtWmYZTb | dev_mHH1Cp4gPybCPR8y | 1645554501000000000 | 0           | 2022-02-23T18:28:50.228Z | 2022-02-23T18:28:50.228Z | {"color":"green","user":"adrian"}                                        |
| evt_N6doUtPYh8i7iZxf | dev_jCuXYeFwCkZowpHs | 1646147056000000000 | 23000000000 | 2022-03-04T15:04:31.546Z | 2022-03-04T15:04:31.546Z | {"a":"13"}                                                               |
| evt_idMGJImlICYP4dcy | dev_mHH1Cp4gPybCPR8y | 1646248453000000000 | 60          | 2022-03-02T19:14:39.117Z | 2022-03-02T19:14:39.117Z | {"requires-labeling":"true"}                                             |
```

For adding events, see `foxglove beta events add -h`.

#### Querying data
```
$ foxglove data export --device-id dev_drpLqjBZYUzus3gv --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format mcap0 --topics /gps/fix,/gps/fix_velocity > output.mcap

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
```

Query data as a ROS bag:

```
$ foxglove data export --device-id dev_drpLqjBZYUzus3gv --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format bag1 --topics /gps/fix,/gps/fix_velocity > output.bag
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

Output data to the console as JSON:

```
$ foxglove data export --device-id dev_flm75pLkfzUBX2DH --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --topics /tf --output-format json | head -n 5
{"topic":"/tf","sequence":0,"log_time":1490149580.103843113,"publish_time":1490149580.103843113,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.117017840,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
{"topic":"/tf","sequence":0,"log_time":1490149580.113944947,"publish_time":1490149580.113944947,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.127078895,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
{"topic":"/tf","sequence":0,"log_time":1490149580.124028613,"publish_time":1490149580.124028613,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.137141823,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
{"topic":"/tf","sequence":0,"log_time":1490149580.134219155,"publish_time":1490149580.134219155,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.147199242,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
{"topic":"/tf","sequence":0,"log_time":1490149580.144292780,"publish_time":1490149580.144292780,"data":{"transforms":[{"header":{"seq":0,"stamp":1490149580.157286100,"frame_id":"base_link"},"child_frame_id":"radar","transform":{"translation":{"x":3.835,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1}}}]}}
```

#### Studio extensions

With a paid foxglove account, you can upload
[Studio extensions](https://foxglove.dev/docs/studio/extensions/getting-started)
to share with your organization.

Extensions are created and packaged with the
[foxglove-extension](https://github.com/foxglove/create-foxglove-extension/)
tool. The latest version of each uploaded extension will be installed in Studio
for all organization members.

To publish a new extension, or update one with a newer version:

```
foxglove extensions upload ./my-extension.1.0.0.foxe
```

To list your extensions:

```
foxglove extensions list
```

Example JSON output:

```json
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

You can use the global `--format` flag to change the output type.

To unpublish an extension, use the ID listed from the above command. This will
**delete** your extension files and cause the extension to be uninstalled from
Studio for all organization members.

```
foxglove extensions unpublish ext_BsGXKGsZ9c4WQF1
```

### Shell autocompletion

The foxglove tool supports shell autocompletion for subcommands and some kinds
of parameters (such as device IDs). To enable this, consult the instructions
for your shell under `foxglove completion <shell> -h`. Supported shells are
bash, zsh, fish, and PowerShell.
