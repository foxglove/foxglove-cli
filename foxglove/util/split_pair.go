package util

import (
	"fmt"
	"strings"
)

// Split a key/value string on a given delimiter into a pair
func SplitPair(kv string, delim rune) (key string, value string, err error) {
	parts := strings.FieldsFunc(kv, func(c rune) bool { return c == delim })
	if len(parts) != 2 {
		return "", "", fmt.Errorf("invalid key/value pair: %s", kv)
	}
	return parts[0], parts[1], nil
}
