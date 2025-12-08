use crate::common::*;
use polars::prelude::*;
use rhai::{Array, plugin::*};

#[export_module]
pub mod module {

    #[rhai_fn(name = "dataframe", global, return_raw)]
    pub fn new(series: Array) -> RTResult<DataFrame> {
        let cols = series
            .into_iter()
            .map(|s| {
                s.try_cast_result::<Series>()
                    .map(|s| s.into())
                    .map_err(to_rt_err)
            })
            .collect::<RTResult<Vec<_>>>()?;
        DataFrame::new(cols).map_err(to_rt_err)
    }

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(df: &mut DataFrame) -> String {
        df.to_string()
    }

    #[rhai_fn(name = "height", pure, global)]
    pub fn height(df: &mut DataFrame) -> i64 {
        df.height() as i64
    }

    #[rhai_fn(name = "width", pure, global)]
    pub fn width(df: &mut DataFrame) -> i64 {
        df.width() as i64
    }

    #[rhai_fn(name = "shape", pure, global)]
    pub fn shape(df: &mut DataFrame) -> Array {
        let shape = df.shape();
        vec![Dynamic::from(shape.0 as i64), Dynamic::from(shape.1 as i64)]
    }

    #[rhai_fn(name = "columns", pure, global)]
    pub fn columns(df: &mut DataFrame) -> Array {
        df.get_column_names_str()
            .iter()
            .map(|s| Dynamic::from(s.to_string()))
            .collect()
    }

    #[rhai_fn(name = "dtypes", pure, global)]
    pub fn dtypes(df: &mut DataFrame) -> Array {
        df.dtypes()
            .iter()
            .map(|dt| Dynamic::from(dt.to_owned()))
            .collect()
    }

    #[rhai_fn(name = "is_empty", pure, global)]
    pub fn is_empty(df: &mut DataFrame) -> bool {
        df.is_empty()
    }

    #[rhai_fn(name = "column", pure, global, return_raw)]
    pub fn column(df: &mut DataFrame, name: &str) -> RTResult<Series> {
        df.column(name)
            .map(|c| c.as_materialized_series().to_owned())
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "select", pure, global, return_raw)]
    pub fn select(df: &mut DataFrame, cols: Array) -> RTResult<DataFrame> {
        let names = cols
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        df.select(&names).map_err(to_rt_err)
    }

    #[rhai_fn(name = "head", pure, global)]
    pub fn head(df: &mut DataFrame) -> DataFrame {
        df.head(None)
    }

    #[rhai_fn(name = "head", pure, global)]
    pub fn head_n(df: &mut DataFrame, n: i64) -> DataFrame {
        df.head(Some(n as usize))
    }

    #[rhai_fn(name = "tail", pure, global)]
    pub fn tail(df: &mut DataFrame) -> DataFrame {
        df.tail(None)
    }

    #[rhai_fn(name = "tail", pure, global)]
    pub fn tail_n(df: &mut DataFrame, n: i64) -> DataFrame {
        df.tail(Some(n as usize))
    }

    #[rhai_fn(name = "slice", pure, global)]
    pub fn slice(df: &mut DataFrame, offset: i64, len: i64) -> DataFrame {
        df.slice(offset, len as usize)
    }

    #[rhai_fn(name = "with_column", global, return_raw)]
    pub fn with_column(df: &mut DataFrame, s: Series) -> RTResult<DataFrame> {
        df.with_column(s).map_err(to_rt_err)?;
        Ok(df.clone())
    }

    #[rhai_fn(name = "drop", pure, global, return_raw)]
    pub fn drop(df: &mut DataFrame, name: &str) -> RTResult<DataFrame> {
        df.drop(name).map_err(to_rt_err)
    }

    #[rhai_fn(name = "rename", global, return_raw)]
    pub fn rename(df: &mut DataFrame, old: &str, new: &str) -> RTResult<DataFrame> {
        df.rename(old, new.into()).map_err(to_rt_err)?;
        Ok(df.clone())
    }

    #[rhai_fn(name = "sort", pure, global, return_raw)]
    pub fn sort(df: &mut DataFrame, by: Array, desc: Array) -> RTResult<DataFrame> {
        let cols = by
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let descs = desc
            .into_iter()
            .map(|d| d.as_bool().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        df.sort(
            cols,
            SortMultipleOptions::default().with_order_descending_multi(descs),
        )
        .map_err(to_rt_err)
    }

    #[rhai_fn(name = "filter", pure, global, return_raw)]
    pub fn filter(df: &mut DataFrame, mask: Series) -> RTResult<DataFrame> {
        df.filter(mask.bool().map_err(to_rt_err)?)
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "vstack", pure, global, return_raw)]
    pub fn vstack(df: &mut DataFrame, other: DataFrame) -> RTResult<DataFrame> {
        df.vstack(&other).map_err(to_rt_err)
    }

    #[rhai_fn(name = "join", pure, global, return_raw)]
    pub fn join(
        df: &mut DataFrame,
        other: DataFrame,
        left: Array,
        right: Array,
        how: JoinType,
    ) -> RTResult<DataFrame> {
        let l = left
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let r = right
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        df.join(&other, l, r, JoinArgs::new(how), None)
            .map_err(to_rt_err)
    }

    #[rhai_fn(name = "reverse", pure, global)]
    pub fn reverse(df: &mut DataFrame) -> DataFrame {
        df.reverse()
    }

    #[rhai_fn(name = "shift", pure, global)]
    pub fn shift(df: &mut DataFrame, periods: i64) -> DataFrame {
        df.shift(periods)
    }

    #[rhai_fn(name = "null_count", pure, global)]
    pub fn null_count(df: &mut DataFrame) -> DataFrame {
        df.null_count()
    }

    #[rhai_fn(name = "drop_nulls", pure, global, return_raw)]
    pub fn drop_nulls(df: &mut DataFrame) -> RTResult<DataFrame> {
        df.drop_nulls::<String>(None).map_err(to_rt_err)
    }

    #[rhai_fn(name = "lazy", global)]
    pub fn lazy(df: DataFrame) -> LazyFrame {
        df.lazy()
    }
}
