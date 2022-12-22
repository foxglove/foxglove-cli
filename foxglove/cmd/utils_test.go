package cmd

import (
	"bytes"
	"strings"
	"testing"

	"github.com/foxglove/mcap/go/mcap"
	"github.com/stretchr/testify/assert"
)

type TestRecord struct {
	A string `json:"a"`
	B string `json:"b"`
}

func (r TestRecord) Fields() []string {
	return []string{r.A, r.B}
}

func (r TestRecord) Headers() []string {
	return []string{"a", "b"}
}

func TestValidateImportLooksLegal(t *testing.T) {
	t.Run("accepts a valid mcap file", func(t *testing.T) {
		buf := &bytes.Buffer{}
		w, err := mcap.NewWriter(buf, &mcap.WriterOptions{})
		assert.Nil(t, err)
		assert.Nil(t, w.Close())
		reader := bytes.NewReader(buf.Bytes())
		assert.Nil(t, validateImportLooksLegal(reader))
	})
	t.Run("rejects a truncated mcap file", func(t *testing.T) {
		buf := &bytes.Buffer{}
		w, err := mcap.NewWriter(buf, &mcap.WriterOptions{})
		assert.Nil(t, err)
		assert.Nil(t, w.Close())
		reader := bytes.NewReader(buf.Bytes()[:len(mcap.Magic)+1])
		assert.ErrorIs(t, validateImportLooksLegal(reader), ErrTruncatedMCAP)
	})
	t.Run("accepts a bag file", func(t *testing.T) {
		reader := bytes.NewReader([]byte("#ROSBAG V2.0\n"))
		assert.Nil(t, validateImportLooksLegal(reader))
	})
	t.Run("rejects a file that is neither bag nor mcap", func(t *testing.T) {
		reader := bytes.NewReader(make([]byte, 10))
		assert.ErrorIs(t, validateImportLooksLegal(reader), ErrInvalidInput)
	})
}

func TestRenderCSV(t *testing.T) {
	records := []TestRecord{
		{A: "a", B: "b"},
		{A: "c", B: "d"},
	}
	buf := &bytes.Buffer{}
	err := renderCSV(buf, records)
	assert.Nil(t, err)
	assert.Equal(t, "a,b\na,b\nc,d\n", buf.String())
}
func TestRenderJSON(t *testing.T) {
	records := []TestRecord{
		{A: "a", B: "b"},
		{A: "c", B: "d"},
	}
	buf := &bytes.Buffer{}
	err := renderJSON(buf, records)
	assert.Nil(t, err)
	assert.Equal(t, `[{"a":"a","b":"b"},{"a":"c","b":"d"}]`,
		strings.ReplaceAll(
			strings.ReplaceAll(strings.TrimSpace(buf.String()), " ", ""),
			"\n", ""))
}

func TestRenderTable(t *testing.T) {
	records := []TestRecord{
		{A: "a", B: "b"},
		{A: "c", B: "d"},
	}
	buf := &bytes.Buffer{}
	renderTable(buf, records)
	assert.Equal(t, "| A | B |\n|---|---|\n| a | b |\n| c | d |\n", buf.String())
}

func TestRenderList(t *testing.T) {
	records := []TestRecord{
		{A: "a", B: "b"},
		{A: "c", B: "d"},
	}
	cases := []struct {
		assertion string
		format    string
	}{
		{
			"table",
			"table",
		},
		{
			"json",
			"json",
		},
		{
			"csv",
			"csv",
		},
	}
	for _, c := range cases {
		t.Run(c.assertion, func(t *testing.T) {
			buf := &bytes.Buffer{}
			err := renderList(buf, nil, func(any) ([]TestRecord, error) { return records, nil }, c.format)
			assert.Nil(t, err)
		})
	}
}
