use std::io::{Read, Write};

pub const PROTOCOL_STR: &str = "BitTorrent protocol";
pub const PROTOCOL_LEN: u8 = 19;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub reserved: [u8; 8],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            info_hash,
            peer_id,
            reserved: [0; 8],
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + PROTOCOL_LEN as usize + 8 + 20 + 20);
        buf.push(PROTOCOL_LEN);
        buf.extend_from_slice(PROTOCOL_STR.as_bytes());
        buf.extend_from_slice(&self.reserved);
        buf.extend_from_slice(&self.info_hash);
        buf.extend_from_slice(&self.peer_id);
        buf
    }

    pub fn decode(data: &[u8]) -> Result<(Self, &[u8]), String> {
        if data.len() < 1 {
            return Err("handshake too short".into());
        }

        let pstrlen = data[0];
        if pstrlen != PROTOCOL_LEN {
            return Err(format!("unexpected protocol length: {}", pstrlen));
        }

        let required = 1 + PROTOCOL_LEN as usize + 8 + 20 + 20;
        if data.len() < required {
            return Err("handshake too short".into());
        }

        let protocol = &data[1..1 + PROTOCOL_LEN as usize];
        if protocol != PROTOCOL_STR.as_bytes() {
            return Err("unexpected protocol string".into());
        }

        let reserved_start = 1 + PROTOCOL_LEN as usize;
        let info_start = reserved_start + 8;
        let peer_start = info_start + 20;

        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&data[reserved_start..info_start]);

        let mut info_hash = [0u8; 20];
        info_hash.copy_from_slice(&data[info_start..peer_start]);

        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&data[peer_start..peer_start + 20]);

        Ok((
            Handshake {
                info_hash,
                peer_id,
                reserved,
            },
            &data[peer_start + 20..],
        ))
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), String> {
        writer.write_all(&self.encode()).map_err(|e| e.to_string())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, String> {
        let mut header = [0u8; 1];
        reader.read_exact(&mut header).map_err(|e| e.to_string())?;
        if header[0] != PROTOCOL_LEN {
            return Err(format!("unexpected protocol length: {}", header[0]));
        }

        let mut buf = vec![0u8; (PROTOCOL_LEN as usize) + 8 + 20 + 20];
        reader.read_exact(&mut buf).map_err(|e| e.to_string())?;

        let protocol = &buf[..PROTOCOL_LEN as usize];
        if protocol != PROTOCOL_STR.as_bytes() {
            return Err("unexpected protocol string".into());
        }

        let reserved_start = PROTOCOL_LEN as usize;
        let info_start = reserved_start + 8;
        let peer_start = info_start + 20;

        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&buf[reserved_start..info_start]);

        let mut info_hash = [0u8; 20];
        info_hash.copy_from_slice(&buf[info_start..peer_start]);

        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&buf[peer_start..peer_start + 20]);

        Ok(Handshake {
            info_hash,
            peer_id,
            reserved,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request { index: u32, begin: u32, length: u32 },
    Piece { index: u32, begin: u32, block: Vec<u8> },
    Cancel { index: u32, begin: u32, length: u32 },
    Port(u16),
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Message::KeepAlive => 0u32.to_be_bytes().to_vec(),
            Message::Choke => [1u32.to_be_bytes().as_ref(), &[0u8]].concat(),
            Message::Unchoke => [1u32.to_be_bytes().as_ref(), &[1u8]].concat(),
            Message::Interested => [1u32.to_be_bytes().as_ref(), &[2u8]].concat(),
            Message::NotInterested => [1u32.to_be_bytes().as_ref(), &[3u8]].concat(),
            Message::Have(piece_index) => {
                let mut buf = Vec::with_capacity(9);
                buf.extend_from_slice(&5u32.to_be_bytes());
                buf.push(4);
                buf.extend_from_slice(&piece_index.to_be_bytes());
                buf
            }
            Message::Bitfield(bits) => {
                let len = 1 + bits.len() as u32;
                let mut buf = Vec::with_capacity(4 + len as usize);
                buf.extend_from_slice(&len.to_be_bytes());
                buf.push(5);
                buf.extend_from_slice(bits);
                buf
            }
            Message::Request { index, begin, length } => {
                let mut buf = Vec::with_capacity(4 + 1 + 12);
                buf.extend_from_slice(&13u32.to_be_bytes());
                buf.push(6);
                buf.extend_from_slice(&index.to_be_bytes());
                buf.extend_from_slice(&begin.to_be_bytes());
                buf.extend_from_slice(&length.to_be_bytes());
                buf
            }
            Message::Piece { index, begin, block } => {
                let len = 9 + block.len() as u32;
                let mut buf = Vec::with_capacity(4 + len as usize);
                buf.extend_from_slice(&len.to_be_bytes());
                buf.push(7);
                buf.extend_from_slice(&index.to_be_bytes());
                buf.extend_from_slice(&begin.to_be_bytes());
                buf.extend_from_slice(block);
                buf
            }
            Message::Cancel { index, begin, length } => {
                let mut buf = Vec::with_capacity(4 + 1 + 12);
                buf.extend_from_slice(&13u32.to_be_bytes());
                buf.push(8);
                buf.extend_from_slice(&index.to_be_bytes());
                buf.extend_from_slice(&begin.to_be_bytes());
                buf.extend_from_slice(&length.to_be_bytes());
                buf
            }
            Message::Port(port) => {
                let mut buf = Vec::with_capacity(4 + 1 + 2);
                buf.extend_from_slice(&3u32.to_be_bytes());
                buf.push(9);
                buf.extend_from_slice(&port.to_be_bytes());
                buf
            }
        }
    }

    pub fn decode(data: &[u8]) -> Result<(Self, &[u8]), String> {
        if data.len() < 4 {
            return Err("message too short".into());
        }

        let len = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
        if data.len() < 4 + len {
            return Err("message length mismatch".into());
        }

        if len == 0 {
            return Ok((Message::KeepAlive, &data[4..]));
        }

        let id = data[4];
        let payload = &data[5..4 + len];

        let msg = match id {
            0 => Message::Choke,
            1 => Message::Unchoke,
            2 => Message::Interested,
            3 => Message::NotInterested,
            4 => {
                if payload.len() != 4 {
                    return Err("have payload wrong size".into());
                }
                let index = u32::from_be_bytes(payload.try_into().unwrap());
                Message::Have(index)
            }
            5 => Message::Bitfield(payload.to_vec()),
            6 => {
                if payload.len() != 12 {
                    return Err("request payload wrong size".into());
                }
                let index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let begin = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let length = u32::from_be_bytes(payload[8..12].try_into().unwrap());
                Message::Request { index, begin, length }
            }
            7 => {
                if payload.len() < 8 {
                    return Err("piece payload too short".into());
                }
                let index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let begin = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let block = payload[8..].to_vec();
                Message::Piece { index, begin, block }
            }
            8 => {
                if payload.len() != 12 {
                    return Err("cancel payload wrong size".into());
                }
                let index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let begin = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let length = u32::from_be_bytes(payload[8..12].try_into().unwrap());
                Message::Cancel { index, begin, length }
            }
            9 => {
                if payload.len() != 2 {
                    return Err("port payload wrong size".into());
                }
                let port = u16::from_be_bytes(payload.try_into().unwrap());
                Message::Port(port)
            }
            _ => return Err(format!("unknown message id: {}", id)),
        };

        Ok((msg, &data[4 + len..]))
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), String> {
        writer.write_all(&self.encode()).map_err(|e| e.to_string())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, String> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
        let len = u32::from_be_bytes(len_buf);

        if len == 0 {
            return Ok(Message::KeepAlive);
        }

        let mut payload = vec![0u8; len as usize];
        reader.read_exact(&mut payload).map_err(|e| e.to_string())?;
        let mut full = Vec::with_capacity(4 + payload.len());
        full.extend_from_slice(&len_buf);
        full.extend_from_slice(&payload);
        Message::decode(&full).map(|(msg, _)| msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_encode_decode_roundtrip() {
        let info_hash = [0x11u8; 20];
        let peer_id = *b"-TC0001-123456789012";
        let handshake = Handshake::new(info_hash, peer_id);
        let encoded = handshake.encode();
        let (decoded, rest) = Handshake::decode(&encoded).unwrap();

        assert_eq!(rest.len(), 0);
        assert_eq!(decoded.info_hash, info_hash);
        assert_eq!(decoded.peer_id, peer_id);
        assert_eq!(decoded.reserved, [0; 8]);
    }

    #[test]
    fn message_encode_decode_roundtrip() {
        let messages = vec![
            Message::KeepAlive,
            Message::Choke,
            Message::Unchoke,
            Message::Interested,
            Message::NotInterested,
            Message::Have(42),
            Message::Bitfield(vec![0b10101010, 0b11001100]),
            Message::Request { index: 1, begin: 0, length: 16384 },
            Message::Piece { index: 1, begin: 0, block: vec![1, 2, 3, 4] },
            Message::Cancel { index: 1, begin: 0, length: 16384 },
            Message::Port(6881),
        ];

        let mut buf = Vec::new();
        for message in &messages {
            buf.extend_from_slice(&message.encode());
        }

        let mut rest = &buf[..];
        for expected in messages {
            let (decoded, remaining) = Message::decode(rest).unwrap();
            assert_eq!(decoded, expected);
            rest = remaining;
        }
        assert!(rest.is_empty());
    }
}
