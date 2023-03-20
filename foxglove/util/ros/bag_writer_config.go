package ros

type BagWriterOption func(c *bagWriterConfig)

// WithChunksize sets the chunksize for the bag writer. The default is 4MB.
func WithChunksize(chunksize int) BagWriterOption {
	return func(c *bagWriterConfig) {
		c.chunksize = chunksize
	}
}

// SkipHeader skips writing the header to the bag. This is useful for appending
// to an existing partial bag or output stream.
func SkipHeader(skipHeader bool) BagWriterOption {
	return func(c *bagWriterConfig) {
		c.skipHeader = skipHeader
	}
}

type bagWriterConfig struct {
	chunksize   int
	skipHeader  bool
	compression string
}
