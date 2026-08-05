use crate::core::bencode::BencodeValue;
use sha1::{Sha1, Digest};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct TorrentFile {
    pub announce: String,
    pub info_hash: [u8; 20],       // raw sha1 bytes is what trackers + peers want
    pub piece_length: i64,
    pub pieces: Vec<[u8; 20]>,     // sha1 hash per piece chopped from one big blob :sob:
    pub name: String,
    pub length: i64,               // total size, singlefile mode only for now
}
