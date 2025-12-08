use anyhow::{Result, ensure};
use chrono::DateTime;
use chrono_tz::Tz;
use indexmap::IndexMap;
use polars::prelude::*;
use rust_decimal::prelude::*;
use strum::Display;

/// 时间
pub type Time = DateTime<Tz>;

/// Map
pub type Map<K, V> = IndexMap<K, V>;

/// 运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Mode {
    /// 回测
    Backtest,
    /// 实盘
    Mainnet,
}

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum OrderType {
    /// 限价单
    Limit,
    /// 市价单
    Market,
}

/// 交易方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Direction {
    /// 做多
    Long,
    /// 做空
    Short,
}

/// 买卖方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Side {
    /// 买入
    Buy,
    /// 卖出
    Sell,
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum OrderStatus {
    /// 新创建
    New,
    /// 挂单中
    Pending,
    /// 已成交
    Filled,
    /// 取消中
    Canceling,
    /// 已取消
    Canceled,
    /// 已拒绝
    Rejected,
}

/// 定时器
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Timer {
    /// 每日执行
    Daily,
    /// 每小时执行
    Hourly,
    /// 每分钟执行
    Minutely,
    /// 每秒执行
    Secondly,
}

/// K线
#[derive(Debug, Clone)]
pub struct Bar {
    /// 时间
    pub time: Time,
    /// 开盘价
    pub open: f64,
    /// 最高价
    pub high: f64,
    /// 最低价
    pub low: f64,
    /// 收盘价
    pub close: f64,
    /// 成交量
    pub size: f64,
    /// 成交额
    pub cash: f64,
}

/// 交易对
#[derive(Debug, Clone)]
pub struct Symbol {
    /// 交易对
    pub code: String,
    /// 最小价格变动
    pub price_tick: Decimal,
    /// 最小数量变动
    pub size_tick: Decimal,
    /// 最小交易数量
    pub min_size: Decimal,
    /// 最小交易金额
    pub min_cash: Decimal,
    /// 最大杠杆倍数
    pub max_lever: Decimal,
    /// 合约面值
    pub face_val: Decimal,
    /// 标记价格
    pub mark_price: Decimal,
    /// 最新成交价格
    pub price: Decimal,
    /// 资金费率
    pub funding_rate: Decimal,
}

impl Symbol {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_string(),
            price_tick: dec!(0.00000001),
            size_tick: dec!(0.00000001),
            min_size: dec!(0.00000001),
            min_cash: dec!(0.00000001),
            max_lever: dec!(10),
            face_val: dec!(1),
            mark_price: dec!(0),
            price: dec!(0),
            funding_rate: dec!(0),
        }
    }

    #[inline]
    pub fn trunc_size(&self, size: Decimal) -> Decimal {
        (size / self.size_tick).floor() * self.size_tick
    }

    #[inline]
    pub fn trunc_price(&self, price: Decimal) -> Decimal {
        (price / self.price_tick).floor() * self.price_tick
    }

    #[inline]
    pub fn cash_to_size(&self, cash: Decimal, price: Option<Decimal>) -> Decimal {
        self.trunc_size(cash / price.unwrap_or(self.mark_price))
    }
}

/// 交易记录(用于回测)
#[derive(Debug, Clone)]
pub struct Trade {
    /// 订单id
    pub id: String,
    /// 成交时间
    pub time: Time,
    /// 交易对
    pub code: String,
    /// 交易方向
    pub direction: Direction,
    /// 买卖方向
    pub side: Side,
    /// 成交价格
    pub price: Decimal,
    /// 成交数量
    pub size: Decimal,
    /// 手续费
    pub fee: Decimal,
    /// 已实现盈亏
    pub rpl: Decimal,
}

/// 订单
#[derive(Debug, Clone)]
pub struct Order {
    /// 订单id
    pub id: String,
    /// 交易对
    pub code: String,
    /// 订单类型
    pub type_: OrderType,
    /// 交易方向
    pub direction: Direction,
    /// 买卖方向
    pub side: Side,
    /// 订单价格
    pub price: Option<Decimal>,
    /// 订单数量
    pub size: Decimal,
    /// 已成交数量
    pub filled: Decimal,
    /// 订单状态
    pub status: OrderStatus,
    /// 创建时间
    pub time: Time,
}

/// 方向持仓
#[derive(Debug, Clone, Default)]
pub struct DirectionPosition {
    /// 持仓均价
    pub price: Decimal,
    /// 持仓数量
    pub size: Decimal,
}

/// 持仓
#[derive(Debug, Clone)]
pub struct Position {
    /// 交易对
    pub code: String,
    /// 杠杆倍数
    pub lever: Decimal,
    /// 多头持仓
    pub long: DirectionPosition,
    /// 空头持仓
    pub short: DirectionPosition,
}

impl Position {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_string(),
            lever: dec!(1),
            long: Default::default(),
            short: Default::default(),
        }
    }
}

/// 上下文
#[derive(Debug, Clone)]
pub struct Context {
    /// 资金
    pub cash: Decimal,
    /// 交易对
    pub symbols: Map<String, Symbol>,
    /// 持仓
    pub positions: Map<String, Position>,
    /// 订单
    pub orders: Map<String, Order>,
    /// K线
    pub bars: Map<String, DataFrame>,
    /// 信号
    pub signals: DataFrame,
}

impl Context {
    pub fn new(cash: Decimal, codes: &[String]) -> Result<Self> {
        let mut symbols = Map::with_capacity(codes.len());
        let mut positions = Map::with_capacity(codes.len());
        let mut bars = Map::with_capacity(codes.len());

        for code in codes {
            ensure!(!symbols.contains_key(code));
            symbols.insert(code.to_string(), Symbol::new(code));
            positions.insert(code.to_string(), Position::new(code));
            bars.insert(code.to_string(), Default::default());
        }

        Ok(Self {
            cash,
            symbols,
            positions,
            orders: Default::default(),
            bars,
            signals: Default::default(),
        })
    }

    /// 计算未成交订单冻结资金
    pub fn calc_order_frozen_cash(&self) -> Decimal {
        self.orders
            .values()
            .filter(|order| {
                let is_open = matches!(
                    (order.direction, order.side),
                    (Direction::Long, Side::Buy) | (Direction::Short, Side::Sell)
                );
                is_open
                    && matches!(
                        order.status,
                        OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
                    )
                    && order.size > order.filled
            })
            .map(|order| {
                let unfill_size = order.size - order.filled;
                let pos = self.positions.get(&order.code).unwrap();
                let price = if order.type_ == OrderType::Market {
                    self.symbols.get(&order.code).unwrap().mark_price
                } else {
                    order.price.unwrap()
                };
                (price * unfill_size) / pos.lever
            })
            .sum()
    }

    /// 计算持仓冻结资金
    pub fn calc_pos_frozen_cash(&self) -> Decimal {
        self.positions
            .iter()
            .map(|(code, pos)| {
                let symbol = self.symbols.get(code).unwrap();
                (symbol.mark_price * pos.long.size) / pos.lever
                    + (symbol.mark_price * pos.short.size) / pos.lever
            })
            .sum()
    }

    /// 计算冻结资金
    pub fn calc_frozen_cash(&self) -> Decimal {
        self.calc_order_frozen_cash() + self.calc_pos_frozen_cash()
    }

    /// 计算可用资金
    pub fn calc_avail_cash(&self) -> Decimal {
        (self.calc_equity() - self.calc_frozen_cash()).max(Decimal::ZERO)
    }

    /// 计算未实现盈亏
    pub fn calc_upl(&self) -> Decimal {
        self.positions
            .iter()
            .map(|(code, pos)| {
                let symbol = self.symbols.get(code).unwrap();
                (symbol.mark_price - pos.long.price) * pos.long.size
                    + (pos.short.price - symbol.mark_price) * pos.short.size
            })
            .sum()
    }

    /// 计算总权益
    pub fn calc_equity(&self) -> Decimal {
        self.cash + self.calc_upl()
    }

    /// 计算持仓冻结数量
    pub fn calc_pos_frozen_size(&self, code: &str, direction: Direction) -> Decimal {
        self.orders
            .values()
            .filter(|order| {
                let is_close = matches!(
                    (order.direction, order.side),
                    (Direction::Long, Side::Sell) | (Direction::Short, Side::Buy)
                );
                order.code == code
                    && is_close
                    && order.direction == direction
                    && matches!(
                        order.status,
                        OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
                    )
            })
            .map(|order| order.size - order.filled)
            .sum()
    }

    /// 获取持仓可用数量
    pub fn calc_pos_avail_size(&self, code: &str, direction: Direction) -> Decimal {
        let pos = self.positions.get(code).unwrap();
        let pos_size = match direction {
            Direction::Long => pos.long.size,
            Direction::Short => pos.short.size,
        };
        let frozen_size = self.calc_pos_frozen_size(code, direction);
        (pos_size - frozen_size).max(Decimal::ZERO)
    }
}

/// 引擎
pub trait Engine {
    /// 获取上下文
    fn get_context(&self) -> &Context;
    /// 获取当前时间
    fn get_time(&self) -> Time;
    /// 获取k线
    fn get_bars(&self, code: &str, all: bool) -> Result<DataFrame>;
    /// 获取信号
    fn get_signals(&self) -> DataFrame;
    /// 设置信号
    fn set_signals(&mut self, signals: DataFrame) -> Result<()>;
    /// 设置杠杆
    fn set_lever(&mut self, code: &str, lever: u32) -> Result<()>;
    /// 下单
    fn place_order(
        &mut self,
        code: &str,
        type_: OrderType,
        direction: Direction,
        side: Side,
        size: Decimal,
        price: Option<Decimal>,
    ) -> Result<String>;
    /// 撤单
    fn cancel_order(&mut self, id: &str) -> Result<()>;
    /// 做多开仓
    #[inline]
    fn buy(&mut self, code: &str, size: Decimal, price: Option<Decimal>) -> Result<String> {
        self.place_order(
            code,
            if price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            Direction::Long,
            Side::Buy,
            size,
            price,
        )
    }
    /// 做多平仓
    #[inline]
    fn sell(&mut self, code: &str, size: Decimal, price: Option<Decimal>) -> Result<String> {
        self.place_order(
            code,
            if price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            Direction::Long,
            Side::Sell,
            size,
            price,
        )
    }
    /// 做空开仓
    #[inline]
    fn short(&mut self, code: &str, size: Decimal, price: Option<Decimal>) -> Result<String> {
        self.place_order(
            code,
            if price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            Direction::Short,
            Side::Sell,
            size,
            price,
        )
    }
    /// 做空平仓
    #[inline]
    fn cover(&mut self, code: &str, size: Decimal, price: Option<Decimal>) -> Result<String> {
        self.place_order(
            code,
            if price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            Direction::Short,
            Side::Buy,
            size,
            price,
        )
    }
}

#[derive(Clone, Copy)]
pub struct EngineProvider(*mut dyn Engine);

impl EngineProvider {
    pub fn new(engine: &mut dyn Engine) -> Self {
        Self(engine as *mut _ as *mut _)
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get(&self) -> &mut dyn Engine {
        unsafe { &mut *self.0 }
    }
}

/// 策略
pub trait Strategy {
    /// 策略启动
    fn on_start(&mut self, engine: &mut dyn Engine) -> Result<()>;
    /// 策略停止
    fn on_stop(&mut self, engine: &mut dyn Engine) -> Result<()>;
    /// K线更新
    fn on_bar(&mut self, engine: &mut dyn Engine, code: &str) -> Result<()>;
    /// 信号更新
    fn on_signal(&mut self, engine: &mut dyn Engine) -> Result<()>;
    /// 定时器
    fn on_timer(&mut self, engine: &mut dyn Engine, timer: Timer, time: Time) -> Result<()>;
    /// 订单更新
    fn on_order(&mut self, engine: &mut dyn Engine, order_id: &str) -> Result<()>;
    /// 持仓更新
    fn on_position(&mut self, engine: &mut dyn Engine, code: &str) -> Result<()>;
}

/// 一年的天数（考虑闰年，更准确）
const DAYS_PER_YEAR: f64 = 365.25;
/// 一年的分钟数（用于年化波动率计算）
const MINUTES_PER_YEAR: f64 = DAYS_PER_YEAR * 24.0 * 60.0;

/// 回测报告
#[derive(Debug, Clone)]
pub struct Report {
    /// 策略收益率
    pub ret: f64,
    /// 策略年化收益率
    pub ar: f64,

    /// 最大回撤
    pub mdd: f64,
    /// 年化波动率
    pub vol: f64,

    /// 夏普比率
    pub sr: f64,
    /// 索提诺比率
    pub sor: f64,
    /// 卡玛比率
    pub cr: f64,

    /// 胜率
    pub win_rate: f64,
    /// 盈亏比
    pub pl_ratio: f64,
    /// 交易次数
    pub trade_cnt: usize,

    /// 总手续费
    pub fee: f64,
}

impl Report {
    /// 生成回测报告
    pub fn new(
        init_eq: f64,
        final_eq: f64,
        hist_eq: &[f64],
        start: Time,
        end: Time,
        trades: &[Trade],
    ) -> Result<Self> {
        let fee = trades
            .iter()
            .map(|trade| trade.fee)
            .sum::<Decimal>()
            .to_f64()
            .unwrap_or(0.0);

        let days = (end - start).num_days() as f64;
        let days = days.max(1.0);

        let ret = if init_eq > 0.0 {
            (final_eq - init_eq) / init_eq
        } else {
            0.0
        };

        let ar = if days > 0.0 && init_eq > 0.0 && final_eq > 0.0 {
            ((final_eq / init_eq).powf(DAYS_PER_YEAR / days) - 1.0).max(-1.0)
        } else if init_eq > 0.0 && final_eq <= 0.0 {
            -1.0 // 权益为负，年化收益率为 -100%
        } else {
            0.0
        };

        let (mdd, vol, sor) = if hist_eq.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let eq_series = Series::new("equity".into(), hist_eq.to_vec());
            let eq_df = DataFrame::new(vec![eq_series.into()])?;
            let eq_height = eq_df.height();

            if eq_height < 2 {
                (0.0, 0.0, 0.0)
            } else {
                let metrics_df = eq_df
                    .lazy()
                    .with_columns([
                        ((col("equity") - col("equity").shift(lit(1)))
                            / col("equity").shift(lit(1)))
                        .fill_null(0.0)
                        .alias("returns"),
                        col("equity").cum_max(false).alias("peak"),
                    ])
                    .with_columns([((col("peak") - col("equity")) / col("peak")).alias("drawdown")])
                    .collect()?;

                let stats_df = metrics_df
                    .lazy()
                    .select([
                        col("drawdown").max().alias("mdd"),
                        col("returns").std(1).alias("returns_std"),
                        col("returns")
                            .filter(col("returns").lt(0.0))
                            .std(1)
                            .alias("downside_std"),
                    ])
                    .collect()?;

                let mdd = stats_df
                    .column("mdd")?
                    .f64()?
                    .get(0)
                    .unwrap_or(0.0)
                    .max(0.0);

                let ret_std = stats_df.column("returns_std")?.f64()?.get(0).unwrap_or(0.0);

                let down_std = stats_df
                    .column("downside_std")?
                    .f64()?
                    .get(0)
                    .unwrap_or(0.0);

                let vol = (ret_std * MINUTES_PER_YEAR.sqrt()).max(0.0);

                let ann_down_vol = (down_std * MINUTES_PER_YEAR.sqrt()).max(0.0);

                let sor = if ann_down_vol > 0.0 {
                    ar / ann_down_vol
                } else {
                    0.0
                };

                (mdd, vol, sor)
            }
        };

        let sr = if vol > 0.0 { ar / vol } else { 0.0 };
        let cr = if mdd > 0.0 { ar / mdd } else { 0.0 };

        let (win_rate, pl_ratio, trade_cnt) = if trades.is_empty() {
            (0.0, 0.0, 0)
        } else {
            let rpl_vals: Vec<f64> = trades
                .iter()
                .map(|t| t.rpl.to_f64().unwrap_or(0.0))
                .collect();

            let rpl_ser = Series::new("rpl".into(), rpl_vals);
            let trades_df = DataFrame::new(vec![rpl_ser.into()])?;

            let stats_df = trades_df
                .lazy()
                .filter(col("rpl").neq(0.0))
                .select([
                    col("rpl").count().alias("cnt"),
                    col("rpl")
                        .filter(col("rpl").gt(0.0))
                        .count()
                        .alias("win_cnt"),
                    col("rpl")
                        .filter(col("rpl").gt(0.0))
                        .mean()
                        .alias("avg_profit"),
                    col("rpl")
                        .filter(col("rpl").lt(0.0))
                        .mean()
                        .alias("avg_loss"),
                ])
                .collect()?;

            let trade_cnt = stats_df.column("cnt")?.u32()?.get(0).unwrap_or(0);

            if trade_cnt == 0 {
                (0.0, 0.0, 0)
            } else {
                let win_cnt = stats_df.column("win_cnt")?.u32()?.get(0).unwrap_or(0) as f64;

                let win_rate = win_cnt / trade_cnt as f64;

                let avg_win = stats_df.column("avg_profit")?.f64()?.get(0).unwrap_or(0.0);

                let avg_loss = stats_df
                    .column("avg_loss")?
                    .f64()?
                    .get(0)
                    .unwrap_or(0.0)
                    .abs();

                let pl_ratio = if avg_loss > 0.0 {
                    avg_win / avg_loss
                } else {
                    0.0
                };

                (win_rate, pl_ratio, trade_cnt as usize)
            }
        };

        Ok(Self {
            ret,
            ar,
            mdd,
            vol,
            sr,
            sor,
            cr,
            win_rate,
            pl_ratio,
            trade_cnt,
            fee,
        })
    }
}
