package main

import "github.com/foxglove/foxglove-cli/foxglove/cmd"

// build variables
var (
	Version string
)

func main() {
	cmd.Execute(Version)
}
