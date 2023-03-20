package ros

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"

	"github.com/pierrec/lz4/v4"
)

var ErrKeyNotFound = fmt.Errorf("key not found")

type readerConfig struct {
	validateMagic bool
}

type ReaderOption func(*readerConfig)

// SkipMagic skips validation of the magic number during construction. This may
// be useful for parsing from the middle of a file.
func SkipMagic(skip bool) ReaderOption {
	return func(c *readerConfig) {
		c.validateMagic = skip
	}
}

// BagLexer is a basic scanner for ROS files.
type BagLexer struct {
	baseReader   io.Reader
	activeReader io.Reader
	chunkReader  *lz4.Reader
	inChunk      bool

	buf    []byte
	header []byte
	data   []byte
}

// NewBagLexer returns a new bag lexer.
func NewBagLexer(r io.Reader, opts ...ReaderOption) (*BagLexer, error) {
	config := readerConfig{
		validateMagic: true,
	}
	for _, opt := range opts {
		opt(&config)
	}

	if config.validateMagic {
		buf := make([]byte, len(Magic))
		_, err := io.ReadFull(r, buf)
		if err != nil {
			return nil, err
		}
	}

	return &BagLexer{
		baseReader:   r,
		activeReader: r,
		chunkReader:  lz4.NewReader(nil),
		buf:          make([]byte, 8),
	}, nil
}

// Next gets the next token from the bag reader.
func (b *BagLexer) Next() (OpCode, []byte, []byte, error) {
	for {
		headerLen, err := b.readUint32()
		if err != nil {
			// If we are in a chunk, and we get an EOF, exit the chunk and try
			// calling Next() on the base reader.
			if errors.Is(err, io.EOF) && b.inChunk {
				b.inChunk = false
				b.activeReader = b.baseReader
				continue
			}
			return OpError, nil, nil, err
		}

		if uint32(len(b.header)) < headerLen {
			b.header = make([]byte, headerLen)
		}

		_, err = io.ReadFull(b.activeReader, b.header[:headerLen])
		if err != nil {
			return OpError, nil, nil, err
		}

		opcodeValue, err := getHeaderValue("op", b.header)
		if err != nil {
			return OpError, nil, nil, err
		}

		opcode := OpCode(opcodeValue[0])
		if opcode == OpChunk {
			err = b.loadChunk(b.header[:headerLen])
			if err != nil {
				return OpError, nil, nil, err
			}
			continue
		}

		// otherwise, read the data
		dataLen, err := b.readUint32()
		if err != nil {
			return OpError, nil, nil, err
		}
		if uint32(len(b.data)) < dataLen {
			b.data = make([]byte, dataLen)
		}
		_, err = io.ReadFull(b.activeReader, b.data[:dataLen])
		if err != nil {
			return OpError, nil, nil, err
		}
		return opcode, b.header, b.data, nil
	}
}

func (b *BagLexer) readUint32() (uint32, error) {
	_, err := b.activeReader.Read(b.buf[:4])
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint32(b.buf), nil
}

func (b *BagLexer) loadChunk(header []byte) error {
	compression, err := getHeaderValue("compression", header)
	if err != nil {
		return err
	}
	if string(compression) != "lz4" {
		return fmt.Errorf("unsupported compression")
	}
	dataLen, err := b.readUint32()
	if err != nil {
		return err
	}
	b.chunkReader.Reset(io.LimitReader(b.baseReader, int64(dataLen)))
	b.activeReader = b.chunkReader
	b.inChunk = true
	return nil
}
