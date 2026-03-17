package util

import (
	"fmt"
	"strconv"
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

// ParsePropertyValue attempts to interpret a string as a bool, number, or falls back to string.
func ParsePropertyValue(val string) interface{} {
	if b, err := strconv.ParseBool(val); err == nil {
		return b
	}
	if f, err := strconv.ParseFloat(val, 64); err == nil {
		return f
	}
	return val
}
