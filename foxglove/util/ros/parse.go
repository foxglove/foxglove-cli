package ros

import (
	"bytes"
	"encoding/binary"
	"fmt"
)

func parseROSTime(data []byte) uint64 {
	secs := uint64(binary.LittleEndian.Uint32(data[:4]))
	nsecs := uint64(binary.LittleEndian.Uint32(data[4:]))
	return secs*1e9 + nsecs
}

func readHeader(buf []byte) map[string][]byte {
	result := make(map[string][]byte)
	offset := 0
	for offset < len(buf) {
		fieldLength := binary.LittleEndian.Uint32(buf[offset:])
		offset += 4
		separatorIdx := bytes.Index(buf[offset:], []byte{'='})
		key := string(buf[offset : offset+separatorIdx])
		value := buf[offset+separatorIdx+1 : offset+int(fieldLength)]
		result[key] = value
		offset += int(fieldLength)
	}
	return result
}

func ParseBagHeader(record []byte) (*BagHeader, error) {
	headerLength := int(binary.LittleEndian.Uint32(record))
	headerPortion := record[4 : 4+headerLength]
	header := readHeader(headerPortion)
	indexPos, ok := header["index_pos"]
	if !ok {
		return nil, fmt.Errorf("index_pos not found")
	}
	connCount, ok := header["conn_count"]
	if !ok {
		return nil, fmt.Errorf("conn_count not found")
	}
	chunkCount, ok := header["chunk_count"]
	if !ok {
		return nil, fmt.Errorf("chunk_count not found")
	}
	return &BagHeader{
		IndexPos:   binary.LittleEndian.Uint64(indexPos),
		ConnCount:  binary.LittleEndian.Uint32(connCount),
		ChunkCount: binary.LittleEndian.Uint32(chunkCount),
	}, nil
}

func ParseConnection(record []byte) (*Connection, error) {
	offset := 0
	headerLength := int(binary.LittleEndian.Uint32(record[offset:]))
	offset += 4
	header := readHeader(record[offset : offset+headerLength])
	offset += headerLength
	dataLength := int(binary.LittleEndian.Uint32(record[offset:]))
	offset += 4
	data := readHeader(record[offset : offset+dataLength])
	var callerID *string
	if v, ok := data["callerid"]; ok {
		s := string(v)
		callerID = &s
	}
	var latching *bool
	if v, ok := data["latching"]; ok {
		var value bool
		if string(v) == "1" {
			value = true
		} else {
			value = false
		}
		latching = &value
	}
	return &Connection{
		Conn:  binary.LittleEndian.Uint32(header["conn"]),
		Topic: string(header["topic"]),
		Data: ConnectionData{
			Topic:             string(data["topic"]),
			Type:              string(data["type"]),
			MD5Sum:            string(data["md5sum"]),
			MessageDefinition: data["message_definition"],
			CallerID:          callerID,
			Latching:          latching,
		},
	}, nil
}

func ParseMessage(record []byte) (*Message, error) {
	offset := 0
	headerLength := int(binary.LittleEndian.Uint32(record[offset:]))
	offset += 4
	header := readHeader(record[offset : offset+headerLength])
	offset += headerLength
	return &Message{
		Conn: binary.LittleEndian.Uint32(header["conn"]),
		Time: parseROSTime(header["time"]),
		Data: record[offset+4:], // skip the 4-byte length prefix
	}, nil
}

func ParseChunkInfo(record []byte) (*ChunkInfo, error) {
	offset := 0
	headerLength := int(binary.LittleEndian.Uint32(record[offset:]))
	offset += 4
	header := readHeader(record[offset : offset+headerLength])
	offset += headerLength
	dataLength := int(binary.LittleEndian.Uint32(record[offset:]))
	offset += 4
	dataEnd := offset + dataLength
	data := make(map[uint32]uint32)
	for offset < dataEnd {
		connID := binary.LittleEndian.Uint32(record[offset:])
		offset += 4
		count := binary.LittleEndian.Uint32(record[offset:])
		offset += 4
		data[connID] = count
	}
	return &ChunkInfo{
		ChunkPos:  binary.LittleEndian.Uint64(header["chunk_pos"]),
		StartTime: parseROSTime(header["start_time"]),
		EndTime:   parseROSTime(header["end_time"]),
		Count:     binary.LittleEndian.Uint32(header["count"]),
		Data:      data,
	}, nil
}
