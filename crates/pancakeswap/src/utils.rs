use alloy::primitives::U256;

pub fn f64_to_u256(decimal: f64, decimals: u32) -> U256 {
    let scaled = (decimal * 10f64.powi(decimals as i32)) as u128;
    U256::from(scaled)
}

pub fn get_u256_token(decimals: u32) -> U256 {
    U256::from(10).pow(U256::from(decimals))
}
