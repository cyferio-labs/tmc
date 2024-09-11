// Run the code with `cargo run --release --bin fhe-keygen` at root directory
use serde_json::json;
use sov_bank_fhe::fhe_key::fhe_key_gen;
use std::{env, fs, path};

// For timing
use std::time::Instant;

fn main() {
    let start = Instant::now();

    // generate FHE keys
    let key_config = fhe_key_gen();

    let genesis_config = json!({
        "tokens": [],
        "fhe_public_key": key_config.public_key,
        "fhe_server_key": key_config.server_key,
    });

    // get the root path and join with the key directory
    let root_path = env::current_dir().unwrap();
    let genesis_path = path::Path::new(&root_path).join("test-data/genesis/mock");
    let sui_genesis_path = path::Path::new(&root_path).join("test-data/genesis/sui");
    let key_path = path::Path::new(&root_path).join("test-data/keys");

    // write the genesis file
    fs::write(
        genesis_path.join("bank_fhe.json"),
        genesis_config.to_string(),
    )
    .unwrap();

    println!(
        "[Init] FHE Keys generated and serialized in {:?}\n[Init] Public key and server key are stored in {:?}",
        start.elapsed(),
        genesis_path.join("bank_fhe.json")
    );

    fs::write(
        sui_genesis_path.join("bank_fhe.json"),
        genesis_config.to_string(),
    )
    .unwrap();

    println!(
        "[Init] FHE Keys generated and serialized in {:?}\n[Init] Public key and server key are stored in {:?}",
        start.elapsed(),
        sui_genesis_path.join("bank_fhe.json")
    );

    // store the private key for debug usage
    let fhe_private_key = json!({
        "fhe_private_key": key_config.private_key,
    });

    std::fs::write(
        key_path.join("private_key.json"),
        fhe_private_key.to_string(),
    )
    .unwrap();
    println!(
        "[Init] Private key for debugging stored in {:?}",
        key_path.join("private_key.json")
    );
}
