package ros

import (
	"bytes"
	"errors"
	"io"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestBagLexer(t *testing.T) {
	buf := &bytes.Buffer{}
	writer, err := NewBagWriter(buf)
	assert.Nil(t, err)

	assert.Nil(t, writer.WriteConnection(&Connection{
		Conn:  1,
		Topic: "/foo",
		Data: ConnectionData{
			Topic:             "/foo",
			Type:              "type",
			MD5Sum:            "abc",
			MessageDefinition: []byte{0x01, 0x02},
		},
	}))

	for i := 0; i < 100000; i++ {
		err = writer.WriteMessage(&Message{
			Conn: 1,
			Data: []byte{0x01, 0x02},
		})
		assert.Nil(t, err)
	}

	assert.Nil(t, writer.Close())

	lexer, err := NewBagLexer(buf)
	assert.Nil(t, err)

	opcounts := map[OpCode]int{}
	for {
		opcode, _, err := lexer.Next()
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			t.Error(err)
		}
		opcounts[opcode]++
	}
	assert.Equal(t, map[OpCode]int{
		OpBagHeader:   1,
		OpConnection:  2,
		OpMessageData: 100000,
		OpIndexData:   2,
		OpChunkInfo:   2,
	}, opcounts)
}

func BenchmarkBagLexer(b *testing.B) {
	buf := &bytes.Buffer{}
	writer, err := NewBagWriter(buf)
	assert.Nil(b, err)
	assert.Nil(b, writer.WriteConnection(&Connection{
		Conn:  1,
		Topic: "/foo",
		Data: ConnectionData{
			Topic:             "/foo",
			Type:              "type",
			MD5Sum:            "abc",
			MessageDefinition: []byte{0x01, 0x02},
		},
	}))
	for i := 0; i < 1000000; i++ {
		err = writer.WriteMessage(&Message{
			Conn: 1,
			Data: []byte{0x01, 0x02},
		})
		assert.Nil(b, err)
	}
	assert.Nil(b, writer.Close())

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		reader := bytes.NewReader(buf.Bytes())
		lexer, err := NewBagLexer(reader)
		assert.Nil(b, err)
		for {
			_, _, err := lexer.Next()
			if err != nil {
				if errors.Is(err, io.EOF) {
					break
				}
				b.Error(err)
			}
		}
	}
}
