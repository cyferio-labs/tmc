use crate::fee::SuiFee;
use crate::spec::address::SuiAddress;
use crate::spec::block::SuiBlock;
use crate::spec::hash::SuiHash;
use crate::spec::header::SuiHeader;
use crate::spec::transaction::{Transaction, TransactionIdentifier};
use crate::spec::SuiDaLayerSpec;
use crate::verifier::SuiDaVerifier;
use anyhow::Error;
use async_trait::async_trait;
// use borsh::BorshDeserialize;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use reqwest::Client;
use serde_json::json;
use sov_rollup_interface::da::{DaBlobHash, DaSpec, RelevantBlobs, RelevantProofs};
use sov_rollup_interface::services::da::{DaService, MaybeRetryable};
use std::sync::Arc;
use sui_json_rpc_types::{Checkpoint, EventFilter, SuiEvent, SuiTransactionBlockResponseOptions};
use sui_sdk::{SuiClient, SuiClientBuilder};

/// Runtime configuration for the DA service
#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SuiConfig {
    pub node_client_url: String,
    pub sender_address: SuiAddress,
}

#[derive(Clone)]
pub struct DaProvider {
    pub client: SuiClient,
}

impl DaProvider {
    pub async fn from_config(config: SuiConfig) -> Self {
        let client = SuiClientBuilder::default()
            .build(config.node_client_url) // local network address
            .await
            .expect("Failed to create client with sui");
        Self { client }
    }

    pub async fn new(config: SuiConfig) -> Self {
        let client = SuiClientBuilder::default()
            .build(config.node_client_url) // local network address
            .await
            .expect("Failed to create client with sui");
        Self { client }
    }
    async fn create_block_response(
        &self,
        checkpoint: Checkpoint,
    ) -> Result<SuiBlock, MaybeRetryable<Error>> {
        let index = checkpoint.sequence_number;
        let _hash = checkpoint.digest;
        let mut transactions = vec![];
        for batch in checkpoint.transactions.chunks(50) {
            let transaction_responses = self
                .client
                .read_api()
                .multi_get_transactions_with_options(
                    batch.to_vec(),
                    SuiTransactionBlockResponseOptions::new()
                        .with_input()
                        .with_effects()
                        .with_balance_changes()
                        .with_events(),
                )
                .await
                .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;
            for tx in transaction_responses.into_iter() {
                transactions.push(Transaction {
                    transaction_identifier: TransactionIdentifier { hash: tx.digest },
                    // operations: Operations::try_from(tx)?,
                    related_transactions: vec![],
                    metadata: None,
                })
            }
        }

        // previous digest should only be None for genesis block.
        if checkpoint.previous_digest.is_none() && index != 0 {
            return Err(MaybeRetryable::Transient(Error::msg(
                "Invalid keypair type",
            ))); // 显式返回错误
        }

        // let _parent_block_identifier = checkpoint
        //     .previous_digest
        //     .map(|hash| BlockIdentifier {
        //         index: index - 1,
        //         hash,
        //     })
        //     .unwrap_or_else(|| BlockIdentifier { index, hash });

        // let _parent_block_identifier = checkpoint
        //     .previous_digest
        //     .map(|hash| {
        //         if index > 0 {
        //             BlockIdentifier {
        //                 index: index - 1,
        //                 hash,
        //             }
        //         } else {
        //             return Err(MaybeRetryable::Transient(Error::msg("Invalid index for previous block")));
        //         }
        //     })
        //     .unwrap_or_else(|| BlockIdentifier { index, hash });
        if index == 0 {
            return Err(MaybeRetryable::Transient(Error::msg(
                "Index cannot be zero",
            ))); // 显式返回错误
        }

        println!("Checkpoint index: {}", index);

        let header: SuiHeader = SuiHeader::new(SuiHeader {
            prev_hash: SuiHash::try_from(index - 1).unwrap(),
            hash: SuiHash::try_from(index).unwrap(),
            height: index,
        });
        Ok(SuiBlock {
            header,
            validity_cond: Default::default(),
            batch_blobs: vec![],
            proof_blobs: vec![],
        })
    }

    async fn extract_header_from_event(&self, _event: SuiEvent) -> Result<SuiHeader, Error> {
        // 根据事件类型提取区块头信息的逻辑
        // 返回相应的 HeaderType
        let checkpoint_number = self
            .client
            .read_api()
            .get_latest_checkpoint_sequence_number()
            .await?;
        let checkpoint = self
            .client
            .read_api()
            .get_checkpoint(checkpoint_number.into())
            .await?;
        let sui_block = self.create_block_response(checkpoint).await?;
        Ok(sui_block.header)
    }
}

#[async_trait]
impl DaService for DaProvider {
    type Spec = SuiDaLayerSpec;

    type Verifier = SuiDaVerifier;

    type FilteredBlock = SuiBlock;

    type HeaderStream = BoxStream<'static, Result<SuiHeader, Self::Error>>;

    type Error = MaybeRetryable<Error>;

    type Fee = SuiFee;

    // Make an RPC call to the node to get the block at the given height, if one exists.
    // If no such block exists, block until one does.
    async fn get_block_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        let checkpoint = self
            .client
            .read_api()
            .get_checkpoint(height.into())
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;
        Ok(self
            .create_block_response(checkpoint)
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?)
        // Ok(Default::default())
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let checkpoint_number = self
            .client
            .read_api()
            .get_latest_checkpoint_sequence_number()
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        let checkpoint = self
            .client
            .read_api()
            .get_checkpoint(checkpoint_number.into())
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        let sui_block = self
            .create_block_response(checkpoint)
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        Ok(sui_block.header)
    }
    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        let this = Arc::new(self.clone()); // 克隆 self 为 Arc

        // 初始化 WebSocket 客户端
        let sui_client = SuiClientBuilder::default()
            .ws_url("wss://rpc.testnet.sui.io:443")
            .build("https://fullnode.testnet.sui.io:443")
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        println!("WS version {:?}", sui_client.api_version());

        // 订阅事件
        let subscribe_all = sui_client
            .event_api()
            .subscribe_event(EventFilter::All(vec![]))
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        // 创建一个流来处理接收到的区块头
        let stream = futures::stream::unfold(subscribe_all, {
            let this = Arc::clone(&this);
            move |mut receiver| {
                let this = this.clone();
                async move {
                    match receiver.next().await {
                        Some(Ok(event)) => {
                            let header_result = this.extract_header_from_event(event).await;
                            match header_result {
                                Ok(header) => Some((Ok(header), receiver)),
                                Err(_) => {
                                    // 处理错误，您可以选择记录或忽略
                                    None
                                }
                            }
                        }
                        Some(Err(_)) => {
                            // 处理错误
                            None
                        }
                        None => {
                            // 流结束
                            None
                        }
                    }
                }
            }
        });

        // 返回流，确保它是 boxed 的
        Ok(stream.boxed())
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        self.get_last_finalized_block_header().await
    }

    // Extract the blob transactions relevant to a particular rollup from a block.
    // NOTE: The avail light client is expected to be run in app specific mode, and hence the
    // transactions in the block are already filtered and retrieved by light client.
    fn extract_relevant_blobs(
        &self,
        block: &Self::FilteredBlock,
    ) -> RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction> {
        block.as_relevant_blobs()
    }

    // Extract the inclusion and completeness proof for filtered block provided.
    // The output of this method will be passed to the verifier.
    // NOTE: The light client here has already completed DA sampling and verification of inclusion and soundness.
    async fn get_extraction_proof(
        &self,
        block: &Self::FilteredBlock,
        _blobs: &RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction>,
    ) -> RelevantProofs<
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    > {
        block.get_relevant_proofs()
    }

    async fn send_transaction(
        &self,
        blob: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        let publisher = "https://publisher-devnet.walrus.space"; // 替换为实际的 URL
        let endpoint = format!("{}/v1/store", publisher);

        // 使用 from_utf8_lossy 处理无效 UTF-8
        let data = String::from_utf8_lossy(blob).to_string();

        // 如果 API 需要 JSON 格式，使用 json! 创建请求体
        let json_data = json!({
            "data": data
        });

        println!("json_data------------{:?}", json_data);

        let client = Client::new();
        let response = client
            .put(&endpoint)
            .json(&json_data) // 使用 JSON 格式的请求体
            .send()
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;

        // 检查响应状态
        if response.status().is_success() {
            println!("Request was successful!");
        } else {
            eprintln!("Failed to send request: {}", response.status());
        }

        let response_text = response.text().await.map_err(|e| {
            eprintln!("Failed to read response text: {}", e);
            MaybeRetryable::Transient(anyhow::Error::from(e))
        })?;

        println!("Response content------------: {}", response_text);
        let sui_hash: SuiHash = SuiHash::try_from(0u64).unwrap();
        Ok(sui_hash)

        // let sui_client = SuiClientBuilder::default()
        //     .build_testnet()
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;
        //
        // let private_key_str =
        //     "suiprivkey1qr78zgu2cwcpdma2t50xrsx04z5ekma0tnjyfvl69rp0mdv996q5xkn5wu6";
        //
        // let sui_keypair =
        //     SuiKeyPair::decode(private_key_str).map_err(|e| anyhow!("解码失败: {:?}", e))?;
        //
        // // 确保是 Ed25519 类型
        // let ed25519_keypair = if let SuiKeyPair::Ed25519(keypair) = sui_keypair {
        //     keypair
        // } else {
        //     return Err(MaybeRetryable::Transient(anyhow::Error::msg(
        //         "Invalid keypair type",
        //     ))); // 显式返回错误
        // };
        //
        // let pk = ed25519_keypair.public();
        // println!("公钥: {:?}", pk);
        //
        // let sender = DaAddress::try_from(pk).unwrap();
        // println!("Sender: {:?}", sender);
        //
        // println!("blob----------------------------------------------------------------------------------------------");
        // println!("blob----------------------------------------------------------------------------------------------");
        // println!("blob----------------------------------------------------------------------------------------------");
        // println!("blob----------------------------------------------------------------------------------------------: {:?}", blob);
        // // 获取 gas_coin
        // // let gas_coin = sui_client
        // //     .coin_read_api()
        // //     .get_coins(sender, None, None, None)
        // //     .await?
        // //     .data
        // //     .into_iter()
        // //     .next()
        // //     .ok_or_else(|| MaybeRetryable::Transient(anyhow::Error::msg("No coins found")))?;
        // let gas_coin = sui_client
        //     .coin_read_api()
        //     .get_coins(sender, None, None, None)
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))? // 显式处理错误
        //     .data
        //     .into_iter()
        //     .next()
        //     .ok_or_else(|| MaybeRetryable::Transient(anyhow::Error::msg("No coins found")))?; // 处理 Option 类型
        //
        // // 构建可编程交易
        // let input_value = 10u64;
        // let input_argument = CallArg::Pure(bcs::to_bytes(&input_value).unwrap());
        //
        // let mut builder = ProgrammableTransactionBuilder::new();
        // builder.input(input_argument)?;
        //
        // let pkg_id = "0x883393ee444fb828aa0e977670cf233b0078b41d144e6208719557cb3888244d";
        // let package = ObjectID::from_hex_literal(pkg_id).map_err(|e| anyhow!(e))?;
        // let module = Identifier::new("hello_world").map_err(|e| anyhow!(e))?;
        // let function = Identifier::new("hello_world").map_err(|e| anyhow!(e))?;
        //
        // builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        //     package,
        //     module,
        //     function,
        //     type_arguments: vec![],
        //     arguments: vec![Argument::Input(0)],
        // })));
        // let ptb = builder.finish();
        //
        // let gas_budget = 10_000_000;
        //
        // // let gas_price = sui_client.read_api().get_reference_gas_price().await?;
        //
        // let gas_price = sui_client
        //     .read_api()
        //     .get_reference_gas_price()
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;
        //
        // // 创建交易数据
        // let tx_data = TransactionData::new_programmable(
        //     sender,
        //     vec![gas_coin.object_ref()],
        //     ptb,
        //     gas_budget,
        //     gas_price,
        // );
        //
        // // 计算需要签名的摘要
        // let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
        // let raw_tx = bcs::to_bytes(&intent_msg).expect("bcs should not fail");
        // let mut hasher = sui_types::crypto::DefaultHash::default();
        // hasher.update(raw_tx.clone());
        // let digest = hasher.finalize().digest;
        // let sui_sig = ed25519_keypair.sign(&digest);
        //
        // println!("Executing the transaction...");
        // let transaction_response = sui_client
        //     .quorum_driver_api()
        //     .execute_transaction_block(
        //         sui_types::transaction::Transaction::from_generic_sig_data(
        //             intent_msg.value,
        //             vec![GenericSignature::Signature(sui_sig)],
        //         ),
        //         SuiTransactionBlockResponseOptions::default(),
        //         None,
        //     )
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?; // 显式处理错误
        //
        // println!(
        //     "Transaction executed. Transaction digest: {}",
        //     transaction_response.digest.base58_encode()
        // );
        // println!("{}", transaction_response);
        // Ok(())
    }

    async fn send_aggregated_zk_proof(
        &self,
        _aggregated_proof_data: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        let sui_hash: SuiHash = SuiHash::try_from(0u64).unwrap();
        Ok(sui_hash)
    }

    async fn get_aggregated_proofs_at(&self, _height: u64) -> Result<Vec<Vec<u8>>, Self::Error> {
        Ok(vec![vec![0u8]])
    }

    async fn estimate_fee(&self, _blob_size: usize) -> Result<Self::Fee, Self::Error> {
        Ok(SuiFee::zero())
    }
}
