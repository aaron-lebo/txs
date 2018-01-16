#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate rocksdb;
extern crate sha2;

use rocksdb::DB;
use sha2::{Digest, Sha256};
use std::{env, mem, time};

#[derive(Debug, Serialize, Deserialize)]
struct Block {
    timestamp: u64,
    data: String,
    prev_hash: String,
    hash: String,
}

impl Block {
    fn new(data: &str, prev_hash: &str) -> Self {
        let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let ts = ts.as_secs() * 1000 + ts.subsec_nanos() as u64 / 1_000_000;
        let ts_bytes: [u8; 8] = unsafe { mem::transmute(ts.to_be()) };
        let mut hash = Sha256::default();
        hash.input(&ts_bytes);
        hash.input(data.as_bytes());
        hash.input(prev_hash.as_bytes());
        Block {
            timestamp: ts,
            data: data.to_owned(),
            prev_hash: prev_hash.to_owned(),
            hash: format!("{:x}", hash.result()),
        }
    }

    fn save(&self, db: &DB) {
        let hash = &self.hash.as_bytes();
        let encoded: Vec<u8> = bincode::serialize(&self, bincode::Infinite).unwrap();
        db.put(hash, &encoded).unwrap();
        db.put(b"tip", hash).unwrap();
    }
}

struct Blockchain {
    db: DB,
    tip: String,
}

impl Blockchain {
    fn new() -> Self {
        let db = DB::open_default("./data").unwrap();
        let tip = match db.get(b"tip") {
            Ok(Some(val)) => String::from_utf8(val.to_vec()).unwrap(),
            Ok(None) => {
                let block = Block::new("genesis", "");
                block.save(&db);
                block.hash
            },
            Err(err) => panic!(err),
        };
        Blockchain { db, tip }
    }

    fn add(&mut self, data: &str) -> Block {
        let block = Block::new(data, &self.tip);
        block.save(&self.db);
        self.tip = block.hash.clone();
        block
    }

    fn items(&self) -> Vec<Block> {
        let mut tip = self.tip.clone();
        let mut blocks = Vec::new();
        while tip != "" {
            let encoded = self.db.get(&tip.as_bytes()).unwrap().unwrap();
            let block: Block = bincode::deserialize(&encoded[..]).unwrap();
            tip = block.prev_hash.clone();
            blocks.push(block);
        }
        blocks
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let args: (&str, &str, &str) = match args.len() {
        4 => (&args[1], &args[2], &args[3]),
        3 => (&args[1], &args[2], ""),
        _ => ("", "", ""),
    };
    let mut chain = Blockchain::new();
    match args {
        ("blocks", "list", "") => {
            for block in chain.items() {
                println!("{:?}", block);
            }
        },
        ("blocks", "create", data) => println!("{:?}", chain.add(data)),
        _ => println!("bad args"),
    }
}
