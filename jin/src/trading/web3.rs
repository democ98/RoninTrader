use crate::module::web3::{Order, PriceList, TradeDestination, Web3TradeCore};
use std::{str::FromStr, time::Duration};

use crate::module::JinCore;
use alloy::{
    primitives::{Address, U256, utils::format_ether},
    providers::Provider,
    signers::k256::elliptic_curve::generic_array::iter,
};
use anyhow::{Context, Result, anyhow, bail};
use log::{error, info};

use crate::utils::trade_helper::*;
use pancakeswap::{
    DexRouter, TradeBotNeed,
    bep_20::{CESS_ADDRESS, USDT_ADDRESS, WBNB_ADDRESS},
    smartswap::{PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapContract},
    utils::*,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

impl Web3TradeCore {
    pub fn new(
        cess_latest_price: U256,
        grids_num: U256,
        grid_upper_limmit: U256,
        grid_lower_limmit: U256,
        slippage: u64,
        deposit_usdt: U256,
        deposit_cess: U256,
        price_tolerance_slippage: u64,
        trade_record_path: String,
    ) -> Result<Web3TradeCore> {
        if slippage >= 1000 {
            bail!("The slippage tolerance is too low. Please adjust your settings!")
        }
        if price_tolerance_slippage >= 1000 {
            bail!("The price tolerance slippage is too low. Please adjust your settings!")
        }

        if grid_lower_limmit >= grid_upper_limmit {
            return Err(anyhow!(
                "grid_upper_limmit must be greater than grid_lower_limmit"
            ));
        }
        let mut result = Web3TradeCore {
            price_queue: PriceList::new(),
            price_seg: U256::ZERO,
            slippage,
            deposited_usdt: deposit_usdt,
            deposited_cess: deposit_cess,
            use_usdt_per_seg: U256::ZERO,
            trade_record_path,
        };
        let per_seg = (grid_upper_limmit - grid_lower_limmit) / grids_num;
        result.price_seg = per_seg;
        result.use_usdt_per_seg = deposit_usdt / grids_num;

        //collect price list and check witch price is closest
        let mut price_list = Vec::new();
        let mut delta = U256::ZERO;
        let mut closest_price = U256::ZERO;

        for i in 0..grids_num.try_into().unwrap() {
            let key_price = grid_lower_limmit + per_seg * U256::from(i);
            if key_price >= cess_latest_price {
                let difference = key_price - cess_latest_price;
                if delta == U256::ZERO || difference < delta {
                    delta = difference;
                    closest_price = key_price;
                }
            } else {
                let difference = cess_latest_price - key_price;
                if delta == U256::ZERO || difference < delta {
                    delta = difference;
                    closest_price = key_price;
                }
            }
            price_list.push(key_price);
        }

        for price in price_list {
            let min_acceptable_price = slippage_compute_min(&price, price_tolerance_slippage);
            let max_acceptable_price = slippage_compute_max(&price, price_tolerance_slippage);

            if price > closest_price {
                let order = Order {
                    mount: None,
                    destination: TradeDestination::SELL,
                    min_acceptable_price,
                    max_acceptable_price,
                };
                result.price_queue.push_back(price, Some(order));
            }
            if price < closest_price {
                let order = Order {
                    mount: None,
                    destination: TradeDestination::BUY,
                    min_acceptable_price,
                    max_acceptable_price,
                };
                result.price_queue.push_back(price, Some(order));
            }
            if price == closest_price {
                result.price_queue.push_back(price, None);
                info!(
                    "The current CESS price is close to the price range of {:?}. No orders will be placed at 【{:?}】.",
                    format_ether(closest_price),
                    format_ether(closest_price)
                )
            }
        }

        Ok(result)
    }

    pub async fn trade_bot_runner<P: Provider + Clone>(
        &self,
        pancakeswap_contract: &PancakeswapContract<P>,
    ) -> Result<()> {
        let n = 1;
        //run robot scripts
        while n == 1 {
            let cess_price = check_cess_to_usdt_price(pancakeswap_contract).await?;
            info!("[🔎]CESS new price is :{:?}", format_ether(cess_price));
            for node in &self.price_queue {
                let mut node_data = node.borrow_mut();
                let mut is_trigger = false;
                match &node_data.order_task {
                    Some(order) => {
                        //check if in the price range
                        if cess_price <= order.max_acceptable_price
                            && order.min_acceptable_price <= cess_price
                        {
                            let amount = match order.mount {
                                Some(mount) => {
                                    //Have order task,need to trigger buy or sell.
                                    mount
                                }
                                None => {
                                    //Haven't order task,use use_usdt_per_seg/cess_price to compute how many cess token need be traded.
                                    (self.use_usdt_per_seg / cess_price) * get_u256_token(18)
                                }
                            };

                            match order.destination {
                                TradeDestination::BUY => {
                                    let tx_hash = buy_cess_using_usdt(
                                        &pancakeswap_contract,
                                        self.use_usdt_per_seg,
                                        amount,
                                        self.slippage,
                                    )
                                    .await?;
                                    info!(
                                        "[BUY💸]Price:{:?} triggered.Buy cess token with amount:{:?},Transaction hash is:{:?}",
                                        format_ether(node_data.this_price),
                                        format_ether(amount),
                                        tx_hash
                                    );
                                    match &node_data.next_price {
                                        Some(next_node) => {
                                            let sell_task = Order {
                                                mount: Some(amount),
                                                destination: TradeDestination::SELL,
                                                min_acceptable_price: slippage_compute_min(
                                                    &next_node.borrow().this_price,
                                                    self.slippage,
                                                ),
                                                max_acceptable_price: slippage_compute_max(
                                                    &next_node.borrow().this_price,
                                                    self.slippage,
                                                ),
                                            };
                                            next_node.borrow_mut().order_task = Some(sell_task);
                                        }
                                        None => {
                                            //logical error
                                            error!(
                                                "A buy order occurred at the last price level, which shouldn't happen!!"
                                            );
                                            bail!(
                                                "A buy order occurred at the last price level, which shouldn't happen!!"
                                            )
                                        }
                                    }
                                }
                                TradeDestination::SELL => {
                                    let cess_to_sell = amount;
                                    let cess_max_willing_pay =
                                        slippage_compute_max(&cess_to_sell, self.slippage);
                                    let usdt_amount_received =
                                        ((cess_to_sell) * (cess_price)) / get_u256_token(18);
                                    let tx_hash = sell_cess_get_usdt(
                                        &pancakeswap_contract,
                                        cess_max_willing_pay,
                                        usdt_amount_received,
                                    )
                                    .await?;
                                    info!(
                                        "[SELL💰]Price:{:?} triggered.Sell CESS token with amount:{:?},Transaction hash is:{:?}",
                                        format_ether(node_data.this_price),
                                        format_ether(amount),
                                        tx_hash
                                    );
                                    match &node_data.previous_price {
                                        Some(privious_node) => {
                                            match privious_node.upgrade() {
                                                Some(priv_node) => {
                                                    let buy_task = Order {
                                                        mount: Some(amount),
                                                        destination: TradeDestination::BUY,
                                                        min_acceptable_price: slippage_compute_min(
                                                            &priv_node.borrow().this_price,
                                                            self.slippage,
                                                        ),
                                                        max_acceptable_price: slippage_compute_max(
                                                            &priv_node.borrow().this_price,
                                                            self.slippage,
                                                        ),
                                                    };
                                                    priv_node.borrow_mut().order_task =
                                                        Some(buy_task);
                                                }
                                                None => {
                                                    //code error
                                                    error!(
                                                        "An error occurred while initializing the price chain — the head node's previous_node should be None!"
                                                    );
                                                    bail!(
                                                        "An error occurred while initializing the price chain — the head node's previous_node should be None!"
                                                    )
                                                }
                                            }
                                        }
                                        None => {
                                            //logical error
                                            error!(
                                                "A sell trade occurred at the first price level, which shouldn't happen!!"
                                            );
                                            bail!(
                                                "A sell trade occurred at the first price level, which shouldn't happen!!"
                                            )
                                        }
                                    }
                                }
                            }
                            //set this node order task to None
                            node_data.order_task = None;
                            is_trigger = true;
                            break;
                        }
                    }
                    None => {
                        // info!("⌛No order tasks found, indicating that the price has not fluctuated to any actionable level~~")
                    }
                }
                if node_data.next_price.is_none() && !is_trigger {
                    info!(
                        "⌛No order tasks found, indicating that the price has not fluctuated to any actionable level~~"
                    )
                }
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        Ok(())
    }
}
