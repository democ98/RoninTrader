mod module;
mod tactics;
mod utils;
use alloy::{
    primitives::U256,
};
use anyhow::Result;
use std::{fs};

use module::{JinCore, config::AppConfig};
use pancakeswap::{
    create_eth_provider,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
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
        config.web3_conf.grid_upper_limmit,
        config.web3_conf.grid_lower_limmit,
        config.web3_conf.deposit_usdt,
        config.web3_conf.deposit_cess,
        config.web3_conf.price_tolerance_slippage,
    )?;

    tactics::trader_runner(jin_core).await?;
    Ok(())
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
