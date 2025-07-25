mod module;
mod trading;
mod utils;
use alloy::primitives::U256;
use anyhow::Result;
use std::fs;

use module::{JinCore, config::AppConfig};
use pancakeswap::create_eth_provider;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = load_config("config.yaml")?;

    let (provider, wallet) = create_eth_provider(
        &config.web3_conf.bsc_endpoint,
        config.web3_conf.mnemonic.clone(),
    )
    .await?;
    let mut jin_core = JinCore::new_jin();
    jin_core.set_strategy_configuration(
        config.web3_conf.slippage,
        U256::from(config.web3_conf.grids_num),
        config.web3_conf.grid_upper_limmit,
        config.web3_conf.grid_lower_limmit,
        config.web3_conf.deposit_usdt,
        config.web3_conf.deposit_cess,
        config.web3_conf.price_tolerance_slippage,
        config.web3_conf.trade_record_path,
    )?;
    jin_core.with_web3_trader(provider, wallet).await?;

    jin_core.start_web3_trade_task().await?;

    Ok(())
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
