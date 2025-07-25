use alloy::primitives::utils::format_ether;
use alloy::{primitives::U256, providers::Provider};
use log::info;
use pancakeswap::{bep_20::BEP20TOKEN, smartswap::PancakeswapContract};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct Web3State<P: Provider> {
    pub wbnb_token: BEP20TOKEN<P>,
    pub cess_token: BEP20TOKEN<P>,
    pub usdt_token: BEP20TOKEN<P>,
    pub pancakeswap_contract: PancakeswapContract<P>,

    pub web3_trade_core: Web3TradeCore,
}

#[derive(Clone, Default)]
pub struct Web3TradeCore {
    pub price_queue: PriceList,
    pub price_seg: U256,
    pub slippage: u64,
    pub deposited_usdt: U256,
    pub deposited_cess: U256,
    pub use_usdt_per_seg: U256,
    pub trade_record_path: String,
}

#[derive(Debug, Clone)]
pub enum TradeDestination {
    SELL,
    BUY,
}

pub struct Order {
    pub mount: Option<U256>, //If it's just an initial order placement, this value will be None. If the trade is triggered by a neighboring node, this value will be Some.
    pub destination: TradeDestination,
    pub min_acceptable_price: U256,
    pub max_acceptable_price: U256,
}

pub struct PriceNode {
    pub this_price: U256,
    pub order_task: Option<Order>,
    pub previous_price: Option<Weak<RefCell<PriceNode>>>,
    pub next_price: Option<Rc<RefCell<PriceNode>>>,
}
impl PriceNode {
    pub fn new(price: U256, order: Option<Order>) -> Self {
        Self {
            this_price: price,
            order_task: order,
            previous_price: None,
            next_price: None,
        }
    }
}

pub struct PriceListIter {
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

#[derive(Clone, Default)]
pub struct PriceList {
    pub head: Option<Rc<RefCell<PriceNode>>>,
    pub tail: Option<Rc<RefCell<PriceNode>>>,
}

impl PriceList {
    pub fn new() -> Self {
        PriceList {
            head: None,
            tail: None,
        }
    }
    pub fn iter(&self) -> PriceListIter {
        PriceListIter {
            current: self.head.clone(),
        }
    }
    pub fn push_back(&mut self, price: U256, order: Option<Order>) {
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
    pub fn push_front(&mut self, price: U256, order: Option<Order>) {
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
    pub fn move_tail_to_other_head(&mut self, other: &mut PriceList) {
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
    pub fn move_head_to_other_tail(&mut self, other: &mut PriceList) {
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

    pub fn print_from_head(&self) {
        for node in self {
            let node = node.borrow();
            if node.order_task.is_none() {
                info!(
                    "------------------------[Direction]:🍵,Price:{:?}------------------------",
                    format_ether(node.this_price)
                )
            } else {
                info!(
                    "------------------------[Direction]:{:?},Price:{:?}------------------------",
                    node.order_task.as_ref().unwrap().destination,
                    format_ether(node.this_price)
                )
            }
        }
    }
    pub fn print_from_head_to_file(&self, file_path: &str) {
        let mut output_to_file = Vec::new();
        let title =
            "=========================【ORDER STATUS】=========================".to_string();
        output_to_file.push(title);
        for node in self {
            let node = node.borrow();
            if node.order_task.is_none() {
                let line = format!(
                    "------------------------[Direction]:🍵,[Price]:{:?},[Order Quantity]:NO ORDER------------------------",
                    format_ether(node.this_price)
                );
                output_to_file.push(line);
            } else {
                let order_task = node.order_task.as_ref().unwrap();
                let line = match order_task.mount {
                    Some(mount) => {
                        let line = format!(
                            "------------------------[Direction]:{:?},Price:{:?},[Order Quantity]:{:?}------------------------",
                            order_task.destination,
                            format_ether(node.this_price),
                            format_ether(mount)
                        );
                        line
                    }
                    None => {
                        let line = format!(
                            "------------------------[Direction]:{:?},Price:{:?},[Order Quantity]:NO ORDER------------------------",
                            order_task.destination,
                            format_ether(node.this_price),
                        );
                        line
                    }
                };
                output_to_file.push(line);
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
