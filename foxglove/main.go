package main

import "github.com/foxglove/foxglove-cli/foxglove/cmd"

// build variables
var (
	GitCommit string
)

func main() {
	cmd.Execute(GitCommit)
}
