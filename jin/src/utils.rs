use alloy::primitives::U256;
use anyhow::Result;



pub fn slippage_compute_min(price: &U256, slippage: u64)->U256 {
   price * U256::from(slippage) / U256::from(1000)
}

pub fn slippage_compute_max(price: &U256, slippage: u64)->U256 {
   price * U256::from(2000-slippage) / U256::from(1000)
}