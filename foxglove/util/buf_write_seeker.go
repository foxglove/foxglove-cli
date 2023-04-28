package util

type BufWriteSeeker struct {
	buf []byte
	pos int
}

func (b *BufWriteSeeker) Write(p []byte) (int, error) {
	needcap := b.pos + len(p)
	if needcap > cap(b.buf) {
		newBuf := make([]byte, len(b.buf), needcap*2)
		copy(newBuf, b.buf)
		b.buf = newBuf
	}
	if needcap > len(b.buf) {
		b.buf = b.buf[:needcap]
	}
	copy(b.buf[b.pos:], p)
	b.pos += len(p)
	return len(p), nil
}

func (b *BufWriteSeeker) Seek(offset int64, whence int) (int64, error) {
	switch whence {
	case 0:
		b.pos = int(offset)
	case 1:
		b.pos += int(offset)
	case 2:
		b.pos = len(b.buf) + int(offset)
	}
	return int64(b.pos), nil
}

func (b *BufWriteSeeker) Bytes() []byte {
	return b.buf
}

func NewBufWriteSeeker() *BufWriteSeeker {
	return &BufWriteSeeker{}
}
