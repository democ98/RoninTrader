use pancakeswap::{
    ContractType, TradeBotNeed,
    cess::{CESS, CESS_ADDRESS, CESSToken},
    create_eth_provider,
    smartswap::PancakeswapContract,
    wbnb::{WBNB, WBNB_ADDRESS, WBNBToken},
};

use alloy::{
    network::{Ethereum, EthereumWallet, Network},
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
};

#[derive(Debug, Clone)]
pub struct Web3State<P: Provider> {
    pub wbnb_token: WBNBToken<P>,
    pub cess_token: CESSToken<P>,
    pub pancakeswap_contract: PancakeswapContract<P>,

    pub slippage: u64,
    pub grids_num: U256,
    pub grid_upper_limmit: U256,
    pub grid_lower_limmit: U256,
}
