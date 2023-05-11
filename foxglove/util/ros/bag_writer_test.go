package ros

import (
	"bytes"
	"crypto/md5"
	"errors"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestProduceSimpleOutputBag(t *testing.T) {
	cases := []struct {
		assertion string
	}{
		{
			"basic bag",
		},
	}

	t.Cleanup(func() {
		os.Remove("test.bag")
	})

	for _, c := range cases {
		t.Run(c.assertion, func(t *testing.T) {
			f, err := os.Create("test.bag")
			assert.Nil(t, err)
			defer f.Close()
			writer, err := NewBagWriter(f)
			assert.Nil(t, err)
			assert.Nil(t, writer.WriteConnection(&Connection{
				Conn:  0,
				Topic: "/foo",
				Data: ConnectionData{
					Topic:             "/foo",
					Type:              "123",
					MD5Sum:            "abc",
					MessageDefinition: []byte{0x01, 0x02},
				},
			}))
			for i := 0; i < 400000; i++ {
				assert.Nil(t, writer.WriteMessage(&Message{
					Conn: 0,
					Time: uint64(i),
					Data: []byte{0x01, 0x02, 0x03},
				}))
			}
			assert.Nil(t, writer.Close())
		})
	}
}

func TestBagWriter(t *testing.T) {
	cases := []struct {
		assertion        string
		inputConnections []Connection
		inputMessages    []Message
		outputTokens     []OpCode
	}{
		{
			"empty bag",
			[]Connection{},
			[]Message{},
			[]OpCode{OpBagHeader},
		},
		{
			"one connection, one message",
			[]Connection{
				connection(1, "/foo"),
			},
			[]Message{
				message(1, 0, []byte{0x01, 0x02, 0x03}),
			},
			[]OpCode{OpBagHeader, OpConnection, OpMessageData, OpIndexData, OpConnection, OpChunkInfo},
		},
		{
			"one connection, no messages",
			[]Connection{
				connection(1, "/foo"),
			},
			[]Message{},
			[]OpCode{OpBagHeader, OpConnection, OpConnection, OpChunkInfo},
		},
	}
	for _, c := range cases {
		t.Run(c.assertion, func(t *testing.T) {
			buf := &bytes.Buffer{}
			writer, err := NewBagWriter(buf, WithChunksize(2048))
			if err != nil {
				t.Error(err)
			}
			for _, connection := range c.inputConnections {
				assert.Nil(t, writer.WriteConnection(&connection))
			}
			for _, message := range c.inputMessages {
				assert.Nil(t, writer.WriteMessage(&message))
			}
			writer.Close()

			lexer, err := NewBagLexer(bytes.NewReader(buf.Bytes()))
			assert.Nil(t, err)

			opcodes := []OpCode{}
			for {
				opcode, _, err := lexer.Next()
				if err != nil {
					if errors.Is(err, io.EOF) {
						break
					}
					t.Error(err)
				}
				opcodes = append(opcodes, opcode)
			}
			assert.Equal(t, c.outputTokens, opcodes)
		})
	}
}

func TestBagWriterHashesDeterministically(t *testing.T) {
	hashes := []string{}

	iterations := 20

	for i := 0; i < iterations; i++ {
		buf := &bytes.Buffer{}
		writer, err := NewBagWriter(buf, WithChunksize(2048))
		assert.Nil(t, err)

		for connID := uint32(0); connID < 5; connID++ {
			assert.Nil(t, writer.WriteConnection(&Connection{
				Conn:  connID,
				Topic: fmt.Sprintf("/foo-%d", connID),
				Data: ConnectionData{
					Topic:             "/foo",
					Type:              "123",
					MD5Sum:            "abc",
					MessageDefinition: []byte{0x01, 0x02},
				},
			}))
		}

		for j := uint32(0); j < 1000; j++ {
			assert.Nil(t, writer.WriteMessage(&Message{
				Conn: j % 5,
				Time: uint64(j),
				Data: []byte{0x01, 0x02, 0x03},
			}))
		}

		assert.Nil(t, writer.Close())
		hashes = append(hashes, fmt.Sprintf("%x", md5.Sum(buf.Bytes())))
	}

	assert.Equal(t, iterations, len(hashes))
	for i := 1; i < iterations; i++ {
		assert.Equal(t, hashes[0], hashes[i])
	}
}

func BenchmarkBagWriter(b *testing.B) {
	for i := 0; i < b.N; i++ {
		f, err := os.Create("test.bag")
		assert.Nil(b, err)
		writer, err := NewBagWriter(f)
		assert.Nil(b, err)
		assert.Nil(b, writer.WriteConnection(&Connection{
			Conn:  0,
			Topic: "/foo",
			Data: ConnectionData{
				Topic:             "/foo",
				Type:              "123",
				MD5Sum:            "abc",
				MessageDefinition: []byte{0x01, 0x02},
			},
		}))

		data := make([]byte, 1000)
		for i := 0; i < 100000; i++ {
			assert.Nil(b, writer.WriteMessage(&Message{
				Conn: 0,
				Time: 1000,
				Data: data,
			}))
		}

		assert.Nil(b, writer.Close())
		assert.Nil(b, f.Close())
		b.ReportAllocs()
	}
}
