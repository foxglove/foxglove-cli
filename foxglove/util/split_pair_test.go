package util

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSplitPair(t *testing.T) {
	t.Run("splits simple key:value", func(t *testing.T) {
		k, v, err := SplitPair("foo:bar", ':')
		assert.Nil(t, err)
		assert.Equal(t, "foo", k)
		assert.Equal(t, "bar", v)
	})

	t.Run("preserves colons in value", func(t *testing.T) {
		k, v, err := SplitPair("url:https://example.com", ':')
		assert.Nil(t, err)
		assert.Equal(t, "url", k)
		assert.Equal(t, "https://example.com", v)
	})

	t.Run("allows empty value", func(t *testing.T) {
		k, v, err := SplitPair("key:", ':')
		assert.Nil(t, err)
		assert.Equal(t, "key", k)
		assert.Equal(t, "", v)
	})

	t.Run("rejects missing delimiter", func(t *testing.T) {
		_, _, err := SplitPair("nocolon", ':')
		assert.NotNil(t, err)
	})

	t.Run("rejects empty key", func(t *testing.T) {
		_, _, err := SplitPair(":value", ':')
		assert.NotNil(t, err)
	})
}

func TestParsePropertyValue(t *testing.T) {
	tests := []struct {
		input    string
		expected interface{}
	}{
		{"true", true},
		{"false", false},
		{"True", true},
		{"FALSE", false},
		{"0", float64(0)},
		{"1", float64(1)},
		{"42", float64(42)},
		{"3.14", float64(3.14)},
		{"-1", float64(-1)},
		{"hello", "hello"},
		{"", ""},
		{"t", "t"},
		{"f", "f"},
		{"yes", "yes"},
		{"no", "no"},
	}
	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			result := ParsePropertyValue(tt.input)
			assert.Equal(t, tt.expected, result)
		})
	}
}
