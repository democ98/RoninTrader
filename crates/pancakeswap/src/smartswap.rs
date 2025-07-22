use crate::{
    smartswap::{
        IQuoterV2::QuoteExactOutputSingleParams,
        IV3SwapRouter::{ExactInputParams, ExactInputSingleParams, ExactOutputParams},
    },
    utils::*,
    ContractType, DexRouter,
};
use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, U160, U256},
    providers::{Provider, },
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
        let amount_out = get_u256_token(18);
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
        let result = PriceCheckResult {
            price: result.amountIn,
            gas_estimate: result.gasEstimate,
        };
        Ok(result)
    }
    async fn check_price_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_out: U256,
    ) -> Result<PriceCheckResult> {
        let trade_path = build_multi_hop_swap_path(path, fee)?;

        let get_price_reulst = self
            .quoter
            .quoteExactOutput(trade_path.into(), amount_out)
            .call()
            .await
            .context("get price with price failed")?;
        let result = PriceCheckResult {
            price: get_price_reulst.amountIn,
            gas_estimate: get_price_reulst.gasEstimate,
        };

        Ok(result)
    }
    async fn swap_exact_tokens_for_tokens(
        &self,
        token0: Address,
        token1: Address,
        amount_in: U256,
        amount_out_min: U256,
    ) -> Result<String> {
        let params = ExactInputSingleParams {
            tokenIn: token0,
            tokenOut: token1,
            fee: Uint::from(100),
            recipient: self.wallet.default_signer().address(),
            amountIn: amount_in,
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

        Ok(hex::encode(tx_hash))
    }
    async fn swap_exact_inpute_tokens_for_tokens_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_in: U256,
        amount_out_min: U256,
    ) -> Result<String> {
        let trade_path = build_multi_hop_swap_path(path, fee)?;

        let params = ExactInputParams {
            path: trade_path,
            recipient: self.wallet.default_signer().address(),
            amountIn: amount_in,
            amountOutMinimum: amount_out_min,
        };

        let tx_hash = self
            .router
            .exactInput(params)
            .send()
            .await
            .context("multi hop swap failed")?
            .tx_hash()
            .to_vec();

        Ok(hex::encode(tx_hash))
    }

    async fn swap_exact_outpute_tokens_for_tokens_with_multi_hop(
        &self,
        path: Vec<Address>,
        fee: Vec<u32>,
        amount_out: U256,
        amount_in_max: U256,
    ) -> Result<String> {
        let trade_path = build_multi_hop_swap_path(path, fee)?;
        let params = ExactOutputParams {
            path: trade_path,
            recipient: self.wallet.default_signer().address(),
            amountOut: amount_out,
            amountInMaximum: amount_in_max,
        };

        let tx_hash = self
            .router
            .exactOutput(params)
            .send()
            .await
            .context("multi hop swap failed")?
            .tx_hash()
            .to_vec();

        Ok(hex::encode(tx_hash))
    }
}

fn build_multi_hop_swap_path(path: Vec<Address>, fee: Vec<u32>) -> Result<Bytes> {
    if path.len() != fee.len() + 1 {
        return Err(anyhow!("path and fee length not match"));
    }
    let mut fee = fee.clone();
    let mut trade_path: Vec<u8> = Vec::new();
    let mut counter = 0;
    for address in path.clone() {
        counter += 1;
        trade_path.extend_from_slice(address.as_slice());

        if !(counter == path.len()) {
            let fee_rate: Uint<24, 1> = Uint::from(fee.remove(0));
            let fee_rate_byte = fee_rate.to_be_bytes_vec();
            trade_path.extend_from_slice(&fee_rate_byte);
        }
    }
    Ok(trade_path.into())
}
