extern crate sha2;

use sha2::{Digest, Sha256};
use std::{mem, time};

#[derive(Debug)]
struct Block {
    timestamp: u64,
    data: Vec<u8>,
    prev_hash: Vec<u8>,
    hash: Vec<u8>,
}

impl Block {
    fn new(data: &[u8], prev_hash: &[u8]) -> Self {
        let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let ts = ts.as_secs() * 1000 + ts.subsec_nanos() as u64 / 1_000_000;
        let ts_bytes: [u8; 8] = unsafe { mem::transmute(ts.to_be()) };
        let mut hash = Sha256::default();
        hash.input(&ts_bytes);
        hash.input(data);
        hash.input(prev_hash);
        Block {
            timestamp: ts,
            data: data.to_owned(),
            prev_hash: prev_hash.to_owned(),
            hash: hash.result()[..].to_owned(),
        }
    }
}

struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    fn new() -> Self {
        Blockchain {
            blocks: vec![Block::new(b"genesis", b"")],
        }
    }

    fn add(&mut self, data: &[u8]) {
        let prev_hash = self.blocks.last().unwrap().hash.clone();
        self.blocks.push(Block::new(data, &prev_hash));
    }
}

fn main() {
    let mut chain = Blockchain::new();
    chain.add(b"hello world");
    for block in chain.blocks {
        println!("{:?}", block);
    }
}
