use crate::common::*;
use polars::prelude::*;
use rhai::{Array, plugin::*};

#[export_module]
pub mod module {

    #[rhai_fn(name = "series", global, return_raw)]
    pub fn new(name: &str, dt: DataType, arr: Array) -> RTResult<Series> {
        match dt {
            DataType::Boolean => {
                let v: Vec<Option<bool>> = arr
                    .into_iter()
                    .map(|d| {
                        if d.is_unit() {
                            Ok(None)
                        } else {
                            d.as_bool().map(Some).map_err(to_rt_err)
                        }
                    })
                    .collect::<RTResult<_>>()?;
                Ok(Series::new(name.into(), v))
            }
            DataType::Int64 => {
                let v: Vec<Option<i64>> = arr
                    .into_iter()
                    .map(|d| {
                        if d.is_unit() {
                            Ok(None)
                        } else {
                            d.as_int().map(Some).map_err(to_rt_err)
                        }
                    })
                    .collect::<RTResult<_>>()?;
                Ok(Series::new(name.into(), v))
            }
            DataType::Float64 => {
                let v: Vec<Option<f64>> = arr
                    .into_iter()
                    .map(|d| {
                        if d.is_unit() {
                            Ok(None)
                        } else {
                            d.as_float().map(Some).map_err(to_rt_err)
                        }
                    })
                    .collect::<RTResult<_>>()?;
                Ok(Series::new(name.into(), v))
            }
            DataType::String => {
                let v: Vec<Option<String>> = arr
                    .into_iter()
                    .map(|d| {
                        if d.is_unit() {
                            Ok(None)
                        } else {
                            d.into_string().map(Some).map_err(to_rt_err)
                        }
                    })
                    .collect::<RTResult<_>>()?;
                Ok(Series::new(name.into(), v))
            }
            DataType::Datetime(_, _) => {
                let v: Vec<Option<i64>> = arr
                    .into_iter()
                    .map(|d| {
                        if d.is_unit() {
                            Ok(None)
                        } else {
                            d.as_int().map(Some).map_err(to_rt_err)
                        }
                    })
                    .collect::<RTResult<_>>()?;

                Ok(Series::new(name.into(), v)
                    .cast(&crate::pl::data_type::module::TIME)
                    .map_err(to_rt_err)?)
            }
            _ => Err(to_rt_err("unsupported dtype")),
        }
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(s: &mut Series) -> String {
        s.to_string()
    }

    #[rhai_fn(name = "rename", global)]
    pub fn rename(s: &mut Series, name: &str) -> Series {
        s.rename(name.into()).to_owned()
    }

    #[rhai_fn(name = "name", pure, global)]
    pub fn name(s: &mut Series) -> String {
        s.name().to_string()
    }

    #[rhai_fn(name = "len", pure, global)]
    pub fn len(s: &mut Series) -> i64 {
        s.len() as i64
    }

    #[rhai_fn(name = "dtype", pure, global)]
    pub fn dtype(s: &mut Series) -> String {
        s.dtype().to_string()
    }

    #[rhai_fn(name = "is_empty", pure, global)]
    pub fn is_empty(s: &mut Series) -> bool {
        s.is_empty()
    }

    #[rhai_fn(name = "null_count", pure, global)]
    pub fn null_count(s: &mut Series) -> i64 {
        s.null_count() as i64
    }

    #[rhai_fn(name = "sum", pure, global, return_raw)]
    pub fn sum(s: &mut Series) -> RTResult<f64> {
        s.sum::<f64>().map_err(to_rt_err)
    }

    #[rhai_fn(name = "mean", pure, global)]
    pub fn mean(s: &mut Series) -> Dynamic {
        s.mean().map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "min", pure, global, return_raw)]
    pub fn min(s: &mut Series) -> RTResult<Dynamic> {
        s.min::<f64>()
            .map(|v| v.map_or(Dynamic::UNIT, Dynamic::from))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "max", pure, global, return_raw)]
    pub fn max(s: &mut Series) -> RTResult<Dynamic> {
        s.max::<f64>()
            .map(|v| v.map_or(Dynamic::UNIT, Dynamic::from))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "std", pure, global)]
    pub fn std(s: &mut Series, ddof: i64) -> Dynamic {
        s.std(ddof as u8).map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "variance", pure, global)]
    pub fn var(s: &mut Series, ddof: i64) -> Dynamic {
        s.var(ddof as u8).map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "median", pure, global)]
    pub fn median(s: &mut Series) -> Dynamic {
        s.median().map_or(Dynamic::UNIT, Dynamic::from)
    }

    #[rhai_fn(name = "get", pure, global, return_raw)]
    pub fn get(s: &mut Series, idx: i64) -> RTResult<Dynamic> {
        match s.get(idx as usize).map_err(to_rt_err)? {
            AnyValue::Null => Ok(Dynamic::UNIT),
            AnyValue::Boolean(b) => Ok(Dynamic::from(b)),
            AnyValue::Int8(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::Int16(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::Int32(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::Int64(n) => Ok(Dynamic::from(n)),
            AnyValue::UInt8(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::UInt16(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::UInt32(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::UInt64(n) => Ok(Dynamic::from(n as i64)),
            AnyValue::Float32(n) => Ok(Dynamic::from(n as f64)),
            AnyValue::Float64(n) => Ok(Dynamic::from(n)),
            AnyValue::String(s) => Ok(Dynamic::from(s.to_string())),
            AnyValue::StringOwned(s) => Ok(Dynamic::from(s.to_string())),
            AnyValue::Datetime(n, ..) => Ok(Dynamic::from(n)),
            AnyValue::DatetimeOwned(n, ..) => Ok(Dynamic::from(n)),
            _ => Err(to_rt_err("unsupported dtype")),
        }
    }

    #[rhai_fn(name = "head", pure, global)]
    pub fn head(s: &mut Series) -> Series {
        s.head(None)
    }

    #[rhai_fn(name = "head", pure, global)]
    pub fn head_n(s: &mut Series, len: i64) -> Series {
        s.head(Some(len as usize))
    }

    #[rhai_fn(name = "tail", pure, global)]
    pub fn tail(s: &mut Series) -> Series {
        s.tail(None)
    }

    #[rhai_fn(name = "tail", pure, global)]
    pub fn tail_n(s: &mut Series, len: i64) -> Series {
        s.tail(Some(len as usize))
    }

    #[rhai_fn(name = "slice", pure, global)]
    pub fn slice(s: &mut Series, offset: i64, len: i64) -> Series {
        s.slice(offset, len as usize)
    }

    #[rhai_fn(name = "reverse", pure, global)]
    pub fn reverse(s: &mut Series) -> Series {
        s.reverse()
    }

    #[rhai_fn(name = "shift", pure, global)]
    pub fn shift(s: &mut Series, periods: i64) -> Series {
        s.shift(periods)
    }

    #[rhai_fn(name = "sort", pure, global, return_raw)]
    pub fn sort(s: &mut Series, desc: bool) -> RTResult<Series> {
        s.sort(SortOptions::default().with_order_descending(desc))
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "unique", pure, global, return_raw)]
    pub fn unique(s: &mut Series) -> RTResult<Series> {
        s.unique().map_err(to_rt_err)
    }

    #[rhai_fn(name = "n_unique", pure, global, return_raw)]
    pub fn n_unique(s: &mut Series) -> RTResult<i64> {
        s.n_unique().map(|n| n as i64).map_err(to_rt_err)
    }

    #[rhai_fn(name = "is_null", pure, global)]
    pub fn is_null(s: &mut Series) -> Series {
        s.is_null().into_series()
    }

    #[rhai_fn(name = "is_not_null", pure, global)]
    pub fn is_not_null(s: &mut Series) -> Series {
        s.is_not_null().into_series()
    }

    #[rhai_fn(name = "cast", pure, global, return_raw)]
    pub fn cast(s: &mut Series, dtype: DataType) -> RTResult<Series> {
        s.cast(&dtype).map_err(to_rt_err)
    }

    #[rhai_fn(name = "fill_null", pure, global, return_raw)]
    pub fn fill_null(s: &mut Series, strategy: FillNullStrategy) -> RTResult<Series> {
        s.fill_null(strategy).map_err(to_rt_err)
    }

    #[rhai_fn(name = "drop_nulls", pure, global)]
    pub fn drop_nulls(s: &mut Series) -> Series {
        s.drop_nulls()
    }

    #[rhai_fn(name = "+", global, return_raw)]
    pub fn series_add(a: Series, b: Series) -> RTResult<Series> {
        (a + b).map_err(to_rt_err)
    }

    #[rhai_fn(name = "-", global, return_raw)]
    pub fn series_sub(a: Series, b: Series) -> RTResult<Series> {
        (a - b).map_err(to_rt_err)
    }

    #[rhai_fn(name = "*", global, return_raw)]
    pub fn series_mul(a: Series, b: Series) -> RTResult<Series> {
        (a * b).map_err(to_rt_err)
    }

    #[rhai_fn(name = "/", global, return_raw)]
    pub fn series_div(a: Series, b: Series) -> RTResult<Series> {
        (a / b).map_err(to_rt_err)
    }

    #[rhai_fn(name = "+", global)]
    pub fn series_add_f64(a: Series, b: f64) -> Series {
        a + b
    }

    #[rhai_fn(name = "-", global)]
    pub fn series_sub_f64(a: Series, b: f64) -> Series {
        a - b
    }

    #[rhai_fn(name = "*", global)]
    pub fn series_mul_f64(a: Series, b: f64) -> Series {
        a * b
    }

    #[rhai_fn(name = "/", global)]
    pub fn series_div_f64(a: Series, b: f64) -> Series {
        a / b
    }

    #[rhai_fn(name = "+", global)]
    pub fn series_add_i64(a: Series, b: i64) -> Series {
        a + b
    }

    #[rhai_fn(name = "-", global)]
    pub fn series_sub_i64(a: Series, b: i64) -> Series {
        a - b
    }

    #[rhai_fn(name = "*", global)]
    pub fn series_mul_i64(a: Series, b: i64) -> Series {
        a * b
    }

    #[rhai_fn(name = "/", global)]
    pub fn series_div_i64(a: Series, b: i64) -> Series {
        a / b
    }
}
