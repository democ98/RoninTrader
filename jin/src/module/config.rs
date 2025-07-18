use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub web3_conf: Web3,
}

#[derive(Debug, Deserialize)]
pub struct Web3 {
    pub mnemonic: String,
    pub bsc_endpoint: String,
    pub slippage: u64,
    pub grids_num: u128,
    pub grid_upper_limmit: u128,
    pub grid_lower_limmit: u128,
}
