use anyhow::anyhow;
use fastcrypto::hash::HashFunction;
use fastcrypto::traits::{KeyPair, ToFromBytes};
use shared_crypto::intent::{Intent, IntentMessage};
use sui_json_rpc_types::SuiTransactionBlockResponseOptions;
use sui_sdk::types::{
    base_types::{ObjectID, SuiAddress},
    crypto::SuiKeyPair,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, Command, ProgrammableMoveCall, TransactionData},
    Identifier,
};
use sui_sdk::SuiClientBuilder;
use sui_types::crypto::Signer;
use sui_types::signature::GenericSignature;

pub async fn send_transaction() -> Result<(), anyhow::Error> {
    // 1) 获取 Sui 客户端
    let sui_client = SuiClientBuilder::default().build_testnet().await?;

    let private_key_str = "suiprivkey1qr78zgu2cwcpdma2t50xrsx04z5ekma0tnjyfvl69rp0mdv996q5xkn5wu6";

    let sui_keypair =
        SuiKeyPair::decode(private_key_str).map_err(|e| anyhow!("解码失败: {:?}", e))?;

    // 确保是 Ed25519 类型
    let ed25519_keypair = if let SuiKeyPair::Ed25519(keypair) = sui_keypair {
        keypair
    } else {
        return Err(anyhow!("解码的密钥对不是 Ed25519 类型"));
    };

    let pk = ed25519_keypair.public();
    println!("公钥: {:?}", pk);

    let address: &[u8] = pk.as_bytes();
    println!("address: {:?}", address);

    let sender = SuiAddress::try_from(pk).unwrap();
    println!("Sender: {:?}", sender);

    // 获取 gas_coin
    let gas_coin = sui_client
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?
        .data
        .into_iter()
        .next()
        .ok_or(anyhow!("No coins found for sender"))?;

    // 构建可编程交易
    let input_value = 10u64;
    let input_argument = CallArg::Pure(bcs::to_bytes(&input_value).unwrap());

    let mut builder = ProgrammableTransactionBuilder::new();
    builder.input(input_argument)?;

    let pkg_id = "0x883393ee444fb828aa0e977670cf233b0078b41d144e6208719557cb3888244d";
    let package = ObjectID::from_hex_literal(pkg_id).map_err(|e| anyhow!(e))?;
    let module = Identifier::new("hello_world").map_err(|e| anyhow!(e))?;
    let function = Identifier::new("hello_world").map_err(|e| anyhow!(e))?;

    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package,
        module,
        function,
        type_arguments: vec![],
        arguments: vec![Argument::Input(0)],
    })));
    let ptb = builder.finish();

    let gas_budget = 10_000_000;
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;

    // 创建交易数据
    let tx_data = TransactionData::new_programmable(
        sender,
        vec![gas_coin.object_ref()],
        ptb,
        gas_budget,
        gas_price,
    );

    // 计算需要签名的摘要
    let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
    let raw_tx = bcs::to_bytes(&intent_msg).expect("bcs should not fail");
    let mut hasher = sui_types::crypto::DefaultHash::default();
    hasher.update(raw_tx.clone());
    let digest = hasher.finalize().digest;
    let sui_sig = ed25519_keypair.sign(&digest);

    println!("Executing the transaction...");
    let transaction_response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            sui_types::transaction::Transaction::from_generic_sig_data(
                intent_msg.value,
                vec![GenericSignature::Signature(sui_sig)],
            ),
            SuiTransactionBlockResponseOptions::default(),
            None,
        )
        .await?;

    println!(
        "Transaction executed. Transaction digest: {}",
        transaction_response.digest.base58_encode()
    );
    println!("{}", transaction_response);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Sui testnet -- https://fullnode.testnet.sui.io:443
    // let sui_testnet = SuiClientBuilder::default().build_testnet().await?;
    // println!("Sui testnet version: {}", sui_testnet.api_version());

    // // Sui devnet -- https://fullnode.devnet.sui.io:443
    // let sui_devnet = SuiClientBuilder::default().build_devnet().await?;
    // println!("Sui devnet version: {}", sui_devnet.api_version());
    let _a = send_transaction().await;
    Ok(())
}
