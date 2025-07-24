use pancakeswap::{
    smartswap::PancakeswapContract,
    bep_20::BEP20TOKEN,
};

use alloy::{
    primitives::U256,
    providers::Provider,
};

#[derive(Clone)]
pub struct Web3State<P: Provider> {
    pub wbnb_token: BEP20TOKEN<P>,
    pub cess_token: BEP20TOKEN<P>,
    pub usdt_token: BEP20TOKEN<P>,
    pub pancakeswap_contract: PancakeswapContract<P>,

    pub slippage: u64,
    pub grids_num: U256,
    pub grid_upper_limmit: U256,
    pub grid_lower_limmit: U256,
    pub deposit_usdt: U256,
    pub deposit_cess: U256,
    pub price_tolerance_slippage:u64,
}
