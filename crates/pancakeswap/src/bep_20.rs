use crate::{ContractType, TradeBotNeed};
use alloy::{
    contract::ContractInstance, network::EthereumWallet, primitives::{Address, U256}, providers::Provider, sol
};
use anyhow::{anyhow, bail, Result, Context};
use async_trait::async_trait;

pub const CESS_ADDRESS: &str = "0x0c78d4605c2972e5f989DE9019De1Fb00c5D3462";
pub const USDT_ADDRESS: &str = "0x55d398326f99059fF775485246999027B3197955";
pub const WBNB_ADDRESS: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c";

sol! {
    #[sol(rpc)]
    contract CESS {
        function balanceOf(address owner) public view returns (uint256);
        function allowance(address owner, address spender) public view returns (uint256);
        function approve(address guy, uint wad) public returns (bool);
    }
}

sol! {
    #[sol(rpc)]
    contract WBNB {
        function balanceOf(address owner) public view returns (uint256);
        function allowance(address owner, address spender) public view returns (uint256);
        function approve(address guy, uint wad) public returns (bool);
    }
}

sol! {
    #[sol(rpc)]
    contract USDT {
        function balanceOf(address owner) public view returns (uint256);
        function allowance(address owner, address spender) public view returns (uint256);
        function approve(address guy, uint wad) public returns (bool);
    }
}


#[derive(Clone)]
pub enum TokenType {
    WBNB (Address) ,
    CESS (Address),
    USDT (Address),
}

#[derive(Clone)]
pub struct BEP20TOKEN<P: Provider> {
    provider: P,
    contract: ContractType<P>,
    pub wallet: EthereumWallet,
}

#[async_trait]
impl<P: Provider+Clone> TradeBotNeed<P> for BEP20TOKEN<P> {
    fn new(provider: P, contract: TokenType, wallet: EthereumWallet) -> Result<Self> {
        match contract {
            TokenType::WBNB(address) => {
                 let wbnb_contract = WBNB::WBNBInstance::new(address, provider.clone());
                 let contract = ContractType::WBNB(wbnb_contract);
                 return Ok(Self {provider,contract,wallet})
            },
            TokenType::CESS(address) => {
                let contract = ContractType::WBNB(WBNB::WBNBInstance::new(address, provider.clone()));
                return Ok(Self {provider,contract,wallet})
            },
            TokenType::USDT(address) => {
                let contract = ContractType::WBNB(WBNB::WBNBInstance::new(address, provider.clone()));
                return Ok(Self {provider,contract,wallet})
            },
        };
    }

    async fn balance_of(&self, address: Address) -> Result<U256> {
        let balance_num_u256 = match &self.contract {
            ContractType::WBNB(contract) => {contract.balanceOf(address).call().await.context("get WBNB balance failed")?},
            ContractType::CESS(contract) => {contract.balanceOf(address).call().await.context("get CESS balance failed")?},
            ContractType::USDT(contract) => {contract.balanceOf(address).call().await.context("get USDT balance failed")?},
            _ => bail!("this contract is not allow to use balcance of")
        };
        Ok(balance_num_u256)
    }

    async fn allowance(&self, spender: Address) -> Result<U256> {
        let allowance_number = match &self.contract {
            ContractType::WBNB(contract) => {
                contract
                .allowance(self.wallet.default_signer().address(), spender)
                .call()
                .await
                .context("call WBNB allowance failed")?},
            ContractType::CESS(contract) => {
                contract
                .allowance(self.wallet.default_signer().address(), spender)
                .call()
                .await
                .context("call CESS allowance failed")?},
            ContractType::USDT(contract) => {
                contract
                .allowance(self.wallet.default_signer().address(), spender)
                .call()
                .await
                .context("call USDT allowance failed")?},
            _ => bail!("this contract is not allow to use balcance of")
        };
        Ok(allowance_number)
    }
    async fn approve(&self, spender: Address) -> Result<String> {
        let tx_hash = match &self.contract {
            ContractType::WBNB(contract) => {
                contract.approve(spender, U256::MAX)
                .send()
                .await
                .context("send approve WBNB failed")?
                .tx_hash()
                .to_vec()
            },
            ContractType::CESS(contract) => {
                contract.approve(spender, U256::MAX)
                .send()
                .await
                .context("send approve CESS failed")?
                .tx_hash()
                .to_vec()
            },
            ContractType::USDT(contract) => {
                contract.approve(spender, U256::MAX)
                .send()
                .await
                .context("send approve USDT failed")?
                .tx_hash()
                .to_vec()
            },
            _ => bail!("this contract is not allow to use balcance of")
        };
        Ok(hex::encode(tx_hash))
    }
}