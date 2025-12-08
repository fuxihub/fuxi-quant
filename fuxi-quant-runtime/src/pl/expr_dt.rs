use polars::prelude::*;
use rhai::plugin::*;

#[export_module]
pub mod module {

    #[rhai_fn(name = "dt_year", global)]
    pub fn year(e: Expr) -> Expr {
        e.dt().year()
    }

    #[rhai_fn(name = "dt_month", global)]
    pub fn month(e: Expr) -> Expr {
        e.dt().month()
    }

    #[rhai_fn(name = "dt_day", global)]
    pub fn day(e: Expr) -> Expr {
        e.dt().day()
    }

    #[rhai_fn(name = "dt_hour", global)]
    pub fn hour(e: Expr) -> Expr {
        e.dt().hour()
    }

    #[rhai_fn(name = "dt_minute", global)]
    pub fn minute(e: Expr) -> Expr {
        e.dt().minute()
    }

    #[rhai_fn(name = "dt_second", global)]
    pub fn second(e: Expr) -> Expr {
        e.dt().second()
    }

    #[rhai_fn(name = "dt_millisecond", global)]
    pub fn millisecond(e: Expr) -> Expr {
        e.dt().millisecond()
    }

    #[rhai_fn(name = "dt_microsecond", global)]
    pub fn microsecond(e: Expr) -> Expr {
        e.dt().microsecond()
    }

    #[rhai_fn(name = "dt_nanosecond", global)]
    pub fn nanosecond(e: Expr) -> Expr {
        e.dt().nanosecond()
    }

    #[rhai_fn(name = "dt_weekday", global)]
    pub fn weekday(e: Expr) -> Expr {
        e.dt().weekday()
    }

    #[rhai_fn(name = "dt_week", global)]
    pub fn week(e: Expr) -> Expr {
        e.dt().week()
    }

    #[rhai_fn(name = "dt_ordinal_day", global)]
    pub fn ordinal_day(e: Expr) -> Expr {
        e.dt().ordinal_day()
    }

    #[rhai_fn(name = "dt_quarter", global)]
    pub fn quarter(e: Expr) -> Expr {
        e.dt().quarter()
    }

    #[rhai_fn(name = "dt_is_leap_year", global)]
    pub fn is_leap_year(e: Expr) -> Expr {
        e.dt().is_leap_year()
    }

    #[rhai_fn(name = "dt_timestamp", global)]
    pub fn timestamp(e: Expr, unit: &str) -> Expr {
        let tu = match unit {
            "ms" => TimeUnit::Milliseconds,
            "us" => TimeUnit::Microseconds,
            "ns" => TimeUnit::Nanoseconds,
            _ => TimeUnit::Milliseconds,
        };
        e.dt().timestamp(tu)
    }

    #[rhai_fn(name = "dt_truncate", global)]
    pub fn truncate(e: Expr, every: &str) -> Expr {
        e.dt().truncate(lit(every))
    }

    #[rhai_fn(name = "dt_round", global)]
    pub fn round(e: Expr, every: &str) -> Expr {
        e.dt().round(lit(every))
    }

    #[rhai_fn(name = "dt_date", global)]
    pub fn date(e: Expr) -> Expr {
        e.dt().date()
    }

    #[rhai_fn(name = "dt_time", global)]
    pub fn time(e: Expr) -> Expr {
        e.dt().time()
    }
}
