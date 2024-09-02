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
use serde_json::Value;
use anyhow::anyhow;
use base58::FromBase58;

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
        let json: Value = serde_json::from_str(&response_text).map_err(|e| {
            eprintln!("Failed to parse JSON: {}", e);
            MaybeRetryable::Transient(anyhow::Error::from(e))
        })?;

        println!("Response json------------: {}", json);
        let tx_digest = json["alreadyCertified"]["event"]["txDigest"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing txDigest"))?; // 使用 anyhow 创建错误
        println!("Response tx_digest------------: {}", tx_digest);

        let bytes = tx_digest.from_base58().map_err(|e| {
            anyhow!("Failed to decode Base58: {:?}", e)
        })?;

        let hash: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
            anyhow!("Decoded bytes must have length 32, but it has {}", bytes.len())
        })?;

        let sui_hash = SuiHash(hash);
        println!("Extracted SuiHash: {:?}", sui_hash);
        Ok(sui_hash)
    }

    async fn send_aggregated_zk_proof(
        &self,
        _aggregated_proof_data: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        // let publisher = "https://publisher-devnet.walrus.space"; // 替换为实际的 URL
        // let endpoint = format!("{}/v1/store", publisher);
        //
        // // 使用 from_utf8_lossy 处理无效 UTF-8
        // let data = String::from_utf8_lossy(aggregated_proof_data).to_string();
        //
        // // 如果 API 需要 JSON 格式，使用 json! 创建请求体
        // let json_data = json!({
        //     "data": data
        // });
        //
        // println!("json_data------------{:?}", json_data);
        //
        // let client = Client::new();
        // let response = client
        //     .put(&endpoint)
        //     .json(&json_data) // 使用 JSON 格式的请求体
        //     .send()
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?;
        //
        // // 检查响应状态
        // if response.status().is_success() {
        //     println!("Request was successful!");
        // } else {
        //     eprintln!("Failed to send request: {}", response.status());
        // }
        //
        // let response_text = response.text().await.map_err(|e| {
        //     eprintln!("Failed to read response text: {}", e);
        //     MaybeRetryable::Transient(anyhow::Error::from(e))
        // })?;
        //
        // println!("Response content------------: {}", response_text);
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
