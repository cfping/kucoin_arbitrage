/// Executes triangular arbitrage
use kucoin_api::client::{Kucoin, KucoinEnv};
use kucoin_arbitrage::broker::symbol::filter::symbol_with_quotes;
use kucoin_arbitrage::broker::symbol::kucoin::{format_subscription_list, get_symbols};
use kucoin_arbitrage::event;
use kucoin_arbitrage::monitor::counter;
use kucoin_arbitrage::monitor::task::{task_log_mps, task_monitor_channel_mps};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast::channel;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // logging format
    kucoin_arbitrage::logger::log_init();
    log::info!("Log setup");

    // credentials
    let config = kucoin_arbitrage::config::from_file("config.toml")?;

    tokio::select! {
        _ = task_signal_handle() => println!("received external signal, terminating program"),
        res = core(config) => println!("core ended first {res:?}"),
    };

    println!("Good bye!");
    Ok(())
}

async fn core(config: kucoin_arbitrage::config::Config) -> Result<(), failure::Error> {
    // config parameters
    let monitor_interval = config.behaviour.monitor_interval_sec;

    // API endpoints
    let api = Kucoin::new(KucoinEnv::Live, Some(config.kucoin_credentials()))?;
    log::info!("Credentials setup");

    // get all symbols concurrently
    let symbol_list = get_symbols(api.clone()).await;
    log::info!("Total exchange symbols: {:?}", symbol_list.len());

    // filter with either btc or usdt as quote
    let symbol_infos = symbol_with_quotes(&symbol_list, "BTC", "USDT");
    log::info!("Total symbols in scope: {:?}", symbol_infos.len());

    // list subscription using the filtered symbols
    let subs = format_subscription_list(&symbol_infos);
    log::info!("Total orderbook WS sessions: {:?}", subs.len());

    // broadcast channels and counters
    let cx_orderbook = Arc::new(Mutex::new(counter::Counter::new("orderbook")));
    let tx_orderbook = channel::<event::orderbook::OrderbookEvent>(1024 * 2).0;
    let cx_orderbook_best = Arc::new(Mutex::new(counter::Counter::new("best_price")));
    let tx_orderbook_best = channel::<event::orderbook::OrderbookEvent>(512).0;
    let cx_chance = Arc::new(Mutex::new(counter::Counter::new("chance")));
    let tx_chance = channel::<event::chance::ChanceEvent>(64).0;
    log::info!("Broadcast channels setup");

    // MPS monitor tasks
    let mut taskpool_monitor = JoinSet::new();
    taskpool_monitor.spawn(task_monitor_channel_mps(
        tx_orderbook.subscribe(),
        cx_orderbook.clone(),
    ));
    taskpool_monitor.spawn(task_monitor_channel_mps(
        tx_orderbook_best.subscribe(),
        cx_orderbook_best.clone(),
    ));
    taskpool_monitor.spawn(task_monitor_channel_mps(
        tx_chance.subscribe(),
        cx_chance.clone(),
    ));
    taskpool_monitor.spawn(task_log_mps(
        vec![
            cx_orderbook.clone(),
            cx_orderbook_best.clone(),
            cx_chance.clone(),
        ],
        monitor_interval as u64,
    ));

    // terminate if taskpools failed
    let message = tokio::select! {
        res = taskpool_monitor.join_next() =>
            format!("Infrastructure task pool error [{res:?}]"),
    };
    Err(failure::err_msg(format!("unexpected error [{message}]")))
}

/// wait for any external terminating signal
async fn task_signal_handle() -> Result<(), failure::Error> {
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => exit_program("SIGTERM").await?,
        _ = sigint.recv() => exit_program("SIGINT").await?,
    };
    Ok(())
}

/// handle external signal
async fn exit_program(signal_alias: &str) -> Result<(), failure::Error> {
    log::info!("Received [{signal_alias}] signal");
    Ok(())
}