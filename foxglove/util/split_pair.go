package util

import (
	"fmt"
	"strconv"
	"strings"
)

// Split a key/value string on a given delimiter into a pair
func SplitPair(kv string, delim rune) (key string, value string, err error) {
	parts := strings.SplitN(kv, string(delim), 2)
	if len(parts) != 2 || parts[0] == "" {
		return "", "", fmt.Errorf("invalid key/value pair: %s", kv)
	}
	return parts[0], parts[1], nil
}

// ParsePropertyValue attempts to interpret a string as a bool, number, or falls back to string.
func ParsePropertyValue(val string) interface{} {
	if strings.EqualFold(val, "true") || strings.EqualFold(val, "false") {
		b, _ := strconv.ParseBool(val)
		return b
	}
	if f, err := strconv.ParseFloat(val, 64); err == nil {
		return f
	}
	return val
}
