use std::{str::FromStr, time::Duration};

use crate::module::JinCore;
use alloy::{
    primitives::{Address, U256, utils::format_ether},
    providers::Provider,
    signers::k256::elliptic_curve::generic_array::iter,
};
use anyhow::{Context, Result, anyhow, bail};
use log::{error, info};

use pancakeswap::{
    DexRouter, TradeBotNeed,
    bep_20::{CESS_ADDRESS, USDT_ADDRESS, WBNB_ADDRESS},
    smartswap::{PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapContract},
    utils::*,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub fn slippage_compute_min(price: &U256, slippage: u64) -> U256 {
    price * U256::from(slippage) / U256::from(1000)
}

pub fn slippage_compute_max(price: &U256, slippage: u64) -> U256 {
    price * U256::from(2000 - slippage) / U256::from(1000)
}

pub async fn check_cess_to_usdt_price<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
) -> Result<U256> {
    let path = vec![
        Address::from_str(CESS_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(USDT_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    let amount_out = get_u256_token(18);
    let price_result = pancakeswap_contract
        .check_price_with_multi_hop(path.clone(), fee.clone(), amount_out)
        .await?;

    Ok(price_result.price)
}

pub async fn buy_cess_using_usdt<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
    usdt_use: U256,
    cess_amount: U256,
    slippage: u64,
) -> Result<String> {
    let path = vec![
        Address::from_str(USDT_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(CESS_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    let cess_min_received = slippage_compute_min(&cess_amount, slippage);

    // info!(
    //     "cess should received :{:?}",
    //     format_ether(cess_amount)
    // );
    // info!("cess min received :{:?}", format_ether(cess_min_received));
    let tx_hash = pancakeswap_contract
        .swap_exact_inpute_tokens_for_tokens_with_multi_hop(path, fee, usdt_use, cess_min_received)
        .await?;
    Ok(tx_hash)
}

pub async fn sell_cess_get_usdt<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
    cess_max_willing_pay: U256,
    amount_received: U256,
) -> Result<String> {
    let path = vec![
        Address::from_str(USDT_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(CESS_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    // info!(
    //     "usdt want to received :{:?}",
    //     format_ether(amount_received)
    // );
    // info!("CESS max pay is :{:?}",format_ether(cess_max_willing_pay));
    let tx_hash = pancakeswap_contract
        .swap_exact_outpute_tokens_for_tokens_with_multi_hop(
            path,
            fee,
            amount_received,
            cess_max_willing_pay,
        )
        .await?;
    Ok(tx_hash)
}
