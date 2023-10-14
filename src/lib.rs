/// Bridge between API calls and internal data structures, with tasks, functions and structs
pub mod broker;
/// Config file reader
pub mod config;
/// Custom error
pub mod error;
/// API independent event enums
pub mod event;
/// Logger intialization
pub mod logger;
/// API independent model struct for both system and multi-exchange support
pub mod model;
/// Everything declared with lazy_statics!. Refrain from overusing lazy_statics, pass variables instead
pub mod monitor;
/// API independent arbitrage strategy algorithms
pub mod strategy;
/// String functions
pub mod strings;
/// Traits/impl to convert between API crate models and internal models
pub mod translator;
