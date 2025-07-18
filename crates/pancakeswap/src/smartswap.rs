use crate::{
    smartswap::{IQuoterV2::QuoteExactOutputSingleParams, IV3SwapRouter::ExactInputSingleParams},
    ContractType, DexRouter,
};
use alloy::{
    dyn_abi::abi::{self, token},
    network::{Ethereum, EthereumWallet, Network},
    primitives::{utils::format_ether, Address, Bytes, U160, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use ruint::Uint;
pub const PANCAKE_SWAP_SMART_ROUTER_V3: &str = "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4";
pub const PANCAKE_SWAP_QUOTER_V2: &str = "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997";

sol! {
    // #[sol(rpc)]
    // contract QUOTER_V2 {
    //     function quoteExactOutput(bytes memory path, uint256 amountOut) external returns (
    //         uint256 amountIn,
    //         uint256 gasEstimate,
    //         uint32[] memory initializedTicksCrossedList,
    //         uint160[] memory sqrtPriceX96AfterList
    //     );
    //     function quoteExactOutputSingle(
    //         address tokenIn,
    //         address tokenOut,
    //         uint24 fee,
    //         uint256 amountOut,
    //         uint160 sqrtPriceLimitX96
    //     ) external returns (uint256 amountIn);
    // }
    #[allow(missing_docs)]
    #[sol(rpc)]
    QUOTER_V2,
    "abi/quoterV2.json"
}
sol! {
    // #[sol(rpc)]
    // contract SMART_ROUTER_V3 {
    //     function swapExactTokensForTokens(
    //         uint256 amountIn,
    //         uint256 amountOutMin,
    //         address[] calldata path,
    //         address to
    //     ) external payable returns (uint256 amountOut);
    // }
    #[allow(missing_docs)]
    #[sol(rpc)]
    SMART_ROUTER_V3,
    "abi/smartRouterV3.json"
}

#[derive(Clone)]
pub struct PancakeswapBundle<P> {
    pub quoter: QUOTER_V2::QUOTER_V2Instance<P>,
    pub router: SMART_ROUTER_V3::SMART_ROUTER_V3Instance<P>,
}

#[derive(Debug, Clone)]
pub struct PancakeswapContract<P> {
    provider: P,
    quoter: QUOTER_V2::QUOTER_V2Instance<P>,
    router: SMART_ROUTER_V3::SMART_ROUTER_V3Instance<P>,
    wallet: EthereumWallet,
}

pub struct PriceCheckResult {
    pub price: Uint<256, 4>,
    pub gas_estimate: Uint<256, 4>,
}

#[async_trait]
impl<P: Provider> DexRouter<P> for PancakeswapContract<P> {
    fn new(provider: P, contract: ContractType<P>, wallet: EthereumWallet) -> Result<Self> {
        match contract {
            ContractType::SmartSwap(contract) => Ok(Self {
                provider,
                quoter: contract.quoter,
                router: contract.router,
                wallet,
            }),
            _ => Err(anyhow!(
                "please new SmartSwap contract client with correct contract type"
            )),
        }
    }
    async fn check_price(&self, token0: Address, token1: Address) -> Result<PriceCheckResult> {
        let amount_out = U256::from_str_radix("1000000000000000000", 10)?;
        let param = QuoteExactOutputSingleParams {
            tokenIn: token1,
            tokenOut: token0,
            amount: amount_out,
            fee: Uint::from(100),
            sqrtPriceLimitX96: U160::ZERO,
        };

        let result = self
            .quoter
            .quoteExactOutputSingle(param)
            .call()
            .await
            .context("get price failed")?;
        println!("转换为 Token1:{} Token0", format_ether(result.amountIn));
        println!("need gas: {}", format_ether(result.gasEstimate));
        let result = PriceCheckResult {
            price: result.amountIn,
            gas_estimate: result.gasEstimate,
        };
        Ok(result)
    }
    async fn swap_exact_tokens_for_tokens(
        &self,
        token0: Address,
        token1: Address,
        amount: U256,
        amount_out_min: U256,
    ) -> Result<String> {
        let params = ExactInputSingleParams {
            tokenIn: token0,
            tokenOut: token1,
            fee: Uint::from(100),
            recipient: self.wallet.default_signer().address(),
            amountIn: amount,
            amountOutMinimum: amount_out_min,
            sqrtPriceLimitX96: Uint::ZERO,
        };

        let tx_hash = self
            .router
            .exactInputSingle(params)
            .send()
            .await
            .context("swap token0 to token1 failed")?
            .tx_hash()
            .to_vec();

        println!("result is {}", hex::encode(tx_hash));
        Ok("".to_string())
    }
}
