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
* Go >= 1.17

To install from source, run

    git clone git@github.com:foxglove/foxglove-cli.git
    make install

### Usage

See `foxglove -h` for complete usage documentation.

1. Authenticate to Foxglove
```
foxglove auth login
```

2. Import data

```
foxglove data import ~/data/bags/gps.bag --device-id dev_flm75pLkfzUBX2DH
```

3. List available devices for querying

```
foxglove devices list
|          ID          |        NAME        |      CREATED AT      |      UPDATED AT      |
|----------------------|--------------------|----------------------|----------------------|
| dev_mHH1Cp4gPybCPR8y | Adrian's Robot     | 2021-10-28T17:20:55Z | 2021-10-28T17:20:55Z |
| dev_WEJUVEOVApoIpe1M | GPS                | 2021-11-01T17:38:55Z | 2021-11-01T17:38:55Z |
| dev_flm75pLkfzUBX2DH | updog              | 2021-11-25T02:22:45Z | 2021-11-25T02:22:45Z |
| dev_lwjzOMxryMmP3yXg | nuScenes-v1.0-mini | 2021-12-09T21:45:51Z | 2021-12-09T21:45:51Z |
```

4. Query imported data

```
$ foxglove data export --device-id dev_flm75pLkfzUBX2DH --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format mcap0 --topics /gps/fix,/gps/fix_velocity > output.bag

$ rosbag reindex output.bag

$ rosbag info output.bag
  path:         output.bag
  version:      2.0
  duration:     5:39s (339s)
  start:        Mar 22 2021 08:03:38.47 (1616425418.47)
  end:          Mar 22 2021 08:09:17.78 (1616425757.78)
  size:         330.2 KB
  messages:     6728
  compression:  lz4 [1/1 chunks; 12.96%]
  uncompressed:   1.8 MB @ 5.5 KB/s
  compressed:   241.6 KB @ 0.7 KB/s (12.96%)
  types:        geometry_msgs/TwistWithCovarianceStamped [8927a1a12fb2607ceea095b2dc440a96]
                sensor_msgs/NavSatFix                    [2d3a8cd499b9b4a0249fb98fd05cfa48]
  topics:       /gps/fix            3364 msgs    : sensor_msgs/NavSatFix
                /gps/fix_velocity   3364 msgs    : geometry_msgs/TwistWithCovarianceStamped
```

### Shell autocompletion

The foxglove tool supports shell autocompletion for subcommands and some kinds
of parameters (such as device IDs). To enable this, consult the instructions
for your shell under `foxglove completion <shell> -h`. Supported shells are
bash, zsh, fish, and PowerShell.
