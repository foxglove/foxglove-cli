package ros

import (
	"errors"
	"fmt"
	"io"

	"github.com/foxglove/mcap/go/ros"
)

type BagReader struct {
	r     io.ReadSeeker
	lexer *BagLexer
	buf   []byte
}

type Info struct {
	MessageEndTime uint64
	MessageCount   uint64
}

func (r *BagReader) Info() (*Info, error) {
	_, err := r.r.Seek(int64(len(ros.BagMagic)), io.SeekStart)
	if err != nil {
		return nil, fmt.Errorf("failed to seek to file start: %w", err)
	}
	op, record, err := r.lexer.Next()
	if err != nil {
		return nil, fmt.Errorf("failed to read bag header: %w", err)
	}
	if op != OpBagHeader {
		return nil, fmt.Errorf("unexpected op: %v. want bag header.", op)
	}
	bagHeader, err := ParseBagHeader(record)
	if err != nil {
		return nil, fmt.Errorf("failed to parse bag header: %w", err)
	}
	if bagHeader.IndexPos == 0 {
		return nil, fmt.Errorf("bag index position is 0")
	}
	_, err = r.r.Seek(int64(bagHeader.IndexPos), io.SeekStart)
	if err != nil {
		return nil, fmt.Errorf("failed to seek to bag index: %w", err)
	}
	// read through the connection records
	// After the connection records we should have chunk info records. Need to
	// scan through these to figure out the max message time.
	var maxEndTime uint64
	var messageCount uint64
	for {
		op, record, err = r.lexer.Next()
		if err != nil && !errors.Is(err, io.EOF) {
			return nil, fmt.Errorf("failed to read bag index: %w", err)
		}
		switch op {
		case OpConnection:
			continue
		case OpChunkInfo:
			chunkInfo, err := ParseChunkInfo(record)
			if err != nil {
				return nil, fmt.Errorf("failed to parse chunk info: %w", err)
			}
			if chunkInfo.EndTime > maxEndTime {
				maxEndTime = chunkInfo.EndTime
			}
			for _, count := range chunkInfo.Data {
				messageCount += uint64(count)
			}
		default:
			return &Info{
				MessageEndTime: maxEndTime,
				MessageCount:   messageCount,
			}, nil
		}
	}
}

func NewBagReader(rs io.ReadSeeker) (*BagReader, error) {
	lexer, err := NewBagLexer(rs)
	if err != nil {
		return nil, err
	}
	return &BagReader{
		r:     rs,
		buf:   make([]byte, 8),
		lexer: lexer,
	}, nil
}
