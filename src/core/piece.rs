use sha1::{Digest, Sha1};
use std::collections::HashMap;

pub const BLOCK_LEN: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRange {
    pub begin: u32,
    pub length: u32,
}

pub fn block_ranges(piece_length: usize) -> Vec<BlockRange> {
    let mut ranges = Vec::new();
    let mut begin = 0usize;

    while begin < piece_length {
        let length = std::cmp::min(BLOCK_LEN, piece_length - begin) as u32;
        ranges.push(BlockRange { begin: begin as u32, length });
        begin += length as usize;
    }

    ranges
}

pub fn assemble_piece(
    piece_length: usize,
    blocks: &HashMap<u32, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let mut piece = Vec::with_capacity(piece_length);
    let mut offset = 0usize;

    while offset < piece_length {
        let block = blocks
            .get(&(offset as u32))
            .ok_or_else(|| format!("missing block starting at {}", offset))?;

        if block.len() > piece_length - offset {
            return Err(format!("block at {} exceeds piece bounds", offset));
        }

        piece.extend_from_slice(block);
        offset += block.len();
    }

    Ok(piece)
}

pub fn verify_piece(expected_hash: &[u8; 20], piece_data: &[u8]) -> Result<(), String> {
    let mut hasher = Sha1::new();
    hasher.update(piece_data);
    let actual: [u8; 20] = hasher.finalize().into();

    if &actual != expected_hash {
        Err("piece hash mismatch".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_ranges_generates_correct_boundaries() {
        let piece_length = 50_000;
        let ranges = block_ranges(piece_length);

        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges[0], BlockRange { begin: 0, length: 16_384 });
        assert_eq!(ranges[1], BlockRange { begin: 16_384, length: 16_384 });
        assert_eq!(ranges[2], BlockRange { begin: 32_768, length: 16_384 });
        assert_eq!(ranges[3], BlockRange { begin: 49_152, length: 848 });
    }

    #[test]
    fn assemble_piece_combines_blocks_in_order() {
        let mut blocks = HashMap::new();
        blocks.insert(0, vec![1, 2, 3]);
        blocks.insert(3, vec![4, 5]);

        let assembled = assemble_piece(5, &blocks).unwrap();
        assert_eq!(assembled, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn verify_piece_detects_valid_hash() {
        let data = b"hello world";
        let mut hasher = Sha1::new();
        hasher.update(data);
        let expected: [u8; 20] = hasher.finalize().into();

        assert!(verify_piece(&expected, data).is_ok());
    }
}
