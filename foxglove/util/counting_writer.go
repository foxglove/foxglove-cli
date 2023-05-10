package util

import "io"

// CountingWriter is an io.Writer that tracks the number of bytes written.
type CountingWriter struct {
	w            io.Writer
	bytesWritten int64
}

// NewCountingWriter constructs a new CountingWriter.
func NewCountingWriter(w io.Writer) *CountingWriter {
	return &CountingWriter{w: w}
}

// Write data to the writer.
func (w *CountingWriter) Write(p []byte) (int, error) {
	n, err := w.w.Write(p)
	w.bytesWritten += int64(n)
	return n, err
}

// BytesWritten returns the number of bytes written to the writer.
func (w *CountingWriter) BytesWritten() int64 {
	return w.bytesWritten
}
