use crate::fee::CyferioFee;
use crate::spec::address::CyferioAddress;
use crate::spec::block::CyferioBlock;
use crate::spec::hash::CyferioHash;
use crate::spec::header::CyferioHeader;
use crate::spec::CyferioDaLayerSpec;
use crate::verifier::CyferioDaVerifier;
use anyhow::Error;
use async_trait::async_trait;
use codec::Decode;
use futures::stream::BoxStream;
use futures::StreamExt;
use sov_rollup_interface::da::{DaBlobHash, DaSpec, RelevantBlobs, RelevantProofs};
use sov_rollup_interface::services::da::{DaService, MaybeRetryable};
use subxt::backend::legacy::rpc_methods::BlockNumber;
use subxt::backend::{legacy::LegacyRpcMethods, rpc::RpcClient};
use subxt::{OnlineClient, SubstrateConfig};
use subxt_signer::sr25519::dev;

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
    pub client: OnlineClient<StatemintConfig>,
}

impl DaProvider {
    pub async fn from_config(config: CyferioConfig) -> Result<Self, Error> {
        let client = OnlineClient::<StatemintConfig>::from_url(&config.node_url)
            .await
            .map_err(Error::from)?;
        Ok(Self { client })
    }

    pub async fn new(config: CyferioConfig) -> Result<Self, Error> {
        let client = OnlineClient::<StatemintConfig>::from_url(&config.node_url)
            .await
            .map_err(Error::from)?;
        let provider = Self { client };

        Ok(provider)
    }

    async fn get_latest_block(&self) -> Result<CyferioBlock, Error> {
        let block = self.client.blocks().at_latest().await?;
        let _block_hash = block.hash();
        Ok(CyferioBlock::default()) // Temporary placeholder
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
        let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944")
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        let rpc = LegacyRpcMethods::<StatemintConfig>::new(rpc_client.clone());
        // let api = OnlineClient::<StatemintConfig>::from_rpc_client(rpc_client.clone())
        //     .await
        //     .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let current_hash = rpc
            .chain_get_block_hash(Some(BlockNumber::Number(height)))
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let block_detail = rpc
            .chain_get_block(current_hash)
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

                // Extract timestamp from extrinsics
                let mut timestamp: u64 = 0;
                for ext in block.block.extrinsics.iter() {
                    if let Ok(call) = substrate::Call::decode(&mut ext.as_ref()) {
                        if let substrate::Call::Timestamp(substrate::timestamp::Call::set { now }) =
                            call
                        {
                            timestamp = now;
                            break;
                        }
                    }
                }

                let cyferio_header = CyferioHeader::new(
                    number,
                    CyferioHash::from(parent_hash.0),
                    CyferioHash::from(state_root.0),
                    CyferioHash::from(extrinsics_root.0),
                    digest,
                    timestamp,
                );
                let cyferio_block = CyferioBlock::new(cyferio_header);
                Ok(cyferio_block)
            }
            None => Err(MaybeRetryable::Transient(Error::msg("Block not found"))),
        }
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944")
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        let rpc = LegacyRpcMethods::<StatemintConfig>::new(rpc_client.clone());

        let finalized_hash = rpc
            .chain_get_finalized_head()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let block_detail = rpc
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

                // Extract timestamp from extrinsics
                let mut timestamp: u64 = 0;
                for ext in block.block.extrinsics.iter() {
                    if let Ok(call) = substrate::Call::decode(&mut ext.as_ref()) {
                        if let substrate::Call::Timestamp(substrate::timestamp::Call::set { now }) =
                            call
                        {
                            timestamp = now;
                            break;
                        }
                    }
                }

                let cyferio_header = CyferioHeader::new(
                    number,
                    CyferioHash::from(parent_hash.0),
                    CyferioHash::from(state_root.0),
                    CyferioHash::from(extrinsics_root.0),
                    digest,
                    timestamp,
                );
                Ok(cyferio_header)
            }
            None => Err(MaybeRetryable::Transient(Error::msg("Block not found"))),
        }
    }

    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        Ok(self
            .client
            .blocks()
            .subscribe_finalized()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .map(|block_res| {
                block_res
                    .map_err(|e| MaybeRetryable::Transient(Error::from(e)))
                    .and_then(|block| {
                        let header = block.header();
                        let parent_hash = header.parent_hash;
                        let state_root = header.state_root;
                        let extrinsics_root = header.extrinsics_root;
                        let number = header.number;
                        let digest = header.digest.clone();

                        // Extract timestamp from extrinsics
                        let timestamp: u64 = 0;

                        let cyferio_header = CyferioHeader::new(
                            number,
                            CyferioHash::from(parent_hash.0),
                            CyferioHash::from(state_root.0),
                            CyferioHash::from(extrinsics_root.0),
                            digest,
                            timestamp,
                        );
                        Ok(cyferio_header)
                    })
            })
            .boxed())
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944")
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;
        let rpc = LegacyRpcMethods::<StatemintConfig>::new(rpc_client.clone());

        let head_hash = rpc
            .chain_get_finalized_head()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let block_detail = rpc
            .chain_get_block(Some(head_hash))
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

                // Extract timestamp from extrinsics
                let mut timestamp: u64 = 0;
                for ext in block.block.extrinsics.iter() {
                    if let Ok(call) = substrate::Call::decode(&mut ext.as_ref()) {
                        if let substrate::Call::Timestamp(substrate::timestamp::Call::set { now }) =
                            call
                        {
                            timestamp = now;
                            break;
                        }
                    }
                }

                let cyferio_header = CyferioHeader::new(
                    number,
                    CyferioHash::from(parent_hash.0),
                    CyferioHash::from(state_root.0),
                    CyferioHash::from(extrinsics_root.0),
                    digest,
                    timestamp,
                );
                Ok(cyferio_header)
            }
            None => Err(MaybeRetryable::Transient(Error::msg("Block not found"))),
        }
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
