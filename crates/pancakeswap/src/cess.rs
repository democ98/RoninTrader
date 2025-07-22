use crate::{ContractType, TradeBotNeed};
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::Provider,
    sol,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
pub const CESS_ADDRESS: &str = "0x0c78d4605c2972e5f989DE9019De1Fb00c5D3462";
sol! {
    #[sol(rpc)]
    contract CESS {
        function balanceOf(address owner) public view returns (uint256);
        function allowance(address owner, address spender) public view returns (uint256);
        function approve(address guy, uint wad) public returns (bool);
    }
}

#[derive(Debug, Clone)]
pub struct CESSToken<P: Provider> {
    provider: P,
    contract: CESS::CESSInstance<P>,
    wallet: EthereumWallet,
}

#[async_trait]
impl<P: Provider> TradeBotNeed<P> for CESSToken<P> {
    fn new(provider: P, contract: ContractType<P>, wallet: EthereumWallet) -> Result<Self> {
        match contract {
            ContractType::CESS(contract) => Ok(Self {
                provider,
                contract,
                wallet,
            }),
            _ => Err(anyhow!(
                "please new CESS token client with correct contract type"
            )),
        }
    }

    async fn balance_of(&self, address: Address) -> Result<U256> {
        let balance_num_u256 = self
            .contract
            .balanceOf(address)
            .call()
            .await
            .context("get CESS balance failed")?;
        Ok(balance_num_u256)
    }

    async fn allowance(&self, owner: Option<Address>, spender: Address) -> Result<U256> {
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
        Ok(allowance_number)
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
