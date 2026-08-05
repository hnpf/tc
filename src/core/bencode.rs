#[derive(Debug, Clone, PartialEq)]
pub enum BencodeValue {
    Int(i64),
    Bytes(Vec<u8>),              // NOT String, torrent data isnt always valid utf8
    List(Vec<BencodeValue>),
    Dict(BTreeMap<Vec<u8>, BencodeValue>), // BTreeMap autosorting keys, is important
    // note to self    
    // bencode dicts have to have sorted keys and most torrent clients rely on ts for 
    //consistent info_hash, basically BTreeMap sorts by key automatically, 
    // HashMap has no guaranteed order :(
}

// this is the key trick for recursive parsing - ((BencodeValue, &[u8]))
pub fn decode(data: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    match data.first() {
        Some(b'i') => decode_int(data),
        Some(b'l') => decode_list(data),
        Some(b'd') => decode_dict(data),
        Some(b'0'..=b'9') => decode_bytes(data),
        _ => Err("invalid bencode".into()),
    }
}

// int decoder is supposed to be simplest one
fn decode_int(data: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let end = data.iter().position(|&b| b == b'e').ok_or("unterminated int")?;
    let num_str = std::str::from_utf8(&data[1..end]).map_err(|_| "bad utf8 in int")?;
    let num: i64 = num_str.parse().map_err(|_| "bad int")?;
    Ok((BencodeValue::Int(num), &data[end + 1..]))
}

// everything else (byte string decode, whatever)
fn decode_bytes(data: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let colon = data.iter().position(|&b| b == b':').ok_or("no colon")?;
    let len_str = std::str::from_utf8(&data[..colon]).map_err(|_| "bad length")?;
    let len: usize = len_str.parse().map_err(|_| "bad length")?;
    let start = colon + 1;
    let end = start + len;
    if end > data.len() { return Err("string too short".into()); }
    Ok((BencodeValue::Bytes(data[start..end].to_vec()), &data[end..]))
}

// just calls decode() in a loop
fn decode_list(data: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let mut items = Vec::new();
    let mut rest = &data[1..];
    // skip 'l'
    while rest.first() != Some(&b'e') {
        let (val, new_rest) = decode(rest)?;
        items.push(val);
        rest = new_rest;
    }
    Ok((BencodeValue::List(items), &rest[1..])) 
    // skip 'e'
}

// this is essentially the same as list but reads pairs.  - (keys gotta always be Bytes per spec, values can be anything)
fn decode_dict(data: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let mut map = BTreeMap::new();
    let mut rest = &data[1..];
    while rest.first() != Some(&b'e') {
        let (key, new_rest) = decode(rest)?;
        let key_bytes = match key {
            BencodeValue::Bytes(b) => b,
            _ => return Err("dict key must be bytes".into()),
        };
        let (val, new_rest2) = decode(new_rest)?;
        map.insert(key_bytes, val);
        rest = new_rest2;
    }
    Ok((BencodeValue::Dict(map), &rest[1..]))
}

use std::collections::BTreeMap;
impl BencodeValue {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_into(&mut buf);
        buf
    }

    pub fn encode_into(&self, buf: &mut Vec<u8>) {
        match self {
            BencodeValue::Int(val) => {
                buf.push(b'i');
                buf.extend_from_slice(val.to_string().as_bytes());
                buf.push(b'e');
            }
            BencodeValue::Bytes(val) => {
                buf.extend_from_slice(val.len().to_string().as_bytes());
                buf.push(b':');
                buf.extend_from_slice(val);
            }
            BencodeValue::List(val) => {
                buf.push(b'l');
                for item in val {
                    item.encode_into(buf);
                }
                buf.push(b'e');
            }
            BencodeValue::Dict(val) => {
                buf.push(b'd');
                // iterates in lexicographical order automatically
                for (k, v) in val {
                    buf.extend_from_slice(k.len().to_string().as_bytes());
                    buf.push(b':');
                    buf.extend_from_slice(k);
                    v.encode_into(buf);
                }
                buf.push(b'e');
            }
        }
    }
}

#[test]
fn roundtrip_real_torrent() {
    let data = std::fs::read("test_data/ubuntu.torrent").unwrap();
    let (parsed, rest) = decode(&data).unwrap();
    assert!(rest.is_empty(), "leftover bytes after decode");
    let re_encoded = parsed.encode();
    assert_eq!(data, re_encoded, "roundtrip mismatch, encoding isn't byte-identical to original");
}
