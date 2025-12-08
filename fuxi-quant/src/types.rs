use fuxi_quant_runtime::runtime::Script;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;

/// 运行模式
#[derive(Serialize, Deserialize, Default)]
pub enum Mode {
    /// 回测
    #[default]
    Backtest,
    /// 参数优化
    Optimize,
    /// 沙盒
    Sandbox,
    /// 实盘
    Mainnet,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub struct Config {
    pub script: Script,
    pub mode: Mode,
    pub gas_max: u64,
    pub log: Log,
    pub backtest: Option<Backtest>,
    pub optimize: Option<Optimize>,
    pub sandbox: Option<Sandbox>,
    pub mainnet: Option<Mainnet>,
}

/// 运行模式
#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => Self::OFF,
            LogLevel::Trace => Self::TRACE,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default, Clone, Copy)]
pub struct Log {
    pub level: LogLevel,
    pub show_span_timing: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Backtest {
    pub codes: Vec<String>,
    pub start_time: String,
    pub end_time: String,
    pub cash: Decimal,
    pub history_bar_len: usize,
    pub maker_fee_rate: Decimal,
    pub taker_fee_rate: Decimal,
    pub slippage: Decimal,
    pub data_dir: PathBuf,
}

impl Default for Backtest {
    fn default() -> Self {
        Self {
            codes: Default::default(),
            start_time: Default::default(),
            end_time: Default::default(),
            cash: dec!(1000),
            history_bar_len: 30,
            maker_fee_rate: dec!(0.0002),
            taker_fee_rate: dec!(0.0005),
            slippage: dec!(0.0005),
            data_dir: PathBuf::from(".cache"),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Optimize {}

#[derive(Serialize, Deserialize, Default)]
pub struct Sandbox {}

#[derive(Serialize, Deserialize, Default)]
pub struct Mainnet {}
