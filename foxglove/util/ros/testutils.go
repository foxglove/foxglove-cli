package ros

func connection(id uint32, topic string) Connection {
	return Connection{
		Conn:  id,
		Topic: topic,
		Data: ConnectionData{
			Topic:             topic,
			Type:              "123",
			MD5Sum:            "abc",
			MessageDefinition: []byte{0x01, 0x02, 0x03},
			CallerID:          nil,
			Latching:          nil,
		},
	}
}

func message(conn uint32, time uint64, data []byte) Message {
	return Message{
		Conn: conn,
		Time: time,
		Data: data,
	}
}
