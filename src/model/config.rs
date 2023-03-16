#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub monitor_interval_sec: u64,
    pub api_key: &'static str,
    pub secret_key: &'static str,
    pub passphrase: &'static str,
}