use std::{str::FromStr, time::Duration};

use crate::module::JinCore;
use alloy::{
    primitives::{Address, U256, utils::format_ether},
    providers::Provider,
};
use anyhow::{Context, Result, anyhow, bail};
use log::{error, info};

use pancakeswap::{
     DexRouter, TradeBotNeed,
    cess::{ CESS_ADDRESS},
    smartswap::{PANCAKE_SWAP_SMART_ROUTER_V3, PancakeswapContract},
    usdt::{USDT_ADDRESS},
    utils::*,
    wbnb::{WBNB_ADDRESS},
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::utils::*;

#[derive(Debug, Clone)]
enum TradeDestination {
    SELL,
    BUY,
}

struct Order {
    mount: Option<U256>, //If it's just an initial order placement, this value will be None. If the trade is triggered by a neighboring node, this value will be Some.
    destination: TradeDestination,
    min_acceptable_price: U256,
    max_acceptable_price: U256,
}

struct PriceNode {
    this_price: U256,
    order_task: Option<Order>,
    previous_price: Option<Weak<RefCell<PriceNode>>>,
    next_price: Option<Rc<RefCell<PriceNode>>>,
}
impl PriceNode {
    fn new(price: U256, order: Option<Order>) -> Self {
        Self {
            this_price: price,
            order_task: order,
            previous_price: None,
            next_price: None,
        }
    }
}

struct PriceListIter {
    current: Option<Rc<RefCell<PriceNode>>>,
}
impl Iterator for PriceListIter {
    type Item = Rc<RefCell<PriceNode>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.take().map(|node| {
            let next = node.borrow().next_price.clone();
            self.current = next;
            node
        })
    }
}


struct PriceList {
    head: Option<Rc<RefCell<PriceNode>>>,
    tail: Option<Rc<RefCell<PriceNode>>>,
}

impl PriceList {
    fn new() -> Self {
        PriceList {
            head: None,
            tail: None,
        }
    }
    fn iter(&self) -> PriceListIter {
        PriceListIter {
            current: self.head.clone(),
        }
    }
    fn push_back(&mut self, price: U256, order: Option<Order>) {
        let new_node = Rc::new(RefCell::new(PriceNode::new(price, order)));
        match self.tail.take() {
            Some(old_tail) => {
                new_node.borrow_mut().previous_price = Some(Rc::downgrade(&old_tail));
                old_tail.borrow_mut().next_price = Some(Rc::clone(&new_node));
                self.tail = Some(new_node);
            }
            None => {
                self.head = Some(Rc::clone(&new_node));
                self.tail = Some(new_node);
            }
        }
    }

    #[allow(dead_code)]
    fn push_front(&mut self, price: U256, order: Option<Order>) {
        let new_node = Rc::new(RefCell::new(PriceNode::new(price, order)));
        match self.head.take() {
            Some(old_head) => {
                new_node.borrow_mut().next_price = Some(Rc::clone(&old_head));
                old_head.borrow_mut().previous_price = Some(Rc::downgrade(&new_node));
                self.head = Some(new_node);
            }
            None => {
                self.head = Some(Rc::clone(&new_node));
                self.tail = Some(new_node);
            }
        }
    }

    #[allow(dead_code)]
    fn move_tail_to_other_head(&mut self, other: &mut PriceList) {
        if let Some(tail_node) = self.tail.take() {
            if let Some(prev_weak) = tail_node.borrow().previous_price.as_ref() {
                if let Some(prev_node) = prev_weak.upgrade() {
                    prev_node.borrow_mut().next_price = None;
                    self.tail = Some(Rc::clone(&prev_node));
                } else {
                    self.head = None;
                    self.tail = None;
                }
            } else {
                self.head = None;
            }

            tail_node.borrow_mut().previous_price = None;

            match other.head.take() {
                Some(old_head) => {
                    tail_node.borrow_mut().next_price = Some(Rc::clone(&old_head));
                    old_head.borrow_mut().previous_price = Some(Rc::downgrade(&tail_node));
                    other.head = Some(tail_node);
                }
                None => {
                    other.head = Some(tail_node.clone());
                    other.tail = Some(tail_node);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn move_head_to_other_tail(&mut self, other: &mut PriceList) {
        if let Some(head_node) = self.head.take() {
            if let Some(next_node) = head_node.borrow().next_price.as_ref() {
                next_node.borrow_mut().previous_price = None;
                self.head = Some(Rc::clone(next_node));
            } else {
                self.tail = None;
            }

            head_node.borrow_mut().next_price = None;

            match other.tail.take() {
                Some(old_tail) => {
                    head_node.borrow_mut().previous_price = Some(Rc::downgrade(&old_tail));
                    old_tail.borrow_mut().next_price = Some(Rc::clone(&head_node));
                    other.tail = Some(head_node);
                }
                None => {
                    other.head = Some(head_node.clone());
                    other.tail = Some(head_node);
                }
            }
        }
    }

    fn print_from_head(&self) {
        for node in self {
            let node = node.borrow();
            if node.order_task.is_none() {
                info!("------------------------[Direction]:🍵,Price:{:?}------------------------", format_ether(node.this_price))
            } else {
                info!("------------------------[Direction]:{:?},Price:{:?}------------------------",node.order_task.as_ref().unwrap().destination, format_ether(node.this_price))
            }
        }
    }
}


impl<'a> IntoIterator for &'a PriceList {
    type Item = Rc<RefCell<PriceNode>>;
    type IntoIter = PriceListIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
pub struct TradeParams {
    price_queue: PriceList,
    price_seg: U256,
    slippage: u64,
    deposited_usdt: U256,
    deposited_cess: U256,
    use_usdt_per_seg: U256,
}

pub fn new_trade_params(
    cess_latest_price: U256,
    grids_num: U256,
    grid_upper_limmit: U256,
    grid_lower_limmit: U256,
    slippage: u64,
    deposit_usdt: U256,
    deposit_cess: U256,
    price_tolerance_slippage: u64,
) -> Result<TradeParams> {
    if slippage >=1000 {
        bail!("The slippage tolerance is too low. Please adjust your settings!")
    }
    if price_tolerance_slippage >=1000 {
        bail!("The price tolerance slippage is too low. Please adjust your settings!")
    }

    if grid_lower_limmit >= grid_upper_limmit {
        return Err(anyhow!(
            "grid_upper_limmit must be greater than grid_lower_limmit"
        ));
    }
    let mut result = TradeParams {
        price_queue: PriceList::new(),
        price_seg: U256::ZERO,
        slippage,
        deposited_usdt: deposit_usdt,
        deposited_cess: deposit_cess,
        use_usdt_per_seg: U256::ZERO,
    };
    let per_seg = (grid_upper_limmit - grid_lower_limmit)/grids_num;
    result.price_seg = per_seg;
    result.use_usdt_per_seg = deposit_usdt / grids_num;
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
        let min_acceptable_price = slippage_compute_min(&price,price_tolerance_slippage);
        let max_acceptable_price = slippage_compute_max(&price,price_tolerance_slippage);

        if price > closest_price {
            let order = Order {
                mount: None,
                destination: TradeDestination::SELL,
                min_acceptable_price,
                max_acceptable_price
            };
            result.price_queue.push_back(price, Some(order));
        }
        if price < closest_price {
            let order = Order {
                mount: None,
                destination: TradeDestination::BUY,
                min_acceptable_price,
                max_acceptable_price
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

pub async fn trader_runner<P>(core: JinCore<P>) -> Result<()>
where
    P: Provider + Clone,
{
    let core = core
        .web3_state
        .ok_or(anyhow!("web3_state is None"))?
        .clone();

    //approve usdt && check usdt balances
    let usdt_token = core.usdt_token.clone();
    let my_wallet_address = usdt_token.wallet.default_signer().address();
    let usdt_num = usdt_token.balance_of(my_wallet_address).await?;
    info!(
        "The amount of USDT in your wallet is :{}",
        format_ether(usdt_num)
    );
    if usdt_num < core.deposit_usdt.clone() {
        bail!("Your wallet's USDT is not enough. Please put more USDT into your wallet!")
    };

    let usdt_allowance_num = usdt_token
        .allowance(
            None,
            Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                .context("Invalid address when get pancake swap router allowance in USDT")?,
        )
        .await?;
    if usdt_allowance_num == U256::MAX {
        info!("Your wallet's USDT has been approved for pancakeswap smart router contract use.");
    } else {
        info!(
            "Your wallet's USDT is not approved for pancakeswap smart router contract use or approval amount is not enough. Current approval amount is:{}. Start to approve USDT.",
            format_ether(usdt_allowance_num)
        );
        usdt_token
            .approve(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                    .context("Invalid address when approve USDT")?,
            )
            .await?;
        info!("USDT has been approved for pancakeswap smart router contract successfully!")
    }

    //approve cess && check cess balances
    let cess_token = core.cess_token.clone();
    let cess_num = cess_token.balance_of(my_wallet_address).await?;
    info!(
        "The amount of CESS in your wallet is :{}",
        format_ether(cess_num)
    );
    if cess_num < core.deposit_cess.clone() {
        bail!("Your wallet's CESS is not enough. Please put more CESS into your wallet!")
    }
    let cess_allowance_num = cess_token
        .allowance(
            None,
            Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                .context("Invalid address when get pancake swap router allowance in CESS")?,
        )
        .await?;
    if cess_allowance_num == U256::MAX {
        info!("Your wallet's CESS has been approved for pancakeswap smart router contract use.");
    } else {
        info!(
            "Your wallet's CESS is not approved for pancakeswap smart router contract use or approval amount is not enough. Current approval amount is:{}. Start to approve CESS.",
            format_ether(cess_allowance_num)
        );
        cess_token
            .approve(
                Address::from_str(PANCAKE_SWAP_SMART_ROUTER_V3)
                    .context("Invalid address when approve CESS")?,
            )
            .await?;
        info!("CESS has been approved for pancakeswap smart router contract successfully!")
    }

    //get trade params
    let pancakeswap_contract = core.pancakeswap_contract.clone();

    let cess_latest_price = check_cess_to_usdt_price(&pancakeswap_contract).await?;

    let trade_params = new_trade_params(
        cess_latest_price,
        core.grids_num.clone(),
        core.grid_upper_limmit.clone(),
        core.grid_lower_limmit.clone(),
        core.slippage.clone(),
        core.deposit_usdt.clone(),
        core.deposit_cess.clone(),
        core.price_tolerance_slippage,
    )?;
    info!(
        "------------------------1 CESS ==> {} USDT------------------------",
        format_ether(cess_latest_price)
    );
    trade_params.price_queue.print_from_head();

    // //buy cess token
    // let amount = (trade_params.use_usdt_per_seg.clone() / cess_latest_price) * get_u256_token(18);
    // let tx_hash = buy_cess_using_usdt(&pancakeswap_contract, amount, trade_params.slippage).await?;
    // info!("buy cess tx_hash is {:?}", tx_hash);

    // //sell cess token
    // let cess_to_sell = U256::from(200);
    // let cess_max_willing_pay = slippage_compute_max(&cess_to_sell,trade_params.slippage)*get_u256_token(18);
    // let amount_received = cess_to_sell*cess_latest_price;
    // info!("amount_received:{:?}",format_ether(amount_received));
    // info!("max willing pay CESS:{:?}",format_ether(cess_max_willing_pay));

    // let tx_hash = sell_cess_get_usdt(
    //     &pancakeswap_contract,
    //     cess_max_willing_pay,
    //     amount_received,
    // )
    // .await?;
    // info!("sell cess tx_hash is {:?}", tx_hash);

    let n = 1;

    info!("🤑🤑🤑Start to run robot scripts🤑🤑🤑");
    //run robot scripts
    while n == 1 {
        let mut add_task = false;
        let cess_price = check_cess_to_usdt_price(&pancakeswap_contract).await?;
        info!("[🔎]CESS new price is :{:?}",format_ether(cess_price));
        for node in &trade_params.price_queue {
            let node_data = node.borrow();
            match &node_data.order_task {
                Some(order) => {
                    //check if in the price range
                    if cess_price <= order.max_acceptable_price && order.min_acceptable_price <= cess_price {
                        let amount = match order.mount {
                            Some(mount) => {
                                //Have order task,need to trigger buy or sell.
                                mount
                            },
                            None => {
                                //Haven't order task,use use_usdt_per_seg/cess_price to compute how many cess token need be traded.
                                add_task=true;
                                trade_params.use_usdt_per_seg.clone()/cess_price
                            },
                        };

                        match order.destination {
                            TradeDestination::BUY => {
                                let tx_hash = buy_cess_using_usdt(&pancakeswap_contract, amount, trade_params.slippage).await?;
                                info!("[BUY💸]Price:{:?} triggered.Buy cess token with amount:{:?},Transaction hash is:{:?}",format_ether(cess_price),amount,tx_hash);
                                if add_task {
                                    match &node_data.next_price {
                                        Some(next_node) => {
                                            let sell_task = Order {
                                                mount: Some(amount),
                                                destination: TradeDestination::SELL,
                                                min_acceptable_price: slippage_compute_min(&next_node.borrow().this_price,trade_params.slippage),
                                                max_acceptable_price: slippage_compute_max(&next_node.borrow().this_price,trade_params.slippage),
                                            };
                                            next_node.borrow_mut().order_task = Some(sell_task);
                                        },
                                        None => {
                                            //logical error
                                            error!("A buy order occurred at the last price level, which shouldn't happen!!");
                                            bail!("A buy order occurred at the last price level, which shouldn't happen!!")
                                        }
                                    }
                                }
                            }
                            TradeDestination::SELL => {
                                let cess_to_sell = amount;
                                let cess_max_willing_pay = slippage_compute_max(&cess_to_sell,trade_params.slippage)*get_u256_token(18);
                                let usdt_amount_received = cess_to_sell*cess_price;
                                let tx_hash = sell_cess_get_usdt(
                                    &pancakeswap_contract,
                                    cess_max_willing_pay,
                                    usdt_amount_received,
                                )
                                .await?;
                                info!("[SELL💰]Price:{:?} triggered.Sell CESS token with amount:{:?},Transaction hash is:{:?}",format_ether(cess_price),amount,tx_hash);
                                if add_task {
                                    match &node_data.previous_price {
                                        Some(privious_node) => {
                                            match privious_node.upgrade() {
                                                Some(priv_node) => {
                                                    let buy_task = Order {
                                                        mount: Some(amount),
                                                        destination: TradeDestination::BUY,
                                                        min_acceptable_price: slippage_compute_min(&priv_node.borrow().this_price,trade_params.slippage),
                                                        max_acceptable_price: slippage_compute_max(&priv_node.borrow().this_price,trade_params.slippage),
                                                    };
                                                priv_node.borrow_mut().order_task = Some(buy_task);
                                                },
                                                None => {
                                                    //code error
                                                    error!("An error occurred while initializing the price chain — the head node's previous_node should be None!");
                                                    bail!("An error occurred while initializing the price chain — the head node's previous_node should be None!")
                                                }
                                            }
                                        },
                                        None => {
                                            //logical error
                                            error!("A sell trade occurred at the first price level, which shouldn't happen!!");
                                            bail!("A sell trade occurred at the first price level, which shouldn't happen!!")
                                        }
                                    }
                                }
                            },
                        }
                        //set this node order task to None
                        node.borrow_mut().order_task = None;
                        break
                    }
                },
                None => {
                    info!("⌛No order tasks found, indicating that the price has not fluctuated to any actionable level~~")
                },
            }
        }
        
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    Ok(())
}

async fn check_cess_to_usdt_price<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
) -> Result<U256> {
    let path = vec![
        Address::from_str(CESS_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(USDT_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    let amount_out = get_u256_token(18);
    let price_result = pancakeswap_contract
        .check_price_with_multi_hop(path.clone(), fee.clone(), amount_out)
        .await?;

    Ok(price_result.price)
}

async fn buy_cess_using_usdt<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
    amount:U256,
    slippage: u64,
) -> Result<String> {
    let path = vec![
        Address::from_str(USDT_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(CESS_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    let cess_min_received = slippage_compute_min(&amount,slippage);

    info!(
        "cess should received :{:?}",
        format_ether(amount)
    );
    info!("cess min received :{:?}", format_ether(cess_min_received));
    let tx_hash = pancakeswap_contract
        .swap_exact_inpute_tokens_for_tokens_with_multi_hop(
            path,
            fee,
            amount,
            cess_min_received,
        )
        .await?;
    Ok(tx_hash)
}

async fn sell_cess_get_usdt<P: Provider + Clone>(
    pancakeswap_contract: &PancakeswapContract<P>,
    cess_max_willing_pay:U256,
    amount_received:U256,
) -> Result<String> {
    let path = vec![
        Address::from_str(USDT_ADDRESS)?,
        Address::from_str(WBNB_ADDRESS)?,
        Address::from_str(CESS_ADDRESS)?,
    ];
    let fee = vec![100_u32, 100_u32];
    info!(
        "usdt want to received :{:?}",
        format_ether(amount_received)
    );
    let tx_hash = pancakeswap_contract
        .swap_exact_outpute_tokens_for_tokens_with_multi_hop(
            path,
            fee,
            amount_received,
            cess_max_willing_pay,
        )
        .await?;
    Ok(tx_hash)
}

