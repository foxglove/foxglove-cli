## Foxglove CLI

The `foxglove` CLI tool enables command line interaction with remotely stored
data in Foxglove Data Platform.

The tool is currently in development and lacks a packaging pipeline. To get
started, install using go:

    git clone git@github.com:foxglove/foxglove-cli.git
    make install

### Usage

1. Authenticate to Foxglove
```
foxglove login
```

2. Import data

```
foxglove import --filename ~/data/bags/gps.bag --device-id dev_flm75pLkfzUBX2DH
```

3. Query data

```
$ foxglove export --device-id dev_flm75pLkfzUBX2DH --start 2001-01-01T00:00:00Z --end 2022-01-01T00:00:00Z --output-format bag1 --topics /gps/fix,/gps/fix_velocity > output.bag

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

See `foxglove -h` for additional usage details.
