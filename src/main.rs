#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate rocksdb;
extern crate sha2;

use rocksdb::DB;
use sha2::{Digest, Sha256};
use std::{mem, time};

#[derive(Debug, Serialize, Deserialize)]
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

    fn save(&self, db: &DB) {
        let hash = &self.hash;
        let encoded: Vec<u8> = bincode::serialize(&self, bincode::Infinite).unwrap();
        db.put(hash, &encoded).unwrap();
        db.put(b"tip", hash).unwrap();
    }
}

struct Blockchain {
    db: DB,
    tip: Vec<u8>,
}

impl Blockchain {
    fn new() -> Self {
        let db = DB::open_default("./data").unwrap();
        let tip = match db.get(b"tip") {
            Ok(Some(val)) => val.to_vec(),
            Ok(None) => {
                let block = Block::new(b"genesis", b"");
                block.save(&db);
                block.hash
            },
            Err(err) => panic!(err),
        };
        Blockchain { db, tip }
    }

    fn add(&mut self, data: &[u8]) {
        let block = Block::new(data, &self.tip);
        block.save(&self.db);
        self.tip = block.hash;
    }

    fn items(&self) -> Vec<Block> {
        let mut tip = self.tip.clone();
        let mut blocks = Vec::new();
        while tip != b"" {
            let encoded = self.db.get(&tip).unwrap().unwrap();
            let block: Block = bincode::deserialize(&encoded[..]).unwrap();
            tip = block.prev_hash.clone();
            blocks.push(block);
        }
        blocks
    }
}

fn main() {
    let mut chain = Blockchain::new();
    chain.add(b"hello world");
    for block in chain.items() {
        println!("{:?}", block);
    }
}
