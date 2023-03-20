package ros

import (
	"bytes"
	"encoding/binary"
)

// Reference: http://wiki.ros.org/Bags/Format/2.0

// Magic is the magic number for ROS bag files.
var Magic = []byte("#ROSBAG V2.0\n")

// OpCode is a single-byte opcode identifying a record type. See the ROS bag
// spec for details.
type OpCode byte

const (
	// OpError is not in the bag spec. We return it only in cases where an error
	// value is also returned, rendering the opcode useless.
	OpError OpCode = 0x00

	// Bag header record: http://wiki.ros.org/Bags/Format/2.0#Bag_header
	OpBagHeader OpCode = 0x03

	// Chunk record: http://wiki.ros.org/Bags/Format/2.0#Chunk
	OpChunk OpCode = 0x05

	// Connection record: http://wiki.ros.org/Bags/Format/2.0#Connection
	OpConnection OpCode = 0x07

	// Message data record: http://wiki.ros.org/Bags/Format/2.0#Message_data
	OpMessageData OpCode = 0x02

	// Index data record: http://wiki.ros.org/Bags/Format/2.0#Index_data
	OpIndexData OpCode = 0x04

	// Chunk info record: http://wiki.ros.org/Bags/Format/2.0#Chunk_info
	OpChunkInfo OpCode = 0x06
)

// ChunkInfo represents the chunk info record. The "ver" field is omitted, and
// instead assumed to be 1. A ChunkInfo record is placed in the index section of
// a ROS bag, to allow a reader to easily locate chunks within the file by
// offset.
type ChunkInfo struct {
	ChunkPos  uint64
	StartTime uint64
	EndTime   uint64
	Count     uint32
	// Data is a map of connID to message count.
	Data map[uint32]uint32
}

// Message represents the message record. Message records are timestamped byte
// arrays associated with a connection via "connID". The byte array is expected
// to be decodable using the message_definition field of the connection record
// associated with "connID".
type Message struct {
	Conn uint32
	Time uint64
	Data []byte
}

// ConnectionData represents the data portion of a connection record.
type ConnectionData struct {
	Topic             string
	Type              string
	MD5Sum            string
	MessageDefinition []byte
	CallerID          *string
	Latching          *bool
}

// Connection represents the connection record.
type Connection struct {
	Conn  uint32
	Topic string
	Data  ConnectionData
}

// IndexData represents the index data record. The "ver" field is omitted and
// instead assumed to be 1.
type IndexData struct {
	Conn  uint32
	Count uint32
	Data  *bytes.Buffer
}

// BagHeader represents the bag header record.
type BagHeader struct {
	IndexPos   uint64
	ConnCount  uint32
	ChunkCount uint32
}

// copyUint32 copies a uint32 into a byte slice and returns the number of bytes
// copied. It is a convenience function for providing the output semantics of
// "copy" while writing integer data.
func copyUint32(buf []byte, x uint32) int {
	binary.LittleEndian.PutUint32(buf, x)
	return 4
}

func getHeaderValue(key string, header []byte) ([]byte, error) {
	offset := 0
	for offset < len(header) {
		fieldLen := binary.LittleEndian.Uint32(header[offset:])
		offset += 4
		fieldEnd := offset + int(fieldLen)
		separatorIdx := bytes.Index(header[offset:], []byte("="))
		fieldKey := string(header[offset : offset+separatorIdx])
		offset += separatorIdx + 1
		fieldValue := header[offset:fieldEnd]
		if fieldKey == key {
			return fieldValue, nil
		}
		offset = fieldEnd
	}

	return nil, ErrKeyNotFound
}
