pub mod config;
pub mod web3;
use std::str::FromStr;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::Provider,
};
use anyhow::{Result, anyhow};
use pancakeswap::{
    ContractType, DexRouter, TradeBotNeed,
    cess::{CESS, CESS_ADDRESS, CESSToken},
    smartswap::{
        PANCAKE_SWAP_QUOTER_V2, PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapBundle,
        PancakeswapContract, QUOTER_V2, SMART_ROUTER_V3,
    },
    usdt::{USDT, USDT_ADDRESS, USDTToken},
    utils::*,
    wbnb::{WBNB, WBNB_ADDRESS, WBNBToken},
};
use web3::Web3State;

#[derive(Debug, Clone)]
pub struct JinCore<P: Provider + Clone> {
    pub web3_state: Option<Web3State<P>>,
}

impl<P: Provider + Clone> JinCore<P> {
    pub async fn new(provider: P, wallet: EthereumWallet) -> Result<Self> {
        let usdt_contract =
            USDT::USDTInstance::new(Address::from_str(USDT_ADDRESS)?, provider.clone());
        let usdt_token = USDTToken::new(
            provider.clone(),
            ContractType::USDT(usdt_contract),
            wallet.clone(),
        )?;

        let wbnb_contract =
            WBNB::WBNBInstance::new(Address::from_str(WBNB_ADDRESS)?, provider.clone());
        let wbnb_token = WBNBToken::new(
            provider.clone(),
            ContractType::WBNB(wbnb_contract),
            wallet.clone(),
        )?;

        let cess_contract =
            CESS::CESSInstance::new(Address::from_str(CESS_ADDRESS)?, provider.clone());
        let cess_token = CESSToken::new(
            provider.clone(),
            ContractType::CESS(cess_contract),
            wallet.clone(),
        )?;

        let pancakeswap_bundle = PancakeswapBundle {
            quoter: QUOTER_V2::QUOTER_V2Instance::new(
                Address::from_str(PANCAKE_SWAP_QUOTER_V2)?,
                provider.clone(),
            ),
            router: SMART_ROUTER_V3::SMART_ROUTER_V3Instance::new(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)?,
                provider.clone(),
            ),
        };
        let pancakeswap_contract = PancakeswapContract::new(
            provider.clone(),
            ContractType::SmartSwap(pancakeswap_bundle),
            wallet.clone(),
        )?;

        let result = JinCore {
            web3_state: Some(Web3State {
                wbnb_token,
                cess_token,
                usdt_token,
                pancakeswap_contract,
                slippage: 990,
                grids_num: U256::ZERO,
                grid_upper_limmit: U256::ZERO,
                grid_lower_limmit: U256::ZERO,
                deposit_usdt: U256::ZERO,
                deposit_cess: U256::ZERO,
                price_tolerance_slippage: 995,
            }),
        };

        Ok(result)
    }

    pub fn set_trade_params(
        &mut self,
        slippage: u64,
        grids_num: U256,
        grid_upper_limmit: f64,
        grid_lower_limmit: f64,
        deposit_usdt: f64,
        deposit_cess: f64,
        price_tolerance_slippage: u64,
    ) -> Result<()> {
        let state = self.web3_state.as_mut().unwrap();
        state.slippage = slippage;
        state.grids_num = grids_num;
        state.grid_upper_limmit = f64_to_u256(grid_upper_limmit, 18);
        state.grid_lower_limmit = f64_to_u256(grid_lower_limmit, 18);
        state.deposit_usdt = f64_to_u256(deposit_usdt, 18);
        state.deposit_cess = f64_to_u256(deposit_cess, 18);
        state.price_tolerance_slippage=price_tolerance_slippage;
        Ok(())
    }
}
