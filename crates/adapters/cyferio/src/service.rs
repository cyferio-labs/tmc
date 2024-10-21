use crate::fee::CyferioFee;
use crate::spec::CyferioDaLayerSpec;
use crate::spec::block::CyferioBlock;
use crate::spec::header::CyferioHeader;
use crate::spec::hash::CyferioHash;
use crate::verifier::CyferioDaVerifier;
use anyhow::Error;
use async_trait::async_trait;
use futures::stream::BoxStream;
use sov_rollup_interface::da::{DaBlobHash, DaSpec, RelevantBlobs, RelevantProofs};
use sov_rollup_interface::services::da::{DaService, MaybeRetryable};
use substrate_api_client::{Api, ApiClient};

pub struct CyferioConfig {
    pub node_url: String,
}

pub struct DaProvider {
    client: ApiClient,
}

impl DaProvider {
    pub async fn new(config: CyferioConfig) -> Result<Self, Error> {
        let client = ApiClient::new(&config.node_url).await?;
        Ok(Self { client })
    }

    async fn get_latest_block(&self) -> Result<CyferioBlock, Error> {
        // Implement the logic to fetch the latest block from the Substrate chain
        unimplemented!()
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
        // Implement the logic to fetch a block at a specific height
        unimplemented!()
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        // Implement the logic to fetch the last finalized block header
        unimplemented!()
    }

    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        // Implement the logic to subscribe to finalized headers
        unimplemented!()
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        // Implement the logic to fetch the head block header
        unimplemented!()
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
        // Implement the logic to send a transaction to the Substrate chain
        unimplemented!()
    }

    async fn send_aggregated_zk_proof(
        &self,
        _aggregated_proof_data: &[u8],
        _fee: Self::Fee,
    ) -> Result<DaBlobHash<Self::Spec>, Self::Error> {
        // Implement the logic to send an aggregated ZK proof to the Substrate chain
        unimplemented!()
    }

    async fn get_aggregated_proofs_at(&self, _height: u64) -> Result<Vec<Vec<u8>>, Self::Error> {
        // Implement the logic to fetch aggregated proofs at a specific height
        Ok(vec![vec![0u8]])
    }

    async fn estimate_fee(&self, _blob_size: usize) -> Result<Self::Fee, Self::Error> {
        Ok(CyferioFee::zero())
    }
}