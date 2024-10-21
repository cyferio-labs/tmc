use anyhow::{anyhow, Result};
use codec::Compact;
use sov_cyferio_da::service::{CyferioConfig, DaProvider};
use sov_cyferio_da::spec::address::CyferioAddress;
use sov_rollup_interface::services::da::DaService;
use sp_keyring::AccountKeyring;
use sp_runtime::{generic::Era, MultiAddress};
use sp_std::prelude::*;
use substrate_api_client::{Api, XtStatus};
use frame_support::sp_runtime::traits::Vec;

async fn send_transaction() -> Result<()> {
    // 1) Get Cyferio client
    let node_client_url = "ws://localhost:9944"; // Replace with your Substrate node WebSocket URL
    let cyferio_config = CyferioConfig {
        node_url: node_client_url.to_string(),
    };
    let da_provider = DaProvider::new(cyferio_config).await?;

    println!("Successfully connected to Cyferio network");

    // 2) Create a test address
    let sender = AccountKeyring::Alice.pair();
    let sender_address = CyferioAddress::from(sender.public());
    println!("Sender: {:?}", sender_address);

    // 3) Prepare a test blob
    let blob: Vec<u8> = vec![1, 2, 3, 4, 5];

    // 4) Send transaction
    println!("Sending transaction...");
    let api = Api::<RococoRuntimeConfig, _>::new(node_client_url).await?;
    let extrinsic_signer = ExtrinsicSigner::<RococoRuntimeConfig>::new(sender);
    api.set_signer(extrinsic_signer.clone());

    let last_finalized_header_hash = api.get_finalized_head().await?.unwrap();
    let header = api
        .get_header(Some(last_finalized_header_hash))
        .await?
        .unwrap();
    let period = 5;

    let additional_params = GenericAdditionalParams::new()
        .era(
            Era::mortal(period, header.number.into()),
            last_finalized_header_hash,
        )
        .tip(0);

    let call = compose_call!(api.metadata(), "Tasks", "submit_task", Compact(blob))?;

    let xt = api.compose_extrinsic_offline(call, api.get_nonce().await?);

    match api
        .submit_and_watch_extrinsic_until(xt, XtStatus::InBlock)
        .await
    {
        Ok(tx_result) => {
            println!(
                "Transaction sent successfully. Block hash: {:?}",
                tx_result.block_hash
            );
            Ok(())
        }
        Err(e) => {
            println!("Error sending transaction: {:?}", e);
            Err(anyhow!("Transaction sending failed: {:?}", e))
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
