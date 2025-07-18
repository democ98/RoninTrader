mod module;
mod tactics;
use alloy::{
    contract::ContractInstance,
    primitives::{Address, U256, address, utils::format_ether},
};
use anyhow::{Context, Result};
use std::{fs, str::FromStr};

use module::{JinCore, config::AppConfig};
use pancakeswap::{
    ContractType, DexRouter, TradeBotNeed,
    cess::{CESS, CESS_ADDRESS, CESSToken},
    create_eth_provider,
    smartswap::PANCAKE_SWAP_SMART_ROUTER_V3,
    wbnb::{WBNB, WBNB_ADDRESS, WBNBToken},
};

const MY_WALLET_ADDRESS: &str = "";

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config("config.yaml")?;

    let (provider, wallet) = create_eth_provider(
        &config.web3_conf.bsc_endpoint,
        config.web3_conf.mnemonic.clone(),
    )
    .await?;
    let mut jin_core = JinCore::new(provider.clone(), wallet.clone()).await?;
    jin_core.set_trade_params(
        config.web3_conf.slippage,
        U256::from(config.web3_conf.grids_num),
        U256::from(config.web3_conf.grid_upper_limmit),
        U256::from(config.web3_conf.grid_lower_limmit),
    )?;

    let wbnb_token = jin_core.web3_state.clone().unwrap().wbnb_token;
    let wbnb_num = wbnb_token
        .balance_of(
            Address::from_str(MY_WALLET_ADDRESS)
                .context("Invalid address when get my wbnb balance")?,
        )
        .await?;
    println!("wbnb num is :{}", format_ether(wbnb_num));

    let (unlimited, allowance_num) = wbnb_token
        .allowance(
            None,
            Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                .context("Invalid address when get pancake swap router allowance in wbnb")?,
        )
        .await?;
    println!("is unlimited ? {}", unlimited);
    println!("allowance num is :{}", format_ether(allowance_num));

    if !unlimited {
        wbnb_token
            .approve(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                    .context("Invalid address when approve wbnb")?,
            )
            .await?;
    }

    // let cess_token = jin_core.web3_state.unwrap().cess_token;
    // let wbnb_num = cess_token
    //     .balance_of(
    //         Address::from_str(MY_WALLET_ADDRESS)
    //             .context("Invalid address when get my wbnb balance")?,
    //     )
    //     .await?;
    // println!("wbnb num is :{}", wbnb_num);

    // let (unlimited, allowance_num) = cess_token
    //     .allowance(
    //         None,
    //         Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
    //             .context("Invalid address when get pancake swap router allowance in wbnb")?,
    //     )
    //     .await?;
    // println!("is unlimited ? {}", unlimited);
    // println!("allowance num is :{}", allowance_num);

    // if !unlimited {
    //     let tx_hash = cess_token
    //         .approve(
    //             Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
    //                 .context("Invalid address when approve wbnb")?,
    //         )
    //         .await?;
    //     println!("approve tx hash is :{}", tx_hash);
    // }

    let pancakeswap_contract = jin_core
        .web3_state
        .clone()
        .unwrap()
        .pancakeswap_contract
        .clone();
    let price_result = pancakeswap_contract
        .check_price(
            Address::from_str(WBNB_ADDRESS)?,
            Address::from_str(CESS_ADDRESS)?,
        )
        .await?;
    let price = price_result.price;
    let gas_estimate = price_result.gas_estimate;
    println!("one WBNB can buy {} CESS", format_ether(price));
    println!("gas estimate is :{}", format_ether(gas_estimate));

    let need_swap_wbnb = wbnb_num.div_ceil(jin_core.web3_state.clone().unwrap().grids_num);
    println!("need swap {} WBNB", format_ether(need_swap_wbnb));

    let cess_should_received =
        need_swap_wbnb * price / U256::from_str_radix("1000000000000000000", 10)?;

    let cess_min_received = cess_should_received
        * U256::from(jin_core.web3_state.clone().unwrap().slippage)
        / U256::from(1000);

    println!("cess number is :{}", format_ether(cess_should_received));
    // pancakeswap_contract
    //     .swap_exact_tokens_for_tokens(
    //         Address::from_str(WBNB_ADDRESS)?,
    //         Address::from_str(CESS_ADDRESS)?,
    //         need_swap_wbnb,
    //         cess_min_received,
    //     )
    //     .await?;

    Ok(())
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
