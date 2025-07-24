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
    smartswap::{
        PANCAKE_SWAP_QUOTER_V2, PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapBundle,
        PancakeswapContract, QUOTER_V2, SMART_ROUTER_V3,
    },
    utils::*,
    bep_20::{TokenType,BEP20TOKEN,USDT_ADDRESS,WBNB_ADDRESS,CESS_ADDRESS}
};
use web3::Web3State;

#[derive(Clone)]
pub struct JinCore<P: Provider + Clone> {
    pub web3_state: Option<Web3State<P>>,
}

impl<P: Provider + Clone> JinCore<P> {
    pub async fn new(provider: P, wallet: EthereumWallet) -> Result<Self> {
        let usdt_contract = TokenType::USDT(Address::from_str(USDT_ADDRESS)?);
        let usdt_token = BEP20TOKEN::new(provider.clone(), usdt_contract, wallet.clone())?;

        let wbnb_contract = TokenType::WBNB(Address::from_str(WBNB_ADDRESS)?);
        let wbnb_token = BEP20TOKEN::new(provider.clone(), wbnb_contract, wallet.clone())?;

        let cess_contract = TokenType::CESS(Address::from_str(CESS_ADDRESS)?);
        let cess_token = BEP20TOKEN::new(provider.clone(), cess_contract, wallet.clone())?;

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
