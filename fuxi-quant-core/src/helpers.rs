use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use polars::frame::DataFrame;
use snowflaked::sync::Generator;

pub const TIME_TZ: chrono_tz::Tz = chrono_tz::Asia::Shanghai;
pub const TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
pub const TIME_FMT_CPT: &str = "%Y%m%d_%H%M";
pub const TIME_MS_FMT: &str = "%Y-%m-%d %H:%M:%S.%3f";

static ID_GENERATOR: Generator = Generator::new(0);

#[inline]
pub fn id_new() -> String {
    format!("FUXI{:8>24X}", ID_GENERATOR.generate::<u64>())
}

#[inline]
pub fn id_is_fuxi(id: &str) -> bool {
    id.starts_with("FUXI") && id.len() == 28
}

pub fn time_from_str(s: &str) -> Result<DateTime<Tz>> {
    let s = s.trim();
    let len = s.len();

    let full_str = match len {
        4 => format!("{s}-01-01 00:00:00"),
        7 => format!("{s}-01 00:00:00"),
        10 => format!("{s} 00:00:00"),
        13 => format!("{s}:00:00"),
        16 => format!("{s}:00"),
        19 => s.to_string(),
        _ => bail!("invalid time format: {}", s),
    };

    let naive_dt = NaiveDateTime::parse_from_str(&full_str, TIME_FMT)?;

    let dt = TIME_TZ
        .from_local_datetime(&naive_dt)
        .single()
        .ok_or_else(|| anyhow!("invalid time: {}", naive_dt))?;

    Ok(dt)
}

pub fn time_from_millis(millis: i64) -> Result<DateTime<Tz>> {
    let dt_utc = DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or_else(|| anyhow!("invalid time: {}", millis))?;
    Ok(dt_utc.with_timezone(&TIME_TZ))
}

#[inline]
pub fn time_to_str(time: &DateTime<Tz>, fmt: Option<&str>) -> String {
    time.format(fmt.unwrap_or(TIME_FMT)).to_string()
}

#[inline]
pub fn time_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&TIME_TZ)
}

#[inline]
pub fn time_default() -> DateTime<Tz> {
    DateTime::<Utc>::default().with_timezone(&TIME_TZ)
}

pub fn df_time(df: &DataFrame, field: &str, idx: usize) -> Result<DateTime<Tz>> {
    let ms = df
        .column(field)?
        .i64()?
        .get(idx)
        .ok_or_else(|| anyhow!("time not found: {}", field))?;
    time_from_millis(ms)
}

#[inline]
pub fn df_f64(df: &DataFrame, col: &str, idx: usize) -> Result<f64> {
    let series = df.column(col)?;
    let val = series.get(idx)?;
    val.try_extract::<f64>()
        .map_err(|e| anyhow!("convert failed: {}", e))
}
