pub mod cess;
pub mod utils;
pub(crate) mod helper;
pub mod smartswap;
pub mod usdt;
pub mod wbnb;
use alloy::{
    dyn_abi::abi::token,
    hex,
    network::{Ethereum, EthereumWallet, Network, NetworkWallet},
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use cess::CESS;
use ruint::aliases::U256;
use smartswap::PriceCheckResult;
use std::{marker::PhantomData, str::FromStr};
use usdt::USDT;
use wbnb::WBNB;
pub struct ContractAccessor<N: Network> {
    _networ: PhantomData<N>,
}

#[derive(Clone)]
pub enum ContractType<P: Provider> {
    WBNB(WBNB::WBNBInstance<P>),
    CESS(CESS::CESSInstance<P>),
    USDT(USDT::USDTInstance<P>),
    SmartSwap(smartswap::PancakeswapBundle<P>),
}

pub async fn create_eth_provider(
    rpc_url: &str,
    mnemonic: String,
) -> Result<(impl Provider<Ethereum> + Clone, EthereumWallet)> {
    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic.clone())
        .index(0)
        .context("local signer creation failed")?
        .password("")
        .build()
        .context("build wallet from mnemonic failed")?;
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .connect(rpc_url)
        .await
        .context("connect to provider failed")?;

    Ok((provider, wallet))
}

#[async_trait]
pub trait TradeBotNeed<P: Provider> {
    fn new(provider: P, contract: ContractType<P>, wallet: EthereumWallet) -> Result<Self>
    where
        Self: Sized;
    async fn balance_of(&self, address: Address) -> Result<U256>;
    async fn allowance(&self, owner: Option<Address>, spender: Address) -> Result<U256>;
    async fn approve(&self, spender: Address) -> Result<String>;
}

#[async_trait]
pub trait DexRouter<P: Provider> {
    fn new(provider: P, contract: ContractType<P>, wallet: EthereumWallet) -> Result<Self>
    where
        Self: Sized;

    //check one token0 can buy how many token1 in single pool
    async fn check_price(&self, token0: Address, token1: Address) -> Result<PriceCheckResult>;
    //check how many(amount_out) token0 value how many token1 in pool multi hop
    async fn check_price_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_out: U256,
    ) -> Result<PriceCheckResult>;
    async fn swap_exact_tokens_for_tokens(
        &self,
        token0: Address,
        token1: Address,
        amount_in: U256,
        amount_out_min: U256,
    ) -> Result<String>;

    async fn swap_exact_inpute_tokens_for_tokens_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_in: U256,
        amount_out_min: U256,
    ) -> Result<String>;
    async fn swap_exact_outpute_tokens_for_tokens_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_out: U256,
        amount_in_max: U256,
    ) -> Result<String>;
}
