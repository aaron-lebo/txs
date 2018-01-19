#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate rocksdb;
extern crate sha2;

use rocksdb::DB;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::{env, mem, time};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Input {
    txid: String,
    index: i8,
    sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Output {
    amount: u64,
    pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    id: String,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
}

impl Transaction {
    fn coinbase() -> Self {
        let mut tx = Transaction {
            id: "".to_owned(),
            inputs: vec!(Input { txid: "".to_owned(), index: -1, sig: "".to_owned() }),
            outputs: vec!(Output { amount: 100, pubkey: "dog".to_owned() })
        };
        tx.id = tx.hash();
        tx
    }

    fn is_coinbase(&self) -> bool {
        if self.inputs.len() != 1 {
            return false;
        }
        let input = &self.inputs[0];
        input.txid == "" && input.index == -1
    }


    fn encode(&self) -> Vec<u8> {
        bincode::serialize(&self, bincode::Infinite).unwrap()
    }

    fn hash(&self) -> String {
        let mut hash = Sha256::default();
        hash.input(&self.encode());
        format!("{:x}", hash.result())
    }
}

type Txs = Vec<Transaction>;

#[derive(Debug, Serialize, Deserialize)]
struct Block {
    timestamp: u64,
    transactions: Txs,
    prev_hash: String,
    hash: String,
}

impl Block {
    fn new(txs: Txs, prev_hash: &str) -> Self {
        let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let ts = ts.as_secs() * 1000 + ts.subsec_nanos() as u64 / 1_000_000;
        let ts_bytes: [u8; 8] = unsafe { mem::transmute(ts.to_be()) };
        let mut hash = Sha256::default();
        hash.input(&ts_bytes);
        for tx in &txs {
            hash.input(&tx.encode());
        }
        hash.input(prev_hash.as_bytes());
        Block {
            timestamp: ts,
            transactions: txs,
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
                let block = Block::new(vec!(Transaction::coinbase()), "");
                block.save(&db);
                block.hash
            },
            Err(err) => panic!(err),
        };
        Blockchain { db, tip }
    }

    fn add(&mut self, txs: Txs) -> Block {
        let block = Block::new(txs, &self.tip);
        block.save(&self.db);
        self.tip = block.hash.clone();
        block
    }

    fn unspent_txs(&self, addr: &str) -> Txs {
        let mut spent: HashMap<String, Vec<i8>> = HashMap::new();
        let mut unspent = vec!();
        for block in self.blocks() {
            for tx in block.transactions {
                'outputs: for (i, out) in tx.outputs.iter().enumerate() {
                    if let Some(idxs) = spent.get(&tx.id) {
                        for idx in idxs {
                            if *idx == i as i8 {
                                continue 'outputs;
                            }
                        }
                    }
                    if out.pubkey == addr {
                        unspent.push(tx.clone());
                    }
                }
                if !tx.is_coinbase() {
                    for input in tx.inputs {
                        spent.entry(tx.id.to_owned()).or_insert(vec!()).push(input.index);
                    }
                }
            }
            if block.prev_hash == "" {
                break;
            }
        }
        unspent
    }

    fn utxos(&self, addr: &str) -> Vec<Output> {
        let mut utxos = vec!();
        for tx in self.unspent_txs(addr) {
            for out in tx.outputs {
                if out.pubkey == addr {
                    utxos.push(out);
                }
            }
        }
        utxos
    }

    fn balance(&self, addr: &str) -> u64 {
        let mut balance = 0;
        for out in self.utxos(addr) {
            balance += out.amount;
        }
        balance
    }

    fn blocks(&self) -> Vec<Block> {
        let mut tip = self.tip.clone();
        let mut blocks = vec!();
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
            for block in chain.blocks() {
                println!("{:?}", block);
            }
        },
        ("balance", addr, "") => println!("{} has a balance of {}", addr, chain.balance(addr)),
        _ => println!("bad args"),
    }
}
