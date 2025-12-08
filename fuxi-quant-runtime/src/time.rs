use crate::common::*;
use chrono::{Datelike, Duration, DurationRound, Timelike};
use fuxi_quant_core::{
    helpers::{TIME_MS_FMT, time_from_millis, time_from_str, time_now, time_to_str},
    types::Time,
};
use rhai::{Engine, exported_module, plugin::*};

pub fn register(engine: &mut Engine) {
    engine.register_global_module(exported_module!(module).into());
}

#[export_module]
pub mod module {

    // ================================================================ //
    //                         Duration 常量
    // ================================================================ //

    /// 1 天
    pub const DAY: Duration = Duration::days(1);

    /// 1 小时
    pub const HOUR: Duration = Duration::hours(1);

    /// 1 分钟
    pub const MINUTE: Duration = Duration::minutes(1);

    /// 1 秒
    pub const SECOND: Duration = Duration::seconds(1);

    /// 1 毫秒
    pub const MILLI: Duration = Duration::milliseconds(1);

    // ================================================================ //
    //                         Time 构造
    // ================================================================ //

    #[rhai_fn(name = "now", global)]
    pub fn now() -> Time {
        time_now()
    }

    #[rhai_fn(name = "to_time", global, return_raw)]
    pub fn ms_to_time(millis: i64) -> RTResult<Time> {
        time_from_millis(millis).map_err(to_rt_err)
    }

    #[rhai_fn(name = "to_time", global, return_raw)]
    pub fn str_to_time(s: &str) -> RTResult<Time> {
        time_from_str(s).map_err(to_rt_err)
    }

    // ================================================================ //
    //                       Time 组件获取
    // ================================================================ //

    #[rhai_fn(get = "year", pure, global)]
    pub fn year(t: &mut Time) -> i64 {
        t.year() as i64
    }

    #[rhai_fn(get = "month", pure, global)]
    pub fn month(t: &mut Time) -> i64 {
        t.month() as i64
    }

    #[rhai_fn(get = "day", pure, global)]
    pub fn day(t: &mut Time) -> i64 {
        t.day() as i64
    }

    #[rhai_fn(get = "hour", pure, global)]
    pub fn hour(t: &mut Time) -> i64 {
        t.hour() as i64
    }

    #[rhai_fn(get = "minute", pure, global)]
    pub fn minute(t: &mut Time) -> i64 {
        t.minute() as i64
    }

    #[rhai_fn(get = "second", pure, global)]
    pub fn second(t: &mut Time) -> i64 {
        t.second() as i64
    }

    /// 毫秒部分 (0-999)
    #[rhai_fn(get = "millis", pure, global)]
    pub fn millis(t: &mut Time) -> i64 {
        t.timestamp_subsec_millis() as i64
    }

    /// 一年中的第几天 (1-366)
    #[rhai_fn(get = "ordinal", pure, global)]
    pub fn ordinal(t: &mut Time) -> i64 {
        t.ordinal() as i64
    }

    /// 星期几 (0=周一, 6=周日)
    #[rhai_fn(get = "weekday", pure, global)]
    pub fn weekday(t: &mut Time) -> i64 {
        t.weekday().num_days_from_monday() as i64
    }

    /// ISO 周数
    #[rhai_fn(get = "week", pure, global)]
    pub fn week(t: &mut Time) -> i64 {
        t.iso_week().week() as i64
    }

    /// 季度 (1-4)
    #[rhai_fn(get = "quarter", pure, global)]
    pub fn quarter(t: &mut Time) -> i64 {
        ((t.month() - 1) / 3 + 1) as i64
    }

    // ================================================================ //
    //                     Time 时间戳与格式化
    // ================================================================ //

    /// 毫秒时间戳
    #[rhai_fn(get = "timestamp_ms", pure, global)]
    pub fn timestamp_millis(t: &mut Time) -> i64 {
        t.timestamp_millis()
    }

    /// 秒时间戳
    #[rhai_fn(get = "timestamp", pure, global)]
    pub fn timestamp(t: &mut Time) -> i64 {
        t.timestamp()
    }

    #[rhai_fn(name = "format", pure, global)]
    pub fn format(t: &mut Time, fmt: &str) -> String {
        t.format(fmt).to_string()
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn time_to_string(t: &mut Time) -> String {
        time_to_str(t, Some(TIME_MS_FMT))
    }

    // ================================================================ //
    //                         Time 判断
    // ================================================================ //

    /// 是否是闰年
    #[rhai_fn(name = "is_leap_year", pure, global)]
    pub fn is_leap_year(t: &mut Time) -> bool {
        t.date_naive().leap_year()
    }

    // ================================================================ //
    //                       Time 比较运算符
    // ================================================================ //

    #[rhai_fn(name = "==", pure, global)]
    pub fn time_eq(t1: &mut Time, t2: Time) -> bool {
        *t1 == t2
    }

    #[rhai_fn(name = "!=", pure, global)]
    pub fn time_ne(t1: &mut Time, t2: Time) -> bool {
        *t1 != t2
    }

    #[rhai_fn(name = "<", pure, global)]
    pub fn time_lt(t1: &mut Time, t2: Time) -> bool {
        *t1 < t2
    }

    #[rhai_fn(name = "<=", pure, global)]
    pub fn time_le(t1: &mut Time, t2: Time) -> bool {
        *t1 <= t2
    }

    #[rhai_fn(name = ">", pure, global)]
    pub fn time_gt(t1: &mut Time, t2: Time) -> bool {
        *t1 > t2
    }

    #[rhai_fn(name = ">=", pure, global)]
    pub fn time_ge(t1: &mut Time, t2: Time) -> bool {
        *t1 >= t2
    }

    // ================================================================ //
    //                         Time 运算
    // ================================================================ //

    #[rhai_fn(name = "+", pure, global)]
    pub fn time_add_duration(t: &mut Time, delta: Duration) -> Time {
        *t + delta
    }

    #[rhai_fn(name = "-", pure, global)]
    pub fn time_sub_duration(t: &mut Time, delta: Duration) -> Time {
        *t - delta
    }

    /// 两个时间之间的差值，返回 Duration
    #[rhai_fn(name = "-", pure, global)]
    pub fn time_sub_time(t1: &mut Time, t2: Time) -> Duration {
        *t1 - t2
    }

    // ================================================================ //
    //                         Time 截断
    // ================================================================ //

    /// 按 Duration 截断时间，如 t.trunc(DAY), t.trunc(HOUR)
    #[rhai_fn(name = "trunc", pure, global, return_raw)]
    pub fn time_trunc(t: &mut Time, duration: Duration) -> RTResult<Time> {
        t.duration_trunc(duration).map_err(to_rt_err)
    }

    // ================================================================ //
    //                       Duration 运算
    // ================================================================ //

    #[rhai_fn(name = "+", pure, global)]
    pub fn duration_add(d1: &mut Duration, d2: Duration) -> Duration {
        *d1 + d2
    }

    #[rhai_fn(name = "-", pure, global)]
    pub fn duration_sub(d1: &mut Duration, d2: Duration) -> Duration {
        *d1 - d2
    }

    #[rhai_fn(name = "*", pure, global)]
    pub fn duration_mul(d: &mut Duration, n: i64) -> Duration {
        *d * (n as i32)
    }

    #[rhai_fn(name = "*", global)]
    pub fn int_mul_duration(n: i64, d: Duration) -> Duration {
        d * (n as i32)
    }

    #[rhai_fn(name = "/", pure, global)]
    pub fn duration_div(d: &mut Duration, n: i64) -> Duration {
        *d / (n as i32)
    }

    #[rhai_fn(name = "-", pure, global)]
    pub fn duration_neg(d: &mut Duration) -> Duration {
        -*d
    }

    // ================================================================ //
    //                       Duration 转换
    // ================================================================ //

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn duration_to_string(d: &mut Duration) -> String {
        d.to_string()
    }

    #[rhai_fn(name = "to_days", pure, global)]
    pub fn duration_to_days(d: &mut Duration) -> i64 {
        d.num_days()
    }

    #[rhai_fn(name = "to_hours", pure, global)]
    pub fn duration_to_hours(d: &mut Duration) -> i64 {
        d.num_hours()
    }

    #[rhai_fn(name = "to_minutes", pure, global)]
    pub fn duration_to_minutes(d: &mut Duration) -> i64 {
        d.num_minutes()
    }

    #[rhai_fn(name = "to_seconds", pure, global)]
    pub fn duration_to_seconds(d: &mut Duration) -> i64 {
        d.num_seconds()
    }

    #[rhai_fn(name = "to_millis", pure, global)]
    pub fn duration_to_millis(d: &mut Duration) -> i64 {
        d.num_milliseconds()
    }

    // ================================================================ //
    //                       Duration 判断
    // ================================================================ //

    #[rhai_fn(name = "is_zero", pure, global)]
    pub fn duration_is_zero(d: &mut Duration) -> bool {
        d.is_zero()
    }
}
