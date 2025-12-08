use crate::common::*;
use fuxi_quant_core::types::*;
use polars::frame::DataFrame;
use rhai::{Array, Dynamic, Engine, exported_module, plugin::*};
use rust_decimal::Decimal;

pub fn register(engine: &mut Engine) {
    engine.register_global_module(exported_module!(module).into());
}

#[export_module]
pub mod module {

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn model_to_string(this: &mut Mode) -> String {
        this.to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn order_type_to_string(this: &mut OrderType) -> String {
        this.to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn direction_to_string(this: &mut Direction) -> String {
        this.to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn side_to_string(this: &mut Side) -> String {
        this.to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn order_status_to_string(this: &mut OrderStatus) -> String {
        this.to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn timer_to_string(this: &mut Timer) -> String {
        this.to_string()
    }

    // ================================================================ //
    // 常量
    // ================================================================ //

    pub const BACKTEST: Mode = Mode::Backtest;
    pub const MAINNET: Mode = Mode::Mainnet;

    pub const LIMIT: OrderType = OrderType::Limit;
    pub const MARKET: OrderType = OrderType::Market;

    pub const LONG: Direction = Direction::Long;
    pub const SHORT: Direction = Direction::Short;

    pub const BUY: Side = Side::Buy;
    pub const SELL: Side = Side::Sell;

    pub const ORD_NEW: OrderStatus = OrderStatus::New;
    pub const ORD_PENDING: OrderStatus = OrderStatus::Pending;
    pub const ORD_FILLED: OrderStatus = OrderStatus::Filled;
    pub const ORD_CANCELING: OrderStatus = OrderStatus::Canceling;
    pub const ORD_CANCELED: OrderStatus = OrderStatus::Canceled;
    pub const ORD_REJECTED: OrderStatus = OrderStatus::Rejected;

    pub const DAILY: Timer = Timer::Daily;
    pub const HOURLY: Timer = Timer::Hourly;
    pub const MINUTELY: Timer = Timer::Minutely;
    pub const SECONDLY: Timer = Timer::Secondly;

    #[rhai_fn(name = "time", pure, global)]
    pub fn api_time(engine: &mut EngineProvider) -> i64 {
        engine.get().get_time().timestamp_millis()
    }

    #[rhai_fn(name = "bars", pure, global, return_raw)]
    pub fn api_bars(engine: &mut EngineProvider, code: &str) -> RTResult<DataFrame> {
        engine.get().get_bars(code, false).map_err(to_rt_err)
    }

    #[rhai_fn(name = "bars", pure, global, return_raw)]
    pub fn api_bars_all(engine: &mut EngineProvider, code: &str, all: bool) -> RTResult<DataFrame> {
        engine.get().get_bars(code, all).map_err(to_rt_err)
    }

    #[rhai_fn(name = "signals", pure, global)]
    pub fn api_signals(engine: &mut EngineProvider) -> DataFrame {
        engine.get().get_signals()
    }

    #[rhai_fn(name = "set_signals", pure, global, return_raw)]
    pub fn api_set_signals(engine: &mut EngineProvider, signals: DataFrame) -> RTResult<()> {
        engine.get().set_signals(signals).map_err(to_rt_err)
    }

    #[rhai_fn(name = "place_order", pure, global, return_raw)]
    pub fn api_place_order(
        engine: &mut EngineProvider,
        code: &str,
        type_: OrderType,
        direction: Direction,
        side: Side,
        size: Decimal,
    ) -> RTResult<String> {
        engine
            .get()
            .place_order(code, type_, direction, side, size, None)
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "place_order", pure, global, return_raw)]
    pub fn api_place_order_2(
        engine: &mut EngineProvider,
        code: &str,
        type_: OrderType,
        direction: Direction,
        side: Side,
        size: Decimal,
        price: Decimal,
    ) -> RTResult<String> {
        engine
            .get()
            .place_order(code, type_, direction, side, size, Some(price))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "cancel_order", pure, global, return_raw)]
    pub fn api_cancel_order(engine: &mut EngineProvider, id: &str) -> RTResult<()> {
        engine.get().cancel_order(id).map_err(to_rt_err)
    }

    #[rhai_fn(name = "buy", pure, global, return_raw)]
    pub fn api_buy(engine: &mut EngineProvider, code: &str, size: Decimal) -> RTResult<String> {
        engine.get().buy(code, size, None).map_err(to_rt_err)
    }

    #[rhai_fn(name = "buy", pure, global, return_raw)]
    pub fn api_buy_2(
        engine: &mut EngineProvider,
        code: &str,
        size: Decimal,
        price: Decimal,
    ) -> RTResult<String> {
        engine.get().buy(code, size, Some(price)).map_err(to_rt_err)
    }

    #[rhai_fn(name = "sell", pure, global, return_raw)]
    pub fn api_sell(engine: &mut EngineProvider, code: &str, size: Decimal) -> RTResult<String> {
        engine.get().sell(code, size, None).map_err(to_rt_err)
    }

    #[rhai_fn(name = "sell", pure, global, return_raw)]
    pub fn api_sell_2(
        engine: &mut EngineProvider,
        code: &str,
        size: Decimal,
        price: Decimal,
    ) -> RTResult<String> {
        engine
            .get()
            .sell(code, size, Some(price))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "short", pure, global, return_raw)]
    pub fn api_short(engine: &mut EngineProvider, code: &str, size: Decimal) -> RTResult<String> {
        engine.get().short(code, size, None).map_err(to_rt_err)
    }

    #[rhai_fn(name = "short", pure, global, return_raw)]
    pub fn api_short_2(
        engine: &mut EngineProvider,
        code: &str,
        size: Decimal,
        price: Decimal,
    ) -> RTResult<String> {
        engine
            .get()
            .short(code, size, Some(price))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "cover", pure, global, return_raw)]
    pub fn api_cover(engine: &mut EngineProvider, code: &str, size: Decimal) -> RTResult<String> {
        engine.get().cover(code, size, None).map_err(to_rt_err)
    }

    #[rhai_fn(name = "cover", pure, global, return_raw)]
    pub fn api_cover_2(
        engine: &mut EngineProvider,
        code: &str,
        size: Decimal,
        price: Decimal,
    ) -> RTResult<String> {
        engine
            .get()
            .cover(code, size, Some(price))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "cash", pure, global)]
    pub fn api_cash(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().cash
    }

    #[rhai_fn(name = "order_frozen_cash", pure, global)]
    pub fn api_calc_order_frozen_cash(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_order_frozen_cash()
    }

    #[rhai_fn(name = "pos_frozen_cash", pure, global)]
    pub fn api_calc_pos_frozen_cash(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_pos_frozen_cash()
    }

    #[rhai_fn(name = "frozen_cash", pure, global)]
    pub fn api_calc_frozen_cash(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_frozen_cash()
    }

    #[rhai_fn(name = "avail_cash", pure, global)]
    pub fn api_calc_avail_cash(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_avail_cash()
    }

    #[rhai_fn(name = "upl", pure, global)]
    pub fn api_calc_upl(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_upl()
    }

    #[rhai_fn(name = "equity", pure, global)]
    pub fn api_calc_equity(engine: &mut EngineProvider) -> Decimal {
        engine.get().get_context().calc_equity()
    }

    #[rhai_fn(name = "pos_frozen_size", pure, global)]
    pub fn api_calc_pos_frozen_size(
        engine: &mut EngineProvider,
        code: &str,
        direction: Direction,
    ) -> Decimal {
        engine
            .get()
            .get_context()
            .calc_pos_frozen_size(code, direction)
    }

    #[rhai_fn(name = "pos_avail_size", pure, global)]
    pub fn api_calc_pos_avail_size(
        engine: &mut EngineProvider,
        code: &str,
        direction: Direction,
    ) -> Decimal {
        engine
            .get()
            .get_context()
            .calc_pos_avail_size(code, direction)
    }

    #[rhai_fn(name = "open_orders", pure, global)]
    pub fn api_get_open_orders(engine: &mut EngineProvider) -> Array {
        engine
            .get()
            .get_context()
            .orders
            .iter()
            .filter(|(_, order)| {
                matches!(
                    order.status,
                    OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
                ) && order.size > order.filled
            })
            .map(|(_, order)| Dynamic::from(order.clone()))
            .collect::<Vec<_>>()
    }

    #[rhai_fn(name = "open_orders", pure, global)]
    pub fn api_get_open_orders_2(engine: &mut EngineProvider, code: &str) -> Array {
        engine
            .get()
            .get_context()
            .orders
            .iter()
            .filter(|(_, order)| {
                order.code == code
                    && matches!(
                        order.status,
                        OrderStatus::New | OrderStatus::Pending | OrderStatus::Canceling
                    )
                    && order.size > order.filled
            })
            .map(|(_, order)| Dynamic::from(order.clone()))
            .collect()
    }

    #[rhai_fn(name = "order", pure, global)]
    pub fn api_get_order(engine: &mut EngineProvider, id: &str) -> Dynamic {
        engine
            .get()
            .get_context()
            .orders
            .get(id)
            .cloned()
            .map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "all_pos", pure, global)]
    pub fn api_get_all_pos(engine: &mut EngineProvider) -> Array {
        engine
            .get()
            .get_context()
            .positions
            .iter()
            .map(|(_, pos)| Dynamic::from(pos.clone()))
            .collect()
    }

    #[rhai_fn(name = "pos", pure, global)]
    pub fn api_get_pos_idx(engine: &mut EngineProvider, idx: i64) -> Dynamic {
        engine
            .get()
            .get_context()
            .positions
            .get_index(idx as usize)
            .map(|(_, pos)| pos.clone())
            .map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "pos", pure, global)]
    pub fn api_get_pos_2(engine: &mut EngineProvider, code: &str) -> Dynamic {
        engine
            .get()
            .get_context()
            .positions
            .get(code)
            .cloned()
            .map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "all_symbol", pure, global)]
    pub fn api_get_symbol(engine: &mut EngineProvider) -> Array {
        engine
            .get()
            .get_context()
            .symbols
            .iter()
            .map(|(_, s)| Dynamic::from(s.clone()))
            .collect()
    }

    #[rhai_fn(name = "symbol", pure, global)]
    pub fn api_get_symbol_idx(engine: &mut EngineProvider, idx: i64) -> Dynamic {
        engine
            .get()
            .get_context()
            .symbols
            .get_index(idx as usize)
            .map(|(_, s)| s.clone())
            .map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "symbol", pure, global)]
    pub fn api_get_symbol_2(engine: &mut EngineProvider, code: &str) -> Dynamic {
        engine
            .get()
            .get_context()
            .symbols
            .get(code)
            .cloned()
            .map_or(Dynamic::UNIT, Dynamic::from)
    }

    // ================================================================ //
    // 订单
    // ================================================================ //

    #[rhai_fn(get = "id", pure, global)]
    pub fn get_order_id(order: &mut Order) -> String {
        order.id.clone()
    }

    #[rhai_fn(get = "code", pure, global)]
    pub fn get_order_code(order: &mut Order) -> String {
        order.code.clone()
    }

    #[rhai_fn(get = "type_", pure, global)]
    pub fn get_order_type(order: &mut Order) -> OrderType {
        order.type_
    }

    #[rhai_fn(get = "direction", pure, global)]
    pub fn get_order_direction(order: &mut Order) -> Direction {
        order.direction
    }

    #[rhai_fn(get = "side", pure, global)]
    pub fn get_order_side(order: &mut Order) -> Side {
        order.side
    }

    #[rhai_fn(get = "price", pure, global)]
    pub fn get_order_price(order: &mut Order) -> Decimal {
        order.price.unwrap_or_default()
    }

    #[rhai_fn(get = "size", pure, global)]
    pub fn get_order_size(order: &mut Order) -> Decimal {
        order.size
    }

    #[rhai_fn(get = "filled", pure, global)]
    pub fn get_order_filled(order: &mut Order) -> Decimal {
        order.filled
    }

    #[rhai_fn(get = "status", pure, global)]
    pub fn get_order_status(order: &mut Order) -> OrderStatus {
        order.status
    }

    #[rhai_fn(get = "time", pure, global)]
    pub fn get_order_time(order: &mut Order) -> i64 {
        order.time.timestamp_millis()
    }

    // ================================================================ //
    // 方向持仓
    // ================================================================ //

    #[rhai_fn(get = "price", pure, global)]
    pub fn get_direction_position_price(pos: &mut DirectionPosition) -> Decimal {
        pos.price
    }

    #[rhai_fn(get = "size", pure, global)]
    pub fn get_direction_position_size(pos: &mut DirectionPosition) -> Decimal {
        pos.size
    }

    // ================================================================ //
    // 持仓
    // ================================================================ //

    #[rhai_fn(get = "code", pure, global)]
    pub fn get_position_code(pos: &mut fuxi_quant_core::types::Position) -> String {
        pos.code.clone()
    }

    #[rhai_fn(get = "lever", pure, global)]
    pub fn get_position_lever(pos: &mut fuxi_quant_core::types::Position) -> Decimal {
        pos.lever
    }

    #[rhai_fn(get = "long", pure, global)]
    pub fn get_position_long(pos: &mut fuxi_quant_core::types::Position) -> DirectionPosition {
        pos.long.clone()
    }

    #[rhai_fn(get = "short", pure, global)]
    pub fn get_position_short(pos: &mut fuxi_quant_core::types::Position) -> DirectionPosition {
        pos.short.clone()
    }

    // ================================================================ //
    // Symbol
    // ================================================================ //

    #[rhai_fn(get = "code", pure, global)]
    pub fn get_symbol_code(s: &mut fuxi_quant_core::types::Symbol) -> String {
        s.code.clone()
    }

    #[rhai_fn(get = "price_tick", pure, global)]
    pub fn get_symbol_price_tick(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.price_tick
    }

    #[rhai_fn(get = "size_tick", pure, global)]
    pub fn get_symbol_size_tick(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.size_tick
    }

    #[rhai_fn(get = "min_size", pure, global)]
    pub fn get_symbol_min_size(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.min_size
    }

    #[rhai_fn(get = "min_cash", pure, global)]
    pub fn get_symbol_min_cash(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.min_cash
    }

    #[rhai_fn(get = "max_lever", pure, global)]
    pub fn get_symbol_max_lever(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.max_lever
    }

    #[rhai_fn(get = "face_val", pure, global)]
    pub fn get_symbol_face_val(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.face_val
    }

    #[rhai_fn(get = "mark_price", pure, global)]
    pub fn get_symbol_mark_price(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.mark_price
    }

    #[rhai_fn(get = "price", pure, global)]
    pub fn get_symbol_price(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.price
    }

    #[rhai_fn(get = "funding_rate", pure, global)]
    pub fn get_symbol_funding_rate(s: &mut fuxi_quant_core::types::Symbol) -> Decimal {
        s.funding_rate
    }

    #[rhai_fn(name = "trunc_size", pure, global)]
    pub fn symbol_trunc_size(s: &mut fuxi_quant_core::types::Symbol, size: Decimal) -> Decimal {
        s.trunc_size(size)
    }

    #[rhai_fn(name = "trunc_price", pure, global)]
    pub fn symbol_trunc_price(s: &mut fuxi_quant_core::types::Symbol, price: Decimal) -> Decimal {
        s.trunc_price(price)
    }

    #[rhai_fn(name = "cash_to_size", pure, global)]
    pub fn symbol_cash_to_size(s: &mut fuxi_quant_core::types::Symbol, cash: Decimal) -> Decimal {
        s.cash_to_size(cash, None)
    }

    #[rhai_fn(name = "cash_to_size", pure, global)]
    pub fn symbol_cash_to_size_2(
        s: &mut fuxi_quant_core::types::Symbol,
        cash: Decimal,
        price: Decimal,
    ) -> Decimal {
        s.cash_to_size(cash, Some(price))
    }
}
