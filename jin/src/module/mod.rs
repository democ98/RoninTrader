pub mod config;
pub mod web3;
use std::str::FromStr;

use crate::{module::web3::Web3TradeCore, utils::trade_helper::check_cess_to_usdt_price};
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256, utils::format_ether},
    providers::Provider,
};
use anyhow::{Context, Result, anyhow, bail};
use log::info;
use pancakeswap::{
    ContractType, DexRouter, TradeBotNeed,
    bep_20::{BEP20TOKEN, CESS_ADDRESS, TokenType, USDT_ADDRESS, WBNB_ADDRESS},
    smartswap::{
        PANCAKE_SWAP_QUOTER_V2, PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapBundle,
        PancakeswapContract, QUOTER_V2, SMART_ROUTER_V3,
    },
    utils::*,
};
use web3::Web3State;

#[derive(Clone)]
pub struct JinCore<P: Provider + Clone> {
    pub web3_state: Option<Web3State<P>>,

    //strategy configuration
    pub slippage: u64,
    pub grids_num: U256,
    pub grid_upper_limmit: U256,
    pub grid_lower_limmit: U256,
    pub deposit_usdt: U256,
    pub deposit_cess: U256,
    pub price_tolerance_slippage: u64,
    pub trade_record_path: String,
}

impl<P: Provider + Clone> JinCore<P> {
    pub fn new_jin() -> Self {
        Self {
            web3_state: None,
            slippage: 990,
            grids_num: U256::ZERO,
            grid_upper_limmit: U256::ZERO,
            grid_lower_limmit: U256::ZERO,
            deposit_usdt: U256::ZERO,
            deposit_cess: U256::ZERO,
            price_tolerance_slippage: 995,
            trade_record_path: "./trade_records.txt".to_string(),
        }
    }

    pub fn set_strategy_configuration(
        &mut self,
        slippage: u64,
        grids_num: U256,
        grid_upper_limmit: f64,
        grid_lower_limmit: f64,
        deposit_usdt: f64,
        deposit_cess: f64,
        price_tolerance_slippage: u64,
        trade_record_path: String,
    ) -> Result<()> {
        self.slippage = slippage;
        self.grids_num = grids_num;
        self.grid_upper_limmit = f64_to_u256(grid_upper_limmit, 18);
        self.grid_lower_limmit = f64_to_u256(grid_lower_limmit, 18);
        self.deposit_usdt = f64_to_u256(deposit_usdt, 18);
        self.deposit_cess = f64_to_u256(deposit_cess, 18);
        self.price_tolerance_slippage = price_tolerance_slippage;
        self.trade_record_path = trade_record_path;
        Ok(())
    }

    pub async fn with_web3_trader(&mut self, provider: P, wallet: EthereumWallet) -> Result<()> {
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
        // let cess_latest_price = self.wallet_asset_checker().await?;

        // let web3_trade_core = Web3TradeCore::new(
        //     cess_latest_price,
        //     self.grids_num,
        //     self.grid_upper_limmit,
        //     self.grid_lower_limmit,
        //     self.slippage,
        //     self.deposit_usdt,
        //     self.deposit_cess,
        //     self.price_tolerance_slippage,
        //     self.trade_record_path.clone(),
        // )
        // .context("new web3 trade cores failed")?;

        let web3_state = Web3State {
            wbnb_token,
            cess_token,
            usdt_token,
            pancakeswap_contract,
            web3_trade_core: Web3TradeCore::default(),
        };
        self.web3_state = Some(web3_state);

        let cess_latest_price = self.wallet_asset_checker().await?;

        let web3_trade_core = Web3TradeCore::new(
            cess_latest_price,
            self.grids_num,
            self.grid_upper_limmit,
            self.grid_lower_limmit,
            self.slippage,
            self.deposit_usdt,
            self.deposit_cess,
            self.price_tolerance_slippage,
            self.trade_record_path.clone(),
        )
        .context("new web3 trade cores failed")?;

        self.web3_state.as_mut().unwrap().web3_trade_core = web3_trade_core;

        Ok(())
    }

    pub async fn wallet_asset_checker(&self) -> Result<U256> {
        let core = self
            .clone()
            .web3_state
            .ok_or(anyhow!("web3_state is None"))?;

        //approve usdt && check usdt balances
        let usdt_token = core.usdt_token.clone();
        let my_wallet_address = usdt_token.wallet.default_signer().address();
        let usdt_num = usdt_token.balance_of(my_wallet_address).await?;
        info!(
            "The amount of USDT in your wallet is :{}",
            format_ether(usdt_num)
        );
        if usdt_num < self.deposit_usdt.clone() {
            bail!("Your wallet's USDT is not enough. Please put more USDT into your wallet!")
        };

        let usdt_allowance_num = usdt_token
            .allowance(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                    .context("Invalid address when get pancake swap router allowance in USDT")?,
            )
            .await?;
        if usdt_allowance_num == U256::MAX {
            info!(
                "Your wallet's USDT has been approved for pancakeswap smart router contract use."
            );
        } else {
            info!(
                "Your wallet's USDT is not approved for pancakeswap smart router contract use or approval amount is not enough. Current approval amount is:{}. Start to approve USDT.",
                format_ether(usdt_allowance_num)
            );
            usdt_token
                .approve(
                    Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                        .context("Invalid address when approve USDT")?,
                )
                .await?;
            info!("USDT has been approved for pancakeswap smart router contract successfully!")
        }

        //approve cess && check cess balances
        let cess_token = core.cess_token.clone();
        let cess_num = cess_token.balance_of(my_wallet_address).await?;
        info!(
            "The amount of CESS in your wallet is :{}",
            format_ether(cess_num)
        );
        if cess_num < self.deposit_cess.clone() {
            bail!("Your wallet's CESS is not enough. Please put more CESS into your wallet!")
        }
        let cess_allowance_num = cess_token
            .allowance(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                    .context("Invalid address when get pancake swap router allowance in CESS")?,
            )
            .await?;
        if cess_allowance_num == U256::MAX {
            info!(
                "Your wallet's CESS has been approved for pancakeswap smart router contract use."
            );
        } else {
            info!(
                "Your wallet's CESS is not approved for pancakeswap smart router contract use or approval amount is not enough. Current approval amount is:{}. Start to approve CESS.",
                format_ether(cess_allowance_num)
            );
            cess_token
                .approve(
                    Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                        .context("Invalid address when approve CESS")?,
                )
                .await?;
            info!("CESS has been approved for pancakeswap smart router contract successfully!")
        }

        //get trade params
        let pancakeswap_contract = core.pancakeswap_contract.clone();

        check_cess_to_usdt_price(&pancakeswap_contract).await
    }

    pub async fn start_web3_trade_task(&self) -> Result<()> {
        if let Some(state) = &self.web3_state {
            info!("🤑🤑🤑Start to run robot scripts🤑🤑🤑");
            state
                .web3_trade_core
                .trade_bot_runner(&state.pancakeswap_contract)
                .await?;
        } else {
            bail!("web3_state without initialization, web3 trader can't start!")
        };

        Ok(())
    }
}
