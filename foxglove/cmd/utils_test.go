package cmd

import (
	"bytes"
	"strings"
	"testing"

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
