/// Test latency between order and private channel order detection
/// place extreme order, receive extreme order, check time difference
extern crate kucoin_rs;
use chrono::prelude::Local;
use kucoin_arbitrage::model::order::OrderSide;
use kucoin_arbitrage::translator::translator::OrderBookChangeTranslator;
use kucoin_rs::failure;
use kucoin_rs::futures::TryStreamExt;
use kucoin_rs::kucoin::{
    client::{Kucoin, KucoinEnv},
    model::websocket::{KucoinWebsocketMsg, WSTopic, WSType},
};
use uuid::Uuid;

/// main function
#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // provide logging format
    kucoin_arbitrage::logger::log_init();
    log::info!("Testing Kucoin REST-to-WS latency");
    let credentials = kucoin_arbitrage::globals::config::credentials();
    log::info!("{credentials:#?}");
    // Initialize the Kucoin API struct
    let api = Kucoin::new(KucoinEnv::Live, Some(credentials))?;
    let url = api.get_socket_endpoint(WSType::Public).await?;
    let mut ws = api.websocket();

    let subs = vec![WSTopic::OrderBook(vec!["BTC-USDT".to_string()])];
    // extreme order
    let id: Uuid = Uuid::new_v4();
    let test_symbol: &str = "BTC-USDT";
    let test_price: f64 = 1.0; // buying BTC at 1 USD, which cannot happen as of 2023
    let test_volume: f64 = 0.1;

    let dt_order_placed = Local::now();

    api.cancel_all_orders(None, None).await.unwrap();
    // TODO set a valid limit order
    api.post_limit_order(
        id.to_string().as_str(),
        test_symbol,
        OrderSide::Buy.as_ref(),
        test_price.to_string().as_str(),
        test_volume.to_string().as_str(),
        None,
    )
    .await?;
    log::info!("Order placed {dt_order_placed}");
    ws.subscribe(url, subs).await?;

    log::info!("Async polling");
    let serial = 0;
    while let Some(msg) = ws.try_next().await? {
        match msg {
            KucoinWebsocketMsg::OrderBookMsg(msg) => {
                let (symbol, data) = msg.data.to_internal(serial);
                // match symbol
                if symbol.ne(test_symbol) {
                    continue;
                }
                // BTC-USDT now, check bid volume
                if let Some(_) = data.bid.get(&ordered_float::OrderedFloat(test_price)) {
                    // price
                    log::info!("data: {:#?}", data);
                    // volume might not be equal, as they are cumulative with other previous orders

                    let dt_order_reported = Local::now();
                    let delta = dt_order_reported - dt_order_placed;
                    log::info!("REST-to-WS: {}ms", delta.num_milliseconds());
                    // I generally get around 2.4s to 3.0s
                    return Ok(());
                }
            }
            KucoinWebsocketMsg::PongMsg(_) => continue,
            KucoinWebsocketMsg::WelcomeMsg(_) => continue,
            _ => {
                panic!("unexpected msgs received: {msg:?}")
            }
        }
    }
    Ok(())
}