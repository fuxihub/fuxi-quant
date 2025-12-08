use crate::{helpers::*, types::*};
use anyhow::{Result, anyhow, ensure};
use chrono::{Duration, DurationRound, Timelike};
use polars::frame::DataFrame;
use rust_decimal::prelude::*;

/// 回测引擎
pub struct Backtest {
    /// 上下文
    pub context: Context,
    /// 策略
    pub strategy: Box<dyn Strategy>,
    /// 初始资金
    pub init_cash: Decimal,
    /// 挂单手续费率
    pub maker_fee_rate: Decimal,
    /// 吃单手续费率
    pub taker_fee_rate: Decimal,
    /// 滑点
    pub slippage: Decimal,
    /// 历史K线长度
    pub history_bar_len: usize,
    /// K线索引
    pub bar_idx: usize,
    /// 开始时间
    pub start_time: Time,
    /// 结束时间
    pub end_time: Time,
    /// 当前时间
    pub curr_time: Time,
    /// 历史权益
    pub history_equities: Vec<f64>,
    /// 交易记录
    pub trades: Vec<Trade>,
}

impl Backtest {
    pub fn new(
        strategy: Box<dyn Strategy>,
        codes: &[String],
        start_time: Time,
        end_time: Time,
        cash: Decimal,
        history_bar_len: usize,
        maker_fee_rate: Decimal,
        taker_fee_rate: Decimal,
        slippage: Decimal,
        data_dir: &str,
    ) -> Result<Self> {
        let start_time = start_time.duration_trunc(Duration::minutes(1))?;
        let end_time = end_time.duration_trunc(Duration::minutes(1))?;

        ensure!(end_time >= start_time);
        ensure!(!codes.is_empty());
        ensure!(!cash.is_sign_negative());
        ensure!(!slippage.is_sign_negative());

        let mut context = Context::new(cash, codes)?;

        let mut bars = Map::with_capacity(codes.len());
        for code in codes {
            bars.insert(
                code.clone(),
                bars::load(data_dir, code, start_time, end_time, history_bar_len)?,
            );
        }
        context.bars = bars;

        Ok(Self {
            context,
            strategy,
            history_bar_len,
            bar_idx: history_bar_len,
            start_time,
            end_time,
            curr_time: start_time,
            maker_fee_rate,
            taker_fee_rate,
            slippage,
            init_cash: cash,
            history_equities: Default::default(),
            trades: Default::default(),
        })
    }
}

impl Backtest {
    #[tracing::instrument(skip_all)]
    fn cross_order(&mut self) -> Result<()> {
        let orders_by_code: Map<String, Vec<(String, Order)>> = self
            .context
            .orders
            .iter()
            .filter(|(_, order)| {
                matches!(
                    order.status,
                    OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
                )
            })
            .map(|(id, order)| (id.clone(), order.clone()))
            .fold(Map::new(), |mut acc, (id, order)| {
                acc.entry(order.code.clone()).or_default().push((id, order));
                acc
            });

        for (code, orders) in orders_by_code {
            let bars = self.context.bars.get(&code).unwrap();

            let open_price = Decimal::try_from(df_f64(bars, "open", self.bar_idx)?)?;
            let high_price = Decimal::try_from(df_f64(bars, "high", self.bar_idx)?)?;
            let low_price = Decimal::try_from(df_f64(bars, "low", self.bar_idx)?)?;

            for (order_id, mut order) in orders {
                let (can_fill, fill_price) = match order.type_ {
                    OrderType::Market => {
                        let price = match order.side {
                            Side::Buy => open_price * (Decimal::ONE + self.slippage),
                            Side::Sell => open_price * (Decimal::ONE - self.slippage),
                        };
                        (true, price)
                    }
                    OrderType::Limit => {
                        let limit_price = order.price.unwrap();
                        match order.side {
                            Side::Buy => {
                                if limit_price >= open_price {
                                    // 开盘价匹配，以开盘价成交
                                    (true, open_price)
                                } else if limit_price >= low_price {
                                    // 盘中匹配，以限价成交
                                    (true, limit_price)
                                } else {
                                    (false, Decimal::ZERO)
                                }
                            }
                            Side::Sell => {
                                if limit_price <= open_price {
                                    // 开盘价匹配，以开盘价成交
                                    (true, open_price)
                                } else if limit_price <= high_price {
                                    // 盘中匹配，以限价成交
                                    (true, limit_price)
                                } else {
                                    (false, Decimal::ZERO)
                                }
                            }
                        }
                    }
                };

                if can_fill {
                    let orig_status = order.status;

                    order.filled = order.size;
                    order.status = OrderStatus::Filled;

                    let pos = self.context.positions.get_mut(&order.code).unwrap();

                    // 根据方向获取对应的 price/size
                    let (old_pos_price, old_pos_size) = match order.direction {
                        Direction::Long => (pos.long.price, pos.long.size),
                        Direction::Short => (pos.short.price, pos.short.size),
                    };

                    let cash = fill_price * order.size;

                    // 手续费率：挂单 vs 吃单
                    let fee_rate = if orig_status == OrderStatus::Pending {
                        self.maker_fee_rate
                    } else {
                        self.taker_fee_rate
                    };

                    let fee_cash = cash * fee_rate;

                    let mut rpl = Decimal::ZERO;
                    let new_pos_size;
                    let mut new_pos_price = old_pos_price;

                    // 合约交易逻辑
                    // Long+Buy=做多开仓, Long+Sell=做多平仓
                    // Short+Sell=做空开仓, Short+Buy=做空平仓
                    let is_open = matches!(
                        (order.direction, order.side),
                        (Direction::Long, Side::Buy) | (Direction::Short, Side::Sell)
                    );

                    if is_open {
                        // 开仓
                        new_pos_size = old_pos_size + order.size;
                        if old_pos_size > Decimal::ZERO {
                            new_pos_price = (old_pos_price * old_pos_size
                                + fill_price * order.size)
                                / new_pos_size;
                        } else {
                            new_pos_price = fill_price;
                        }
                        self.context.cash -= fee_cash;
                    } else {
                        // 平仓
                        rpl = match order.direction {
                            Direction::Long => (fill_price - old_pos_price) * order.size - fee_cash,
                            Direction::Short => {
                                (old_pos_price - fill_price) * order.size - fee_cash
                            }
                        };
                        new_pos_size = old_pos_size - order.size;
                        self.context.cash += rpl;
                    }

                    // 更新持仓
                    match order.direction {
                        Direction::Long => {
                            pos.long.price = new_pos_price;
                            pos.long.size = new_pos_size;
                        }
                        Direction::Short => {
                            pos.short.price = new_pos_price;
                            pos.short.size = new_pos_size;
                        }
                    }

                    let trade = Trade {
                        id: order_id.clone(),
                        time: self.curr_time,
                        code: order.code.clone(),
                        direction: order.direction,
                        side: order.side,
                        price: fill_price,
                        size: order.size,
                        fee: fee_cash,
                        rpl,
                    };
                    self.trades.push(trade);

                    let order_code = order.code.clone();
                    self.context.orders.insert(order_id.clone(), order);

                    unsafe {
                        let this = self as *mut Backtest;
                        self.strategy.on_order(&mut *this, &order_id)?;
                        self.strategy.on_position(&mut *this, &order_code)?;
                    }

                    self.context.orders.swap_remove(&order_id);
                } else {
                    let order = self.context.orders.get_mut(&order_id).unwrap();
                    let should_remove = match order.status {
                        OrderStatus::Canceling => {
                            order.status = OrderStatus::Canceled;
                            true
                        }
                        OrderStatus::New => {
                            order.status = OrderStatus::Pending;
                            false
                        }
                        _ => false,
                    };

                    if should_remove {
                        unsafe {
                            let this = self as *mut Backtest;
                            self.strategy.on_order(&mut *this, &order_id)?;
                        }
                        self.context.orders.swap_remove(&order_id);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Engine for Backtest {
    fn get_context(&self) -> &Context {
        &self.context
    }

    fn get_time(&self) -> Time {
        self.curr_time
    }

    fn get_bars(&self, code: &str, all: bool) -> Result<DataFrame> {
        let bars = self
            .context
            .bars
            .get(code)
            .ok_or_else(|| anyhow!("bars not found: {}", code))?;
        if all {
            Ok(bars.clone())
        } else {
            let end_idx = self.bar_idx.min(bars.height());
            let hist_len = if self.history_bar_len == 0 {
                10
            } else {
                self.history_bar_len
            };
            let start_idx = end_idx.saturating_sub(hist_len);
            let length = end_idx - start_idx;
            let mut result = bars.slice(start_idx as i64, length);
            if result.should_rechunk() {
                result.rechunk_mut();
            }
            Ok(result)
        }
    }

    fn get_signals(&self) -> DataFrame {
        let signals_height = self.context.signals.height();
        if signals_height == 0 || self.bar_idx == 0 {
            return Default::default();
        }
        let end_idx = self.bar_idx.min(signals_height);
        let hist_len = if self.history_bar_len == 0 {
            10
        } else {
            self.history_bar_len
        };
        let start_idx = end_idx.saturating_sub(hist_len);
        let length = end_idx - start_idx;
        let mut result = self.context.signals.slice(start_idx as i64, length);
        if result.should_rechunk() {
            result.rechunk_mut();
        }
        result
    }

    fn set_signals(&mut self, mut signals: DataFrame) -> Result<()> {
        let req_len = self
            .context
            .bars
            .values()
            .next()
            .ok_or_else(|| anyhow!("bars not found"))?
            .height();
        ensure!(signals.height() == req_len);
        if signals.should_rechunk() {
            signals.rechunk_mut();
        }
        self.context.signals = signals;
        Ok(())
    }

    fn set_lever(&mut self, code: &str, lever: u32) -> Result<()> {
        let symbol = self
            .context
            .symbols
            .get(code)
            .ok_or_else(|| anyhow!("symbol not found: {}", code))?;
        let lever = Decimal::from(lever);
        ensure!(lever >= Decimal::ONE && lever <= symbol.max_lever);
        let pos = self.context.positions.get_mut(code).unwrap();
        pos.lever = lever;
        Ok(())
    }

    fn place_order(
        &mut self,
        code: &str,
        type_: OrderType,
        direction: Direction,
        side: Side,
        size: Decimal,
        price: Option<Decimal>,
    ) -> Result<String> {
        let symbol = self
            .context
            .symbols
            .get(code)
            .ok_or_else(|| anyhow!("symbol not found: {}", code))?;

        match type_ {
            OrderType::Market => ensure!(price.is_none(), "market order must not have price"),
            OrderType::Limit => ensure!(price.is_some(), "limit order must have price"),
        }

        let actual_price = match type_ {
            OrderType::Market => symbol.price,
            OrderType::Limit => price.ok_or_else(|| anyhow!("limit order must have price"))?,
        };

        ensure!(actual_price > Decimal::ZERO);
        ensure!(size >= symbol.min_size);

        let cash = actual_price * size;
        ensure!(cash >= symbol.min_cash);

        // 合约交易资金和持仓检查
        // Long+Buy=做多开仓, Long+Sell=做多平仓
        // Short+Sell=做空开仓, Short+Buy=做空平仓
        let is_open = matches!(
            (direction, side),
            (Direction::Long, Side::Buy) | (Direction::Short, Side::Sell)
        );

        if is_open {
            // 开仓检查资金
            let pos = self.context.positions.get(code).unwrap();
            let required_cash = cash / pos.lever;
            let avail_cash = self.context.calc_avail_cash();
            ensure!(avail_cash >= required_cash);
        } else {
            // 平仓检查持仓
            let avail_size = self.context.calc_pos_avail_size(code, direction);
            ensure!(avail_size >= size);
        }

        let order_id = id_new();
        let order = Order {
            id: order_id.clone(),
            code: code.to_string(),
            type_,
            direction,
            side,
            price,
            size,
            filled: Decimal::ZERO,
            status: OrderStatus::New,
            time: self.curr_time,
        };

        self.context.orders.insert(order_id.clone(), order);

        Ok(order_id)
    }

    fn cancel_order(&mut self, id: &str) -> Result<()> {
        let Some(order) = self.context.orders.get_mut(id) else {
            return Ok(());
        };

        ensure!(matches!(
            order.status,
            OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
        ));

        order.status = OrderStatus::Canceling;

        Ok(())
    }
}

impl Backtest {
    #[tracing::instrument(name = "backtest", skip_all)]
    pub fn run(&mut self) -> Result<Report> {
        let codes = self
            .context
            .symbols
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        for code in &codes {
            let bars_df = self.context.bars.get(code).unwrap();
            let open_price = Decimal::try_from(df_f64(bars_df, "open", self.bar_idx)?)?;
            let symbol = self.context.symbols.get_mut(code).unwrap();
            symbol.mark_price = open_price;
            symbol.price = open_price;
        }

        self.history_equities
            .push(self.context.calc_equity().to_f64().unwrap_or(0.0));

        unsafe {
            let this = self as *mut Backtest;
            self.strategy.on_start(&mut *this)?;
        }

        for code in &codes {
            unsafe {
                let this = self as *mut Backtest;
                self.strategy.on_bar(&mut *this, code)?;
            }
        }

        while self.curr_time < self.end_time {
            let span = tracing::info_span!(
                "",
                ________topic________ = time_to_str(&self.curr_time, Some(TIME_FMT_CPT))
            );
            let _guard = span.enter();

            for code in &codes {
                let bars_df = self.context.bars.get(code).unwrap();
                let open_price = Decimal::try_from(df_f64(bars_df, "open", self.bar_idx)?)?;
                let symbol = self.context.symbols.get_mut(code).unwrap();
                symbol.mark_price = open_price;
                symbol.price = open_price;
            }

            self.cross_order()?;

            self.bar_idx += 1;
            self.curr_time += Duration::minutes(1);

            self.history_equities
                .push(self.context.calc_equity().to_f64().unwrap_or(0.0));

            unsafe {
                let this = self as *mut Backtest;
                self.strategy.on_signal(&mut *this)?;
            }

            let min = self.curr_time.minute();
            let hour = self.curr_time.hour();

            // 每秒事件（回测中每分钟触发一次）
            unsafe {
                let this = self as *mut Backtest;
                self.strategy
                    .on_timer(&mut *this, Timer::Secondly, self.curr_time)?;
            }

            // 每分钟事件
            unsafe {
                let this = self as *mut Backtest;
                self.strategy
                    .on_timer(&mut *this, Timer::Minutely, self.curr_time)?;
            }

            if min == 0 {
                unsafe {
                    let this = self as *mut Backtest;
                    self.strategy
                        .on_timer(&mut *this, Timer::Hourly, self.curr_time)?;
                }
            }

            if min == 0 && hour == 0 {
                unsafe {
                    let this = self as *mut Backtest;
                    self.strategy
                        .on_timer(&mut *this, Timer::Daily, self.curr_time)?;
                }
            }
        }

        unsafe {
            let this = self as *mut Backtest;
            self.strategy.on_stop(&mut *this)?;
        }

        Report::new(
            self.init_cash.to_f64().unwrap_or(0.0),
            self.context.calc_equity().to_f64().unwrap_or(0.0),
            &self.history_equities,
            self.start_time,
            self.end_time,
            &self.trades,
        )
    }
}

mod bars {
    use crate::{helpers::*, types::*};
    use anyhow::{Result, bail, ensure};
    use chrono::Duration;
    use polars::prelude::*;
    use std::path::PathBuf;

    #[tracing::instrument(skip_all)]
    pub fn load(
        data_dir: &str,
        code: &str,
        start_time: Time,
        end_time: Time,
        history_bar_len: usize,
    ) -> Result<DataFrame> {
        let data_path = PathBuf::from(data_dir)
            .join("bars")
            .join(format!("{code}.data"));

        ensure!(data_path.exists());

        let start_naive = start_time.naive_utc() - Duration::minutes(history_bar_len as i64);
        let end_naive = end_time.naive_utc();

        let time_series = polars::time::date_range(
            "time".into(),
            start_naive,
            end_naive,
            polars::time::Duration::parse("1m"),
            ClosedWindow::Both,
            TimeUnit::Milliseconds,
            Some(&TIME_TZ),
        )?
        .into_column();

        let time_df = DataFrame::new(vec![time_series])?;

        let file = std::fs::File::open(&data_path)?;
        let data_df = IpcReader::new(file).finish()?;

        let mut result = time_df
            .lazy()
            .join(
                data_df.lazy(),
                [col("time")],
                [col("time")],
                JoinArgs::new(JoinType::Left),
            )
            .collect()?;

        if result.should_rechunk() {
            result.rechunk_mut();
        }

        let incomplete_rs = result
            .clone()
            .lazy()
            .filter(
                col("open")
                    .is_null()
                    .or(col("high").is_null())
                    .or(col("low").is_null())
                    .or(col("close").is_null())
                    .or(col("size").is_null())
                    .or(col("cash").is_null()),
            )
            .select([col("time").cast(DataType::Int64).alias("time_ms")])
            .collect()?;

        let incomplete_cnt = incomplete_rs.height() as u32;

        if incomplete_cnt > 0 {
            let time_col = incomplete_rs.column("time_ms")?.i64()?;

            let mut dates = Vec::new();

            for idx in 0..time_col.len() {
                if let Some(ms) = time_col.get(idx) {
                    dates.push(time_from_millis(ms)?.format(TIME_FMT).to_string());
                }
            }

            bail!("缺失数据: 交易对={code}, 缺失数量={incomplete_cnt}, 缺失日期={dates:?}");
        }

        Ok(result)
    }
}

pub mod history {
    use crate::helpers::*;
    use ::zip::ZipArchive;
    use anyhow::{Result, anyhow};
    use chrono::{Datelike, TimeZone};
    use polars::prelude::*;
    use reqwest::Client;
    use std::{io::Cursor, path::PathBuf};
    use tokio::{
        task::spawn_blocking,
        time::{Duration, sleep},
    };

    const START_YEAR: i32 = 2020;
    const START_MONTH: u32 = 1;

    #[tracing::instrument(skip_all)]
    #[tokio::main(flavor = "current_thread")]
    pub async fn sync_bars(dir: &str, codes: &[String]) -> Result<()> {
        sync(dir, codes).await
    }

    #[tracing::instrument(skip_all)]
    async fn sync(dir: &str, codes: &[String]) -> Result<()> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        for code in codes {
            let feather_path = PathBuf::from(dir)
                .join("bars")
                .join(format!("{}.data", code));

            if feather_path.exists() {
                tracing::trace!("数据已存在: {code}");
                continue;
            }

            let now_utc = chrono::Utc::now();
            let now_tz = TIME_TZ.from_utc_datetime(&now_utc.naive_utc());
            let cur_year = now_tz.year();
            let cur_month = now_tz.month();
            let mut year = START_YEAR;
            let mut month = START_MONTH;

            loop {
                if year > cur_year || (year == cur_year && month >= cur_month) {
                    break;
                }

                let year_month_str = format!("{:04}-{:02}", year, month);
                // 币安格式: BTCUSDT-1m-2020-01.zip
                let symbol = format!("{}USDT", code);
                let zip_filename = format!("{}-1m-{}.zip", symbol, year_month_str);
                let save_path = PathBuf::from(dir).join("resources").join(&zip_filename);

                let url = format!(
                    "https://data.binance.vision/data/futures/um/monthly/klines/{}/1m/{}",
                    symbol, zip_filename
                );

                if save_path.exists() {
                    month += 1;
                    if month > 12 {
                        month = 1;
                        year += 1;
                    }
                    tracing::trace!("数据已存在: {code}-{year}-{month:02}");
                    continue;
                }

                if let Some(parent) = save_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                tracing::trace!("下载数据: {code}-{year}-{month:02}");
                download_zip_file(&client, &url, &save_path).await?;

                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }

            let mut zip_paths = Vec::new();
            let mut year = START_YEAR;
            let mut month = START_MONTH;

            loop {
                if year > cur_year || (year == cur_year && month >= cur_month) {
                    break;
                }

                let symbol = format!("{}USDT", code);
                let zip_path = PathBuf::from(dir)
                    .join("resources")
                    .join(format!("{}-1m-{:04}-{:02}.zip", symbol, year, month));

                if zip_path.exists() {
                    zip_paths.push(zip_path);
                }

                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }

            for zip_path in &zip_paths {
                extract_zip_to_csv(zip_path).await?;
            }

            process_single_symbol(code, dir, &feather_path).await?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn extract_zip_to_csv(zip_path: &PathBuf) -> Result<()> {
        let csv_path = zip_path.with_extension("csv");

        if csv_path.exists() {
            return Ok(());
        }

        let zip_bytes = std::fs::read(zip_path)?;

        let cursor = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(cursor)?;

        let mut csv_data = None;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            if file.name().ends_with(".csv") {
                let mut data = Vec::new();
                std::io::copy(&mut file, &mut data)?;
                csv_data = Some(data);
                break;
            }
        }

        let csv_data = csv_data.ok_or_else(|| anyhow!("解析csv失败: {}", csv_path.display()))?;

        if let Some(parent) = csv_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&csv_path, csv_data)?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn download_zip_file(client: &Client, url: &str, save_path: &PathBuf) -> Result<()> {
        loop {
            let response = client.get(url).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(());
                    }

                    if !status.is_success() {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    let bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(_) => {
                            sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                    };

                    if std::fs::write(save_path, bytes).is_err() {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    return Ok(());
                }
                Err(_) => {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn process_single_symbol(code: &str, dir: &str, feather_path: &PathBuf) -> Result<()> {
        let now_utc = chrono::Utc::now();
        let now_tz = TIME_TZ.from_utc_datetime(&now_utc.naive_utc());
        let cur_year = now_tz.year();
        let cur_month = now_tz.month();
        let mut csv_paths = Vec::new();
        let mut year = START_YEAR;
        let mut month = START_MONTH;

        loop {
            if year > cur_year || (year == cur_year && month >= cur_month) {
                break;
            }

            let symbol = format!("{}USDT", code);
            let csv_path = PathBuf::from(dir)
                .join("resources")
                .join(format!("{}-1m-{:04}-{:02}.csv", symbol, year, month));

            if csv_path.exists() {
                csv_paths.push(csv_path);
            }

            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }

        if csv_paths.is_empty() {
            return Ok(());
        }

        // 币安 CSV 字段顺序:
        // open_time, open, high, low, close, volume, close_time, quote_volume,
        // count, taker_buy_volume, taker_buy_quote_volume, ignore
        // 注意: open_time 和 close_time 在 CSV 中是字符串格式
        let schema = Arc::new(Schema::from_iter(vec![
            Field::new("open_time".into(), DataType::String),
            Field::new("open".into(), DataType::Float64),
            Field::new("high".into(), DataType::Float64),
            Field::new("low".into(), DataType::Float64),
            Field::new("close".into(), DataType::Float64),
            Field::new("volume".into(), DataType::Float64),
            Field::new("close_time".into(), DataType::String),
            Field::new("quote_volume".into(), DataType::Float64),
            Field::new("count".into(), DataType::Int64),
            Field::new("taker_buy_volume".into(), DataType::Float64),
            Field::new("taker_buy_quote_volume".into(), DataType::Float64),
            Field::new("ignore".into(), DataType::String),
        ]));

        let df = LazyCsvReader::new_paths(
            csv_paths
                .iter()
                .map(|path| PlPath::new(path.to_string_lossy().as_ref()))
                .collect::<Vec<_>>()
                .into(),
        )
        .with_has_header(false) // 币安 CSV 无列头
        .with_schema(Some(schema))
        .with_null_values(Some(NullValues::AllColumns(vec!["".into(), "None".into()])))
        .finish()?;

        // 过滤掉可能存在的列头文本行，并将 open_time 从字符串转为 Int64
        let df = df
            .filter(col("open_time").neq(lit("open_time")))
            .with_column(col("open_time").cast(DataType::Int64));

        // 字段映射:
        // open_time -> time, volume -> size, quote_volume -> cash,
        // count -> trades, taker_buy_volume -> taker_size, taker_buy_quote_volume -> taker_cash
        let df = df
            .with_columns([
                col("open_time").alias("time"),
                col("volume").alias("size"),
                col("quote_volume").alias("cash"),
                col("count").alias("trades"),
                col("taker_buy_volume").alias("taker_size"),
                col("taker_buy_quote_volume").alias("taker_cash"),
            ])
            .select([
                col("time").cast(DataType::Datetime(
                    TimeUnit::Milliseconds,
                    Some(polars::datatypes::TimeZone::from_chrono(&TIME_TZ)),
                )),
                col("open"),
                col("high"),
                col("low"),
                col("close"),
                col("size"),
                col("cash"),
                col("trades"),
                col("taker_size"),
                col("taker_cash"),
            ]);

        let df = df.sort(["time"], SortMultipleOptions::default());

        let df = spawn_blocking(move || df.collect()).await??;

        if let Some(parent) = feather_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(feather_path)?;

        IpcWriter::new(&mut file).finish(&mut df.clone())?;

        Ok(())
    }
}
