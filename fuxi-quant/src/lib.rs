mod log;
pub mod types;

use crate::types::Mode;
use anyhow::{Result, ensure};
use fuxi_quant_core::{backtest::Backtest, helpers::*};
use fuxi_quant_runtime::runtime::ScriptStrategy;
use types::Config;

pub fn run(config: Config) -> Result<()> {
    let subscriber =
        crate::log::new_subscriber(config.log.level.into(), config.log.show_span_timing);

    tracing::subscriber::with_default(subscriber, || {
        match &config.mode {
            Mode::Backtest => {
                ensure!(config.backtest.is_some());
                let backtest_config = config.backtest.as_ref().unwrap();

                fuxi_quant_core::backtest::history::sync_bars(
                    &backtest_config.data_dir.to_string_lossy(),
                    &backtest_config.codes,
                )?;

                let strategy = ScriptStrategy::new(&config.script, config.gas_max)?;

                let mut backtest = Backtest::new(
                    Box::new(strategy),
                    &backtest_config.codes,
                    time_from_str(&backtest_config.start_time)?,
                    time_from_str(&backtest_config.end_time)?,
                    backtest_config.cash,
                    backtest_config.history_bar_len,
                    backtest_config.maker_fee_rate,
                    backtest_config.taker_fee_rate,
                    backtest_config.slippage,
                    &backtest_config.data_dir.to_string_lossy(),
                )?;

                let report = backtest.run()?;
                println!("{} 回测报告 {}", "-".repeat(30), "-".repeat(30));
                println!("　　　收益率: {:.2}%", report.ret * 100.0);
                println!("　　　　年化: {:.2}%", report.ar * 100.0);
                println!("　　　　回撤: {:.2}%", report.mdd * 100.0);
                println!("　　　波动率: {:.2}%", report.vol * 100.0);
                println!("　　夏普比率: {:.8}", report.sr);
                println!("　索提诺比率: {:.8}", report.sor);
                println!("　　卡玛比率: {:.8}", report.cr);
                println!("　　　　胜率: {:.2}%", report.win_rate * 100.0);
                println!("　　　盈亏比: {:.2}", report.pl_ratio);
                println!("　　交易次数: {}", report.trade_cnt);
                println!("　　　手续费: {:.2}", report.fee);
            }
            Mode::Optimize => todo!(),
            Mode::Sandbox => todo!(),
            Mode::Mainnet => todo!(),
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Backtest, Log, LogLevel};
    use fuxi_quant_runtime::runtime::Script;
    use std::path::PathBuf;

    #[test]
    fn test_run() -> Result<()> {
        let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_dir.pop();
        data_dir.push(".cache");

        println!("data dir: {data_dir:?}");

        let config = Config {
            script: Script::Source(
                r#"
                    fn on_start() {}
                    fn on_stop() {}
                    fn on_bar(code) {}
                    fn on_signal() {
                        print("hello");
                    }
                    fn on_timer(timer, time) {}
                    fn on_order(order_id) {}
                    fn on_position(code) {}
                "#
                .into(),
            ),
            mode: Mode::Backtest,
            gas_max: 99999999,
            log: Log {
                level: LogLevel::Trace,
                show_span_timing: true,
            },
            backtest: Some(Backtest {
                codes: vec!["BTC".into()],
                start_time: "2023-10-01".into(),
                end_time: "2023-10-02".into(),
                history_bar_len: 0,
                data_dir,
                ..Default::default()
            }),
            ..Default::default()
        };
        run(config)
    }
}
