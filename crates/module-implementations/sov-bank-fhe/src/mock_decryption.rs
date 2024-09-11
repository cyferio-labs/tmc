use serde::{Deserialize, Serialize};
use serde_json;
use std::{env, fs};

use tfhe::{prelude::*, ClientKey, CompressedFheUint64};

// For timing
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug)]
struct PrivateKey {
    fhe_private_key: Vec<u8>,
}

// TODO: refactor this with struct

pub fn decrypt(raw_ct: &Vec<u8>) -> u64 {
    let private_key = get_private_key();
    let ct = bincode::deserialize::<CompressedFheUint64>(raw_ct)
        .unwrap()
        .decompress();
    ct.decrypt(&private_key)
}

// TODO: re-encrypt method
// pub fn mock_reencrypt(raw_ct: Vec<u8>, client_public_key: Vec<u8>) -> Vec<u8> {
//     // TODO
// }

fn get_private_key() -> ClientKey {
    // read private key from the file
    tracing::debug!("Reading private key...");
    let start = Instant::now();
    let root_path = env::current_dir().expect("Failed to get current directory");
    let raw_config = fs::read(root_path.join("../../test-data/keys/private_key.json"))
        .expect("Failed to read private key json");
    let config = serde_json::from_slice::<PrivateKey>(&raw_config)
        .expect("Failed to parse private key json");
    let private_key =
        bincode::deserialize(&config.fhe_private_key).expect("Failed to deserialize private key");
    tracing::debug!("Private key deserialized in {:?}", start.elapsed());
    private_key
}
