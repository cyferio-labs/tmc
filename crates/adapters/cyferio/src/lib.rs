#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use sp_std::prelude::*;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

pub mod spec;
pub mod validity_condition;

#[cfg(feature = "native")]
pub mod service;
pub mod verifier;

#[cfg(feature = "native")]
pub mod fee;

use substrate_api_client::{
    rpc::WsRpcClient,
    Api,
    compose_extrinsic,
};
use sp_core::sr25519::Pair;

pub async fn submit_task<T: MaxTaskSize>(
    api: &Api<Pair, WsRpcClient>,
    origin: OriginFor<T>,
    task: BoundedVec<u8, T>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 确保签名
    ensure_signed(origin)?;

    // 构造 extrinsic
    let ext = compose_extrinsic!(
        api.clone(),
        "TaskModule",
        "submit_task",
        task.to_vec()
    );

    // 发送 extrinsic
    let tx_hash = api.send_extrinsic(ext.hex_encode(), XtStatus::InBlock)?;

    // 等待交易被包含在区块中
    api.wait_until_finalized(tx_hash).await?;

    Ok(())
}
