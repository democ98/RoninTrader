use anyhow::Result;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Serialize;
use serde_urlencoded;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct OrderParams {
    symbol: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    #[serde(rename = "timeInForce")]
    time_in_force: String,
    quantity: String,
    price: String,
    timestamp: u64,
    #[serde(rename = "recvWindow")]
    recv_window: u64,
}

fn generate_signature(params: &str, secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("Invalid key");
    mac.update(params.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

pub async fn order_test() -> Result<()> {
    let api_key = "";
    let api_secret = "";

    let client = Client::new();

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

    let params = OrderParams {
        symbol: "BTCUSDT".to_string(),
        side: "BUY".to_string(),
        order_type: "LIMIT".to_string(),
        time_in_force: "GTC".to_string(),
        quantity: "0.001".to_string(),
        price: "50000.00".to_string(),
        timestamp,
        recv_window: 5000,
    };
    let query_string = serde_urlencoded::to_string(&params).unwrap();
    let signature = generate_signature(&query_string, api_secret);

    let url = format!(
        "https://api.binance.com/api/v3/order/test?{}&signature={}",
        query_string, signature
    );

    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        println!("response : {:?}", body);
    } else {
        println!("error : {}", response.text().await?);
    }

    Ok(())
}

pub async fn exchange_info() -> Result<()> {
    Ok(())
}
