// Run the code with `cargo run --release --bin request-scripts-gen` at root directory
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, fs, path};

use bincode;
use sov_bank_fhe::fhe_key::FheKeyConfig;
use tfhe::{prelude::*, set_server_key, CompressedPublicKey, CompressedServerKey, FheUint64};

// For timing
use std::time::Instant;

fn main() {
    let start = Instant::now();

    // get the root path and join with the key directory
    let root_path = env::current_dir().unwrap();
    let requests_path = path::Path::new(&root_path).join("test-data/requests/fhe/");
    let fhe_key_config_path =
        path::Path::new(&root_path).join("test-data/genesis/mock/bank_fhe.json");

    // read fhe public key from the file
    let raw_config = fs::read(fhe_key_config_path).expect("Failed to read fhe key config json");
    let config = serde_json::from_slice::<FheKeyConfig>(&raw_config)
        .expect("Failed to parse fhe key config json");
    let fhe_public_key = bincode::deserialize::<CompressedPublicKey>(&config.fhe_public_key)
        .unwrap()
        .decompress();

    // read and set the server key in this environment
    let fhe_server_key = bincode::deserialize::<CompressedServerKey>(&config.fhe_server_key)
        .unwrap()
        .decompress();
    set_server_key(fhe_server_key);

    // create-token request
    let init_balance = {
        let amount = FheUint64::try_encrypt(1_000 as u64, &fhe_public_key)
            .unwrap()
            .compress();
        bincode::serialize(&amount).unwrap()
    };
    let create_token_request = json!({
        "CreateToken": {
            "salt": 11,
            "token_name": "sov-confidential-token",
            "initial_balance": init_balance,
            "mint_to_address": "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94",
            "authorized_minters": [
                "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94",
                "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqwr57gc",
            ],
        },
    });

    // mint request
    let mint_amount = {
        let amount = FheUint64::try_encrypt(500 as u64, &fhe_public_key)
            .unwrap()
            .compress();
        bincode::serialize(&amount).unwrap()
    };
    let mint_request = json!({
        "Mint": {
            "mint_to_address": "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94",
            "coins": {
                "amount": mint_amount,
                "token_id": "token_1p0cc94vkffzsyy8xdtmgu70h2lxg85zrqcns7dzaz2pqlt3w3ypq2duf6l",
            },
        },
    });

    // transfer request
    let transfer_amount = {
        let amount = FheUint64::try_encrypt(100 as u64, &fhe_public_key)
            .unwrap()
            .compress();
        bincode::serialize(&amount).unwrap()
    };
    let transfer_request = json!({
        "Transfer": {
            "to": "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqwr57gc",
            "coins": {
                "amount": transfer_amount,
                "token_id": "token_1p0cc94vkffzsyy8xdtmgu70h2lxg85zrqcns7dzaz2pqlt3w3ypq2duf6l",
            },
        },
    });

    // write the requests
    fs::write(
        requests_path.join("create_token.json"),
        create_token_request.to_string(),
    )
    .unwrap();
    fs::write(requests_path.join("mint.json"), mint_request.to_string()).unwrap();
    fs::write(
        requests_path.join("transfer.json"),
        transfer_request.to_string(),
    )
    .unwrap();

    println!(
        "[Init] Requests generated and serialized in {:?}\n[Init] Requests are stored in {:?}",
        start.elapsed(),
        requests_path
    );
}
