package ros

import (
	"bytes"
	"encoding/binary"
	"io"
	"math"
	"sort"

	"github.com/foxglove/foxglove-cli/foxglove/util"
	"github.com/pierrec/lz4/v4"
)

// BagWriter is a basic writer for ROS bag files, exposing to the user the
// ability to write message and connection records, while handling indexing and
// chunk compression internally.
type BagWriter struct {
	w   io.Writer
	out *util.CountingWriter

	chunkBuffer *bytes.Buffer

	chunkWriter *lz4.Writer

	header []byte
	buf8   []byte

	config       bagWriterConfig
	currentChunk currentChunkStats
	outputStats  outputStats
}

type currentChunkStats struct {
	startTime        uint64
	endTime          uint64
	decompressedSize uint32
	messageCount     uint32
	indexData        map[uint32]*IndexData
}

type outputStats struct {
	chunkInfo   []ChunkInfo
	connections []Connection
}

// NewBagWriter constructs a new bag writer. The bag writer implements the ROS
// bag specification, with chunk compression and indexing.
func NewBagWriter(w io.Writer, opts ...BagWriterOption) (*BagWriter, error) {
	// default configuration values
	config := bagWriterConfig{
		chunksize:   4 * 1024 * 1024,
		skipHeader:  false,
		compression: "lz4",
	}

	// apply config overrides
	for _, opt := range opts {
		opt(&config)
	}

	// pre-allocate a buffer of the requested chunksize, to avoid repeatedly
	// expanding the size of the buffer.
	chunkBuffer := bytes.NewBuffer(make([]byte, 0, config.chunksize))
	chunkBuffer.Reset()

	// create a new lz4 writer, which will write to the chunk buffer. lz4 is the
	// only supported compression option today.
	chunkWriter := lz4.NewWriter(chunkBuffer)

	out := util.NewCountingWriter(w)

	writer := &BagWriter{
		w:           w,
		out:         out,
		chunkBuffer: chunkBuffer,
		chunkWriter: chunkWriter,
		buf8:        make([]byte, 8),
		config:      config,
		currentChunk: currentChunkStats{
			endTime:   0,
			startTime: math.MaxUint64,
			indexData: make(map[uint32]*IndexData),
		},
	}

	// write the bag header, if requested.
	if !config.skipHeader {
		_, err := out.Write(Magic)
		if err != nil {
			return nil, err
		}

		// Bag header is initially written empty. This must be filled in after
		// the bag is finalized and the location of the index is known. If the
		// input writer implements io.WriteSeeker, this will be handled by the
		// bag writer. Otherwise if no seeking is possible, this will need to be
		// done out of band via "rosbag reindex" or a similar mechanism.
		err = writer.writeBagHeader(BagHeader{})
		if err != nil {
			return nil, err
		}
	}

	return writer, nil
}

// WriteConnection writes a connection record to the output. A connection record
// should be written prior to any messages on that connection. This is _not_
// enforced by the library, in order to support writing messages to an existing
// partial file. See http://wiki.ros.org/Bags/Format/2.0#Connection for
// additional detail.
func (b *BagWriter) WriteConnection(connection Connection) error {
	n, err := b.writeConnection(b.chunkWriter, connection)
	if err != nil {
		return err
	}
	b.currentChunk.decompressedSize += uint32(n)
	b.outputStats.connections = append(b.outputStats.connections, connection)
	return nil
}

// WriteMessage writes a message data record to the bag file. See
// http://wiki.ros.org/Bags/Format/2.0#Message_data for additional detail.
func (b *BagWriter) WriteMessage(message Message) error {
	// if the current chunk exceeds the requested chunk size, flush it to the
	// output and start a new chunk.
	if b.currentChunk.decompressedSize > uint32(b.config.chunksize) {
		err := b.flushActiveChunk()
		if err != nil {
			return err
		}
	}

	// build the record header
	binary.LittleEndian.PutUint32(b.buf8, message.Conn)
	logTime := rostime(message.Time)
	header := b.buildHeader(&b.header,
		[]byte("conn"), b.buf8[:4],
		[]byte("time"), logTime,
		[]byte("op"), []byte{byte(OpMessageData)},
	)

	// if this is the first message on the connection, create an index data
	// entry to maintain connection statistics. The index data entry in this map
	// will be transformed into an index data record in the output, when the
	// chunk is finalized.
	indexData, ok := b.currentChunk.indexData[message.Conn]
	if !ok {
		indexData = &IndexData{
			Conn:  message.Conn,
			Data:  &bytes.Buffer{},
			Count: 0,
		}
		b.currentChunk.indexData[message.Conn] = indexData
		indexData = b.currentChunk.indexData[message.Conn]
	}

	// increment the message count for this connection, within the current chunk
	indexData.Count++

	// load the message time into the temporary buffer
	binary.LittleEndian.PutUint64(b.buf8, message.Time)

	// write the message time to the index data entry's data section.
	_, err := indexData.Data.Write(b.buf8[:8])
	if err != nil {
		return err
	}

	// write the decompressed size of the current chunk, to the scratch buffer.
	binary.LittleEndian.PutUint32(b.buf8, b.currentChunk.decompressedSize)

	// write decompressed chunk size to the index data entry's data section.
	_, err = indexData.Data.Write(b.buf8[:4])
	if err != nil {
		return err
	}

	// write the message data to the chunk
	n, err := b.writeRecord(b.chunkWriter, header, message.Data)
	if err != nil {
		return err
	}

	// increment the current chunk size by the number of bytes written.
	b.currentChunk.decompressedSize += uint32(n)

	// if the timestamp of the message is less than the start time of the
	// current chunk, lower the current chunk start time.
	if message.Time < b.currentChunk.startTime {
		b.currentChunk.startTime = message.Time
	}

	// if the timestamp of the message exceeds the end time of the current
	// chunk, raise the chunk end time.
	if message.Time > b.currentChunk.endTime {
		b.currentChunk.endTime = message.Time
	}

	// increment the number of messages in the chunk.
	b.currentChunk.messageCount++

	return nil
}

// Close the bag file, and if the output writer implements WriteSeeker, also
// overwrite the bag header record with correct values. If the output writer
// does not implement write seeker, the resulting index will be structurally
// correct, but not linked from the file header. This can be repaired by running
// "rosbag reindex", at the cost of rewriting the bag. A smarter tool could scan
// the file to locate the index records and update the pointer in place.
func (b *BagWriter) Close() error {
	err := b.flushActiveChunk()
	if err != nil {
		return err
	}

	indexPos := b.out.BytesWritten()

	// The bag specification does not exactly spell it out, but ROS tooling
	// expects the post-chunk section to consist of a block of connection
	// records, followed by a block of chunk info records.
	for _, connection := range b.outputStats.connections {
		_, err = b.writeConnection(b.out, connection)
		if err != nil {
			return err
		}
	}

	// The chunk info records mentioned above.
	for _, chunkInfo := range b.outputStats.chunkInfo {
		err = b.writeChunkInfo(chunkInfo)
		if err != nil {
			return err
		}
	}

	// if we have an io.WriteSeeker, seek back to the start and add the pointer
	// to the index. Otherwise, caller will need to reindex the bag for ROS
	// tooling to respect it.
	if ws, ok := b.w.(io.WriteSeeker); ok {
		// location of the bag header is right after the magic.
		_, err := ws.Seek(int64(len(Magic)), io.SeekStart)
		if err != nil {
			return err
		}
		// the overwrite will take identical space to the original, since the
		// only types used are fixed-size.
		err = b.writeBagHeader(BagHeader{
			IndexPos:   uint64(indexPos),
			ConnCount:  uint32(len(b.outputStats.connections)),
			ChunkCount: uint32(len(b.outputStats.chunkInfo)),
		})
		if err != nil {
			return err
		}
	}

	return nil
}

// writeIndexData writes an index data record to the output. See
// http://wiki.ros.org/Bags/Format/2.0#Index_data for details.
func (b *BagWriter) writeIndexData(indexData IndexData) error {
	ver := make([]byte, 4)
	conn := make([]byte, 4)
	count := make([]byte, 4)

	binary.LittleEndian.PutUint32(ver, 1) // version 1 is assumed
	binary.LittleEndian.PutUint32(conn, indexData.Conn)
	binary.LittleEndian.PutUint32(count, indexData.Count)

	header := b.buildHeader(&b.header,
		[]byte("op"), []byte{byte(OpIndexData)},
		[]byte("ver"), ver,
		[]byte("conn"), conn,
		[]byte("count"), count,
	)

	_, err := b.writeRecord(b.out, header, indexData.Data.Bytes())
	if err != nil {
		return err
	}

	return nil
}

// writeChunkInfo writes a chunk info record to the output. See
// http://wiki.ros.org/Bags/Format/2.0#Chunk_info for details.
func (b *BagWriter) writeChunkInfo(chunkInfo ChunkInfo) error {
	ver := make([]byte, 4)
	chunkPos := make([]byte, 8)
	startTime := make([]byte, 8)
	endTime := make([]byte, 8)
	count := make([]byte, 4)

	binary.LittleEndian.PutUint32(ver, 1) // version 1 is assumed
	binary.LittleEndian.PutUint64(chunkPos, chunkInfo.ChunkPos)
	binary.LittleEndian.PutUint64(startTime, chunkInfo.StartTime)
	binary.LittleEndian.PutUint64(endTime, chunkInfo.EndTime)
	binary.LittleEndian.PutUint32(count, uint32(len(chunkInfo.Data)))

	header := b.buildHeader(&b.header,
		[]byte("op"), []byte{byte(OpChunkInfo)},
		[]byte("ver"), ver,
		[]byte("chunk_pos"), chunkPos,
		[]byte("start_time"), startTime,
		[]byte("end_time"), endTime,
		[]byte("count"), count,
	)

	// The data portion of a chunk info record consists of back-to-back
	// connection IDs and record-on-connection counts, serialized as uint32
	// pairs. The writer maintains these as a map during writing. To ensure
	// consistent outputs, we sort the keys here and then write the records out
	// in sorted order.
	connIDs := make([]uint32, 0, len(chunkInfo.Data))
	for connID := range chunkInfo.Data {
		connIDs = append(connIDs, connID)
	}

	sort.Slice(connIDs, func(i, j int) bool {
		return connIDs[i] < connIDs[j]
	})

	data := make([]byte, 8*len(chunkInfo.Data))
	offset := 0

	for _, connID := range connIDs {
		count := chunkInfo.Data[connID]
		offset += copyUint32(data[offset:], connID)
		offset += copyUint32(data[offset:], count)
	}

	// Write header and data to the output.
	_, err := b.writeRecord(b.out, header, data)
	if err != nil {
		return err
	}

	return nil
}

// writeBagHeader writes a bag header record to the output. See
// http://wiki.ros.org/Bags/Format/2.0#Bag_header for details.
func (b *BagWriter) writeBagHeader(bagHeader BagHeader) error {
	indexPos := make([]byte, 8)
	connCount := make([]byte, 4)
	chunkCount := make([]byte, 4)

	binary.LittleEndian.PutUint64(indexPos, bagHeader.IndexPos)
	binary.LittleEndian.PutUint32(connCount, bagHeader.ConnCount)
	binary.LittleEndian.PutUint32(chunkCount, bagHeader.ChunkCount)

	header := b.buildHeader(&b.header,
		[]byte("op"), []byte{byte(OpBagHeader)},
		[]byte("index_pos"), indexPos,
		[]byte("conn_count"), connCount,
		[]byte("chunk_count"), chunkCount,
	)

	// The bag header record is padded to 4096 bytes, including data section,
	// header, and lengths of both as uint32. The padding is the data section,
	// after this subtraction.
	paddingLength := 4096 - len(header) - 4 - 4

	data := make([]byte, paddingLength)
	for i := 0; i < len(data); i++ {
		data[i] = 0x20
	}

	_, err := b.writeRecord(b.out, header, data)
	if err != nil {
		return err
	}

	return nil
}

// writeRecord writes a record to the output. In ROS, a record consists of <. See
// http://wiki.ros.org/Bags/Format/2.0#Records for details.
func (b *BagWriter) writeRecord(w io.Writer, header, data []byte) (int, error) {
	binary.LittleEndian.PutUint32(b.buf8, uint32(len(header)))

	_, err := w.Write(b.buf8[:4])
	if err != nil {
		return 0, err
	}

	_, err = w.Write(header)
	if err != nil {
		return 0, err
	}

	binary.LittleEndian.PutUint32(b.buf8, uint32(len(data)))

	_, err = w.Write(b.buf8[:4])
	if err != nil {
		return 0, err
	}

	_, err = w.Write(data)
	if err != nil {
		return 0, err
	}

	recordLen := 4 + len(header) + 4 + len(data)

	return recordLen, nil
}

// buildHeader builds a header from a list of key-value pairs. See
// http://wiki.ros.org/Bags/Format/2.0#Headers for details.
func (b *BagWriter) buildHeader(buf *[]byte, keyvals ...[]byte) []byte {
	if buf == nil {
		b := make([]byte, 1)
		buf = &b
	}
	if len(keyvals)%2 != 0 {
		panic("keyvals must be even")
	}

	headerLen := 0
	for i := 0; i < len(keyvals); i += 2 {
		headerLen += 4 + len(keyvals[i]) + 1 + len(keyvals[i+1])
	}

	if len(*buf) < headerLen {
		*buf = make([]byte, headerLen)
	}

	offset := 0
	buffer := *buf

	for i := 0; i < len(keyvals); i += 2 {
		key := keyvals[i]
		value := keyvals[i+1]
		offset += copyUint32(buffer[offset:], uint32(len(key)+1+len(value)))
		offset += copy(buffer[offset:], key)
		buffer[offset] = '='
		offset++
		offset += copy(buffer[offset:], value)
	}

	return buffer[:offset]
}

// flushActiveChunk flushes the current chunk to the output, along with
// associated chunk index records. It then opens a new chunk for subsequent
// writes, with appropriate statistics zeroed.
func (b *BagWriter) flushActiveChunk() error {
	if b.currentChunk.decompressedSize == 0 {
		return nil
	}

	// flush any uncompressed bytes to the chunk buffer.
	err := b.chunkWriter.Flush()
	if err != nil {
		return err
	}

	// take current position in the output buffer. This is the location of the chunk record.
	chunkPos := b.out.BytesWritten()

	// read current decompressed size of chunk
	binary.LittleEndian.PutUint32(b.buf8, b.currentChunk.decompressedSize)

	// build header
	header := b.buildHeader(&b.header,
		[]byte("op"), []byte{byte(OpChunk)},
		[]byte("compression"), []byte(b.config.compression),
		[]byte("size"), b.buf8[:4],
	)

	// write header length
	err = binary.Write(b.out, binary.LittleEndian, uint32(len(header)))
	if err != nil {
		return err
	}

	// write header
	_, err = b.out.Write(header)
	if err != nil {
		return err
	}

	// write data (compressed chunk) length
	err = binary.Write(b.out, binary.LittleEndian, uint32(b.chunkBuffer.Len()))
	if err != nil {
		return err
	}

	// write data
	_, err = io.Copy(b.out, b.chunkBuffer)
	if err != nil {
		return err
	}

	// A chunk record is followed by one ChunkInfo record per populated
	// connection. Here we sort these records by ID to ensure deterministic
	// outputs from the writer for identical inputs - note that map iteration in
	// Go is otherwise random. From the spec POV the ordering does not make a
	// difference, but the cost is low.
	chunkInfoData := make(map[uint32]uint32)
	for connID, chunkIndex := range b.currentChunk.indexData {
		chunkInfoData[connID] = chunkIndex.Count
	}

	keys := make([]int, 0, len(b.currentChunk.indexData))
	for key := range b.currentChunk.indexData {
		keys = append(keys, int(key))
	}

	sort.Ints(keys)

	for _, key := range keys {
		indexData := b.currentChunk.indexData[uint32(key)]
		if indexData.Count > 0 {
			err = b.writeIndexData(*indexData)
			if err != nil {
				return err
			}
		}
	}

	// Append a chunk info record to the writer's collection of them. These will
	// be converted to physical records on file close.
	b.outputStats.chunkInfo = append(b.outputStats.chunkInfo, ChunkInfo{
		ChunkPos:  uint64(chunkPos),
		StartTime: b.currentChunk.startTime,
		EndTime:   b.currentChunk.endTime,
		Count:     b.currentChunk.messageCount,
		Data:      chunkInfoData,
	})

	// prepare to proceed with the next chunk, by blanking
	// "currentChunk"-specific state.
	b.resetActiveChunkState()

	return nil
}

// resetActiveChunkState resets the state of the current chunk to zero.
func (b *BagWriter) resetActiveChunkState() {
	b.chunkBuffer.Reset()
	b.chunkWriter.Reset(b.chunkBuffer)
	b.currentChunk.decompressedSize = 0
	b.currentChunk.startTime = math.MaxUint64
	b.currentChunk.endTime = 0
	b.currentChunk.messageCount = 0

	for _, indexData := range b.currentChunk.indexData {
		// zero out all the index data attributes, but keep the underlying space
		indexData.Data.Reset()
		indexData.Count = 0
	}
}

// writeConnection writes a connection record to the output. See
// http://wiki.ros.org/Bags/Format/2.0#Connection for details.
func (b *BagWriter) writeConnection(w io.Writer, connection Connection) (int, error) {
	binary.LittleEndian.PutUint32(b.buf8, connection.Conn)
	binary.LittleEndian.PutUint32(b.buf8, connection.Conn)
	header := b.buildHeader(&b.header,
		[]byte("op"), []byte{byte(OpConnection)},
		[]byte("conn"), b.buf8[:4],
		[]byte("topic"), []byte(connection.Topic),
	)
	dataKeyvals := [][]byte{
		[]byte("topic"), []byte(connection.Data.Topic),
		[]byte("type"), []byte(connection.Data.Type),
		[]byte("md5sum"), []byte(connection.Data.MD5Sum),
		[]byte("message_definition"), connection.Data.MessageDefinition,
	}

	// if callerid is present, append it to the data keyvals
	if connection.Data.CallerID != nil {
		dataKeyvals = append(dataKeyvals, []byte("callerid"), []byte(*connection.Data.CallerID))
	}

	// if latching is requested, append it to the data keyvals
	if latching := connection.Data.Latching; latching != nil {
		if *latching {
			dataKeyvals = append(dataKeyvals, []byte("latching"), []byte("1"))
		} else {
			dataKeyvals = append(dataKeyvals, []byte("latching"), []byte("0"))
		}
	}

	// this allocates - expected to be infrequent
	data := b.buildHeader(nil, dataKeyvals...)

	n, err := b.writeRecord(w, header, data)
	if err != nil {
		return n, err
	}

	return n, nil
}

// rostime computes a "rostime" from a nanosecond unix timestamp. A ROS time
// consists of the 8 sequence created by appending the uint32 nanoseconds since
// last second, to the uint32 seconds since the epoch.
func rostime(x uint64) []byte {
	secs := x / 1e9
	nsecs := x % 1e9
	b := make([]byte, 8)
	binary.LittleEndian.PutUint32(b, uint32(secs))
	binary.LittleEndian.PutUint32(b[4:], uint32(nsecs))

	return b
}
