package util

import "io"

type CountingWriter struct {
	w            io.Writer
	bytesWritten int64
}

func NewCountingWriter(w io.Writer) *CountingWriter {
	return &CountingWriter{w: w}
}

func (w *CountingWriter) Write(p []byte) (int, error) {
	n, err := w.w.Write(p)
	w.bytesWritten += int64(n)
	return n, err
}

func (w *CountingWriter) BytesWritten() int64 {
	return w.bytesWritten
}
