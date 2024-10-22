use crate::fee::CyferioFee;
use crate::spec::address::CyferioAddress;
use crate::spec::block::CyferioBlock;
use crate::spec::hash::CyferioHash;
use crate::spec::header::CyferioHeader;
use crate::spec::transaction::CyferioBlobTransaction;
use crate::spec::CyferioDaLayerSpec;
use crate::verifier::CyferioDaVerifier;
use anyhow::Error;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use parking_lot::Mutex;
use sov_rollup_interface::da::{DaBlobHash, DaSpec, RelevantBlobs, RelevantProofs};
use sov_rollup_interface::services::da::{DaService, MaybeRetryable};
use subxt::config::Header;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use subxt::backend::legacy::rpc_methods::BlockNumber;
use subxt::backend::{legacy::LegacyRpcMethods, rpc::RpcClient};
use subxt::{OnlineClient, SubstrateConfig};
use subxt_signer::sr25519::dev;
use tokio::time::{sleep, Duration};

#[subxt::subxt(runtime_metadata_path = "./src/metadata.scale")]
pub mod substrate {}

type StatemintConfig = SubstrateConfig;

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CyferioConfig {
    pub node_url: String,
    pub sender_address: CyferioAddress,
}

#[derive(Clone)]
pub struct DaProvider {
    pub rpc_client: RpcClient,
    pub client: Arc<OnlineClient<StatemintConfig>>,
    pub rpc: Arc<LegacyRpcMethods<StatemintConfig>>,
    last_processed_block: Arc<AtomicU64>,
    processed_blocks: Arc<Mutex<HashSet<u64>>>,
    last_processed_height: Arc<AtomicU64>,
}

impl DaProvider {
    pub async fn from_config(config: CyferioConfig) -> Result<Self, Error> {
        let rpc_client = RpcClient::from_url(&config.node_url).await?;
        let client = OnlineClient::<StatemintConfig>::from_rpc_client(rpc_client.clone()).await?;
        let rpc = LegacyRpcMethods::<StatemintConfig>::new(rpc_client.clone());
        Ok(Self {
            rpc_client,
            client: Arc::new(client),
            rpc: Arc::new(rpc),
            last_processed_block: Arc::new(AtomicU64::new(0)),
            processed_blocks: Arc::new(Mutex::new(HashSet::new())),
            last_processed_height: Arc::new(AtomicU64::new(0)),
        })
    }

    pub async fn new(config: CyferioConfig) -> Result<Self, MaybeRetryable<Error>> {
        let rpc_client = RpcClient::from_url(&config.node_url)
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        let client = OnlineClient::<StatemintConfig>::from_rpc_client(rpc_client.clone())
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        let rpc = LegacyRpcMethods::<StatemintConfig>::new(rpc_client.clone());

        Ok(Self {
            rpc_client,
            client: Arc::new(client),
            rpc: Arc::new(rpc),
            last_processed_block: Arc::new(AtomicU64::new(0)),
            processed_blocks: Arc::new(Mutex::new(HashSet::new())),
            last_processed_height: Arc::new(AtomicU64::new(0)),
        })
    }

    async fn wait_for_block(&self, height: u64) -> Result<CyferioBlock, MaybeRetryable<Error>> {
        let polling_interval = Duration::from_secs(1);
        let max_retries = 60; // Adjust as needed
        let mut retries = 0;

        loop {
            match self.get_block_inner(height).await {
                Ok(block) => return Ok(block),
                Err(MaybeRetryable::Transient(e))
                    if e.to_string().contains("Block already processed") =>
                {
                    if retries >= max_retries {
                        return Err(MaybeRetryable::Transient(Error::msg(
                            "Max retries reached while waiting for new block",
                        )));
                    }
                    retries += 1;
                    sleep(polling_interval).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn get_block_inner(&self, height: u64) -> Result<CyferioBlock, MaybeRetryable<Error>> {
        let last_processed = self.last_processed_height.load(Ordering::SeqCst);
        if height <= last_processed {
            return Err(MaybeRetryable::Transient(Error::msg(
                "Block already processed",
            )));
        }

        {
            let mut processed_blocks = self.processed_blocks.lock();
            if processed_blocks.contains(&height) {
                println!("Skipping already processed block at height: {}", height);
                return Err(MaybeRetryable::Transient(Error::msg(
                    "Block already processed",
                )));
            }
            processed_blocks.insert(height);
        }

        let last_processed = self.last_processed_block.load(Ordering::SeqCst);
        if height <= last_processed {
            println!("Warning: Processing a block with height {} less than or equal to last processed height {}", height, last_processed);
        }

        let current_hash = self
            .rpc
            .chain_get_block_hash(Some(BlockNumber::Number(height)))
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        println!("current_hash: {:?}", current_hash);
        let block_detail = self
            .rpc
            .chain_get_block(current_hash)
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        match block_detail {
            Some(block) => {
                let header = block.block.header;
                let parent_hash = header.parent_hash;
                let state_root = header.hash();
                let extrinsics_root = header.extrinsics_root;
                let number = header.number;
                let digest = header.digest.clone();
                let cyferio_header = CyferioHeader::new(
                    number,
                    CyferioHash::from(parent_hash.0),
                    CyferioHash::from(state_root.0),
                    CyferioHash::from(extrinsics_root.0),
                    digest,
                );
                let transactions: Vec<CyferioBlobTransaction> = block
                    .block
                    .extrinsics
                    .into_iter()
                    .map(|extrinsic| CyferioBlobTransaction::from(extrinsic.0))
                    .collect();
                let cyferio_block = CyferioBlock::new(cyferio_header, transactions);

                // Update last_processed_block only if the new height is greater
                self.last_processed_block
                    .fetch_max(height, Ordering::SeqCst);
                self.last_processed_height.store(height, Ordering::SeqCst);
                Ok(cyferio_block)
            }
            None => Err(MaybeRetryable::Transient(Error::msg("Block not found"))),
        }
    }
}

#[async_trait]
impl DaService for DaProvider {
    type Spec = CyferioDaLayerSpec;
    type Verifier = CyferioDaVerifier;
    type FilteredBlock = CyferioBlock;
    type HeaderStream = BoxStream<'static, Result<CyferioHeader, Self::Error>>;
    type Error = MaybeRetryable<Error>;
    type Fee = CyferioFee;

    async fn get_block_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        self.wait_for_block(height).await
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let finalized_hash = self
            .rpc
            .chain_get_finalized_head()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let block_detail = self
            .rpc
            .chain_get_block(Some(finalized_hash))
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        match block_detail {
            Some(block) => {
                let header = block.block.header;
                let parent_hash = header.parent_hash;
                let state_root = header.state_root;
                let extrinsics_root = header.extrinsics_root;
                let number = header.number;
                let digest = header.digest.clone();
                let cyferio_header = CyferioHeader::new(
                    number,
                    CyferioHash::from(parent_hash.0),
                    CyferioHash::from(state_root.0),
                    CyferioHash::from(extrinsics_root.0),
                    digest,
                );
                Ok(cyferio_header)
            }
            None => Err(MaybeRetryable::Transient(Error::msg("Block not found"))),
        }
    }

    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        let last_processed_block = self.last_processed_block.clone();
        Ok(self
            .client
            .blocks()
            .subscribe_finalized()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .filter_map(move |block_res| {
                futures::future::ready(match block_res {
                    Ok(block) => {
                        let header = block.header();
                        let number = header.number;
                        let last_processed = last_processed_block.load(Ordering::SeqCst);
                        if u64::from(number) > last_processed {
                            last_processed_block.store(u64::from(number), Ordering::SeqCst);
                            Some(Ok(CyferioHeader::from(header)))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(MaybeRetryable::Transient(Error::from(e)))),
                })
            })
            .boxed())
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let node_client = self.client.clone();
        let latest_block = node_client
            .blocks()
            .at_latest()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        Ok(CyferioHeader::from(latest_block.header()))
    }

    fn extract_relevant_blobs(
        &self,
        block: &Self::FilteredBlock,
    ) -> RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction> {
        block.as_relevant_blobs()
    }

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
        let alice_pair_signer = dev::alice();

        // Construct the offchainWorker submitTask call
        let da_height = self
            .client
            .blocks()
            .at_latest()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .header()
            .number as u64;

        let submit_task_tx = substrate::tx()
            .offchain_worker()
            .submit_task(da_height, blob.to_vec());

        // Submit the transaction and wait for confirmation
        let tx_progress = self
            .client
            .tx()
            .sign_and_submit_then_watch_default(&submit_task_tx, &alice_pair_signer)
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        // Get the transaction hash
        let tx_hash = tx_progress.extrinsic_hash();

        // Wait for the transaction to be finalized
        let events = tx_progress
            .wait_for_finalized_success()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        // Look for the relevant event
        let task_submitted_event = events
            .find_first::<substrate::offchain_worker::events::TaskSubmitted>()
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        if task_submitted_event.is_some() {
            Ok(CyferioHash::from(tx_hash.0))
        } else {
            Err(MaybeRetryable::Transient(anyhow::Error::msg(
                "Task submission event not found",
            )))
        }
    }

    async fn send_aggregated_zk_proof(
        &self,
        _aggregated_proof_data: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        Ok(CyferioHash::from([0u8; 32]))
    }

    async fn get_aggregated_proofs_at(&self, _height: u64) -> Result<Vec<Vec<u8>>, Self::Error> {
        Ok(vec![vec![0u8]])
    }

    async fn estimate_fee(&self, _blob_size: usize) -> Result<Self::Fee, Self::Error> {
        Ok(CyferioFee::new(0u128))
    }
}
