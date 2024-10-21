use crate::fee::CyferioFee;
use crate::spec::block::CyferioBlock;
use crate::spec::hash::CyferioHash;
use crate::spec::header::CyferioHeader;
use crate::spec::CyferioDaLayerSpec;
use crate::verifier::CyferioDaVerifier;
use anyhow::Error;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use sov_rollup_interface::da::{DaBlobHash, DaSpec, RelevantBlobs, RelevantProofs};
use sov_rollup_interface::services::da::{DaService, MaybeRetryable};
use subxt::{OnlineClient, PolkadotConfig};

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CyferioConfig {
    pub node_url: String,
}

#[derive(Clone)]
pub struct DaProvider {
    client: OnlineClient<PolkadotConfig>,
}

impl DaProvider {
    pub async fn from_config(config: CyferioConfig) -> Result<Self, Error> {
        let client = OnlineClient::<PolkadotConfig>::from_url(&config.node_url)
            .await
            .map_err(Error::from)?;
        Ok(Self { client })
    }

    pub async fn new(config: CyferioConfig) -> Result<Self, Error> {
        Self::from_config(config).await
    }

    async fn get_latest_block(&self) -> Result<CyferioBlock, Error> {
        let block_hash = self.client.rpc().finalized_head().await?;
        let _block = self
            .client
            .rpc()
            .block(Some(block_hash))
            .await?
            .ok_or_else(|| Error::msg("Latest block not found"))?;
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
        let block_hash = self
            .client
            .rpc()
            .block_hash(Some(height.into()))
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?
            .ok_or_else(|| MaybeRetryable::Transient(anyhow::Error::msg("Block hash not found")))?;

        let _block = self
            .client
            .rpc()
            .block(Some(block_hash))
            .await
            .map_err(|e| MaybeRetryable::Transient(anyhow::Error::from(e)))?
            .ok_or_else(|| MaybeRetryable::Transient(anyhow::Error::msg("Block not found")))?;

        Ok(CyferioBlock::default()) // Temporary placeholder
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let finalized_hash = self
            .client
            .rpc()
            .finalized_head()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let _header = self
            .client
            .rpc()
            .header(Some(finalized_hash))
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .ok_or_else(|| MaybeRetryable::Transient(Error::msg("Header not found")))?;

        Ok(CyferioHeader::default()) // Temporary placeholder
    }

    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        let stream = self
            .client
            .rpc()
            .subscribe_finalized_block_headers()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .map(|result| {
                result
                    .map_err(|e| MaybeRetryable::Transient(Error::from(e)))
                    .map(|_| CyferioHeader::default()) // Temporary placeholder
            });
        Ok(Box::pin(stream))
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        let finalized_hash = self
            .client
            .rpc()
            .finalized_head()
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?;

        let _header = self
            .client
            .rpc()
            .header(Some(finalized_hash))
            .await
            .map_err(|e| MaybeRetryable::Transient(Error::from(e)))?
            .ok_or_else(|| MaybeRetryable::Transient(Error::msg("Header not found")))?;

        Ok(CyferioHeader::default()) // Temporary placeholder
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
        _blob: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        // Implementation remains the same
        Ok(CyferioHash::from([0u8; 32]))
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
