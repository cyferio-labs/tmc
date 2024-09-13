use anyhow::{anyhow, Result};
use fastcrypto::hash::HashFunction;
use shared_crypto::intent::{Intent, IntentMessage};
use sui_json_rpc_types::SuiTransactionBlockResponseOptions;
use sui_sdk::rpc_types::SuiObjectDataOptions;
use sui_sdk::types::{
    base_types::{ObjectID, SequenceNumber, SuiAddress},
    crypto::SuiKeyPair,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, Command, ObjectArg, ProgrammableMoveCall, TransactionData},
    Identifier,
};
use sui_sdk::SuiClientBuilder;
use sui_types::crypto::DefaultHash;
use sui_types::crypto::KeypairTraits;
use sui_types::crypto::Signer;
use sui_types::signature::GenericSignature;

pub async fn send_transaction() -> Result<()> {
    // 1) 获取 Sui 客户端
    let sui_client = SuiClientBuilder::default().build_testnet().await?;

    // let node_client_url = "https://rpc-testnet.suiscan.xyz:443";
    // let sui_client = SuiClientBuilder::default()
    //     .build(node_client_url)
    //     .await
    //     .map_err(|e| anyhow!("Failed to create Sui client: {:?}", e))?;

    // 验证连接
    let _ = sui_client
        .read_api()
        .get_reference_gas_price()
        .await
        .map_err(|e| anyhow!("Failed to connect to Sui network: {:?}", e))?;

    println!("Successfully connected to Sui network");

    let private_key_str = "suiprivkey1qzlmtflas9pd0lqxr7wyx9u7rdczm2w9ecax2fkwmgx3y407zelj2dz8024";

    let sui_keypair =
        SuiKeyPair::decode(private_key_str).map_err(|e| anyhow!("解码失败: {:?}", e))?;

    // 确保是 Ed25519 类型
    let ed25519_keypair = match sui_keypair {
        SuiKeyPair::Ed25519(keypair) => keypair,
        _ => return Err(anyhow!("解码的密钥对不是 Ed25519 类型")),
    };

    let pk = ed25519_keypair.public();
    println!("公钥: {:?}", pk);

    let sender = SuiAddress::try_from(pk).map_err(|e| anyhow!("地址转换失败: {:?}", e))?;
    println!("Sender: {:?}", sender);

    // 获取 gas_coin
    let gas_coin = sui_client
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?
        .data
        .into_iter()
        .next()
        .ok_or(anyhow!("未找到发送者的 coin"))?;

    // 获取 Walrusda 对象
    let walrus_da_object_id = ObjectID::from_hex_literal(
        "0x6c58237c52be94c62791f85f45573dc0578cddb7eaa81184d94198e9eb283b2f",
    )?;

    let sui_data_options = SuiObjectDataOptions::default();

    // 在交易执行前立即获取最新的对象信息
    let walrus_da_object_with_options = sui_client
        .read_api()
        .get_object_with_options(walrus_da_object_id, sui_data_options.clone())
        .await?;

    let (object_id, version, _digest) =
        walrus_da_object_with_options.object().unwrap().object_ref();

    let past_walrus_da_object = sui_client
        .read_api()
        .try_get_parsed_past_object(object_id, version, sui_data_options.clone())
        .await?;
    println!(" *** Past Object *** ");
    println!("{:?}", past_walrus_da_object);
    println!(" *** Past Object ***\n");

    let sui_get_past_object_request = past_walrus_da_object.clone().into_object()?;

    let walrus_da_object = CallArg::Object(ObjectArg::SharedObject {
        id: sui_get_past_object_request.object_id,
        initial_shared_version: SequenceNumber::from(127634460u64),
        mutable: true,
    });

    println!(
        "initial_shared_version: {:?}",
        sui_get_past_object_request.version.clone()
    );

    // 构建可编程交易
    let da_height = 0u64;
    let blob: Vec<u8> = vec![1, 2, 3, 4, 5];

    let da_height_argument = CallArg::Pure(bcs::to_bytes(&da_height)?);
    let blob_argument = CallArg::Pure(bcs::to_bytes(&blob)?);

    let mut builder = ProgrammableTransactionBuilder::new();
    builder.input(walrus_da_object)?;
    builder.input(da_height_argument)?;
    builder.input(blob_argument)?;

    let pkg_id = "0xaaabcffd2ab47f61a287110cb5626045d5a9ceea2cc8618841f124bccd9972cd";
    let package = ObjectID::from_hex_literal(pkg_id)?;
    let module = Identifier::new("walrus_da")?;
    let function = Identifier::new("add_blob")?;

    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package,
        module,
        function,
        type_arguments: vec![],
        arguments: vec![
            Argument::Input(0), // Walrusda object
            Argument::Input(1), // da_height
            Argument::Input(2), // blob
        ],
    })));
    let ptb = builder.finish();

    let gas_budget = 100_000_000;
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
    let mut hasher = DefaultHash::default();
    hasher.update(raw_tx.clone());
    let digest = hasher.finalize().digest;
    let sui_sig = ed25519_keypair.sign(&digest);

    println!("Executing the transaction...");
    match sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            sui_types::transaction::Transaction::from_generic_sig_data(
                intent_msg.value,
                vec![GenericSignature::Signature(sui_sig)],
            ),
            SuiTransactionBlockResponseOptions::default(),
            None,
        )
        .await
    {
        Ok(transaction_response) => {
            println!(
                "Transaction executed. Transaction digest: {}",
                transaction_response.digest.base58_encode()
            );
            println!("{}", transaction_response);
            Ok(())
        }
        Err(e) => {
            println!("Error executing transaction: {:?}", e);
            Err(anyhow!("Transaction execution failed: {:?}", e))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    match send_transaction().await {
        Ok(_) => println!("Transaction sent successfully"),
        Err(e) => println!("Error sending transaction: {:?}", e),
    }
    Ok(())
}
