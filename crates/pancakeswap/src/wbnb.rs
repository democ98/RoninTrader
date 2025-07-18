use crate::{ContractType, TradeBotNeed};
use alloy::{
    network::{Ethereum, EthereumWallet, Network},
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

pub const WBNB_ADDRESS: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c";

sol! {
    #[sol(rpc)]
    contract WBNB {
        function balanceOf(address owner) public view returns (uint256);
        function allowance(address owner, address spender) public view returns (uint256);
        function approve(address guy, uint wad) public returns (bool);
    }
}

#[derive(Debug, Clone)]
pub struct WBNBToken<P: Provider> {
    provider: P,
    contract: WBNB::WBNBInstance<P>,
    wallet: EthereumWallet,
}

#[async_trait]
impl<P: Provider> TradeBotNeed<P> for WBNBToken<P> {
    fn new(provider: P, contract: ContractType<P>, wallet: EthereumWallet) -> Result<Self> {
        match contract {
            ContractType::WBNB(contract) => Ok(Self {
                provider,
                contract,
                wallet,
            }),
            _ => Err(anyhow!(
                "please new WBNB token client with correct contract type"
            )),
        }
    }

    async fn balance_of(&self, address: Address) -> Result<U256> {
        let balance_num_u256 = self
            .contract
            .balanceOf(address)
            .call()
            .await
            .context("get WBNB balance failed")?;
        Ok(balance_num_u256)
    }

    async fn allowance(&self, owner: Option<Address>, spender: Address) -> Result<(bool, U256)> {
        let allowance_number = match owner {
            Some(owner) => self
                .contract
                .allowance(owner, spender)
                .call()
                .await
                .context("call allowance failed")?,
            None => self
                .contract
                .allowance(self.wallet.default_signer().address(), spender)
                .call()
                .await
                .context("call allowance failed")?,
        };
        if allowance_number == U256::MAX {
            return Ok((true, U256::ZERO));
        } else {
            return Ok((false, allowance_number));
        }
    }
    async fn approve(&self, spender: Address) -> Result<String> {
        let tx_hash = self
            .contract
            .approve(spender, U256::MAX)
            .send()
            .await
            .context("send approve failed")?
            .tx_hash()
            .to_vec();

        Ok(hex::encode(tx_hash))
    }
}
