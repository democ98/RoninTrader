use crate::module::JinCore;
use alloy::{primitives::U256, providers::Provider};
use anyhow::{Result, anyhow};
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct TradeParams {
    grid_segments_map: DashMap<U256, bool>,
}

pub fn new_trade_params(
    grids_num: U256,
    grid_upper_limmit: U256,
    grid_lower_limmit: U256,
) -> Result<TradeParams> {
    let grid_segments_map: DashMap<U256, bool> = DashMap::new();
    if grid_upper_limmit < grid_lower_limmit {
        return Err(anyhow!(
            "grid_upper_limmit must be greater than grid_lower_limmit"
        ));
    }
    let seg = grid_upper_limmit - grid_lower_limmit;
    let per_seg = seg / grids_num;
    let num: u128 = grids_num.try_into().unwrap();
    for i in 0..num {
        let key = grid_lower_limmit + per_seg * U256::from(i);
        grid_segments_map.insert(key, false);
    }

    Ok(TradeParams { grid_segments_map })
}

pub async fn trader_runner<P>(core: JinCore<P>) -> Result<()>
where
    P: Provider + Clone,
{
    let core = core
        .web3_state
        .ok_or(anyhow!("web3_state is None"))?
        .clone();
    let trade_params = new_trade_params(
        core.grids_num,
        core.grid_upper_limmit,
        core.grid_lower_limmit,
    )?;
    Ok(())
}
