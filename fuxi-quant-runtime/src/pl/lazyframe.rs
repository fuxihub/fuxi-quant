use crate::common::*;
use polars::prelude::*;
use rhai::{Array, plugin::*};

#[export_module]
pub mod module {

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(lf: &mut LazyFrame) -> String {
        match lf.describe_plan() {
            Ok(r) => r,
            Err(err) => err.to_string(),
        }
    }

    #[rhai_fn(name = "collect", global, return_raw)]
    pub fn collect(lf: LazyFrame) -> RTResult<DataFrame> {
        lf.collect().map_err(to_rt_err)
    }

    #[rhai_fn(name = "select", global, return_raw)]
    pub fn select(lf: LazyFrame, exprs: Array) -> RTResult<LazyFrame> {
        let exprs = exprs
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.select(exprs))
    }

    #[rhai_fn(name = "filter", global)]
    pub fn filter(lf: LazyFrame, expr: Expr) -> LazyFrame {
        lf.filter(expr)
    }

    #[rhai_fn(name = "with_column", global)]
    pub fn with_column(lf: LazyFrame, expr: Expr) -> LazyFrame {
        lf.with_column(expr)
    }

    #[rhai_fn(name = "with_columns", global, return_raw)]
    pub fn with_columns(lf: LazyFrame, exprs: Array) -> RTResult<LazyFrame> {
        let exprs = exprs
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.with_columns(exprs))
    }

    #[rhai_fn(name = "sort", global, return_raw)]
    pub fn sort(lf: LazyFrame, by: Array, desc: Array) -> RTResult<LazyFrame> {
        let cols = by
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let descs = desc
            .into_iter()
            .map(|d| d.as_bool().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.sort(
            cols,
            SortMultipleOptions::default().with_order_descending_multi(descs),
        ))
    }

    #[rhai_fn(name = "slice", global)]
    pub fn slice(lf: LazyFrame, offset: i64, len: i64) -> LazyFrame {
        lf.slice(offset, len as u32)
    }

    #[rhai_fn(name = "limit", global)]
    pub fn limit(lf: LazyFrame, n: i64) -> LazyFrame {
        lf.limit(n as u32)
    }

    #[rhai_fn(name = "tail", global)]
    pub fn tail(lf: LazyFrame, n: i64) -> LazyFrame {
        lf.tail(n as u32)
    }

    #[rhai_fn(name = "unique", global)]
    pub fn unique(lf: LazyFrame) -> LazyFrame {
        lf.unique(None, Default::default())
    }

    #[rhai_fn(name = "drop_nulls", global)]
    pub fn drop_nulls(lf: LazyFrame) -> LazyFrame {
        lf.drop_nulls(None)
    }

    #[rhai_fn(name = "fill_null", global)]
    pub fn fill_null(lf: LazyFrame, v: Expr) -> LazyFrame {
        lf.fill_null(v)
    }

    #[rhai_fn(name = "group_by", global, return_raw)]
    pub fn group_by(lf: LazyFrame, cols: Array) -> RTResult<LazyGroupBy> {
        let exprs = cols
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.group_by(exprs))
    }

    #[rhai_fn(name = "join", global, return_raw)]
    pub fn join(
        lf: LazyFrame,
        other: LazyFrame,
        left: Array,
        right: Array,
        how: JoinType,
    ) -> RTResult<LazyFrame> {
        let l = left
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let r = right
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.join(other, l, r, JoinArgs::new(how)))
    }

    #[rhai_fn(name = "reverse", global)]
    pub fn reverse(lf: LazyFrame) -> LazyFrame {
        lf.reverse()
    }

    #[rhai_fn(name = "shift", global)]
    pub fn shift(lf: LazyFrame, periods: i64) -> LazyFrame {
        lf.shift(lit(periods))
    }

    #[rhai_fn(name = "cache", global)]
    pub fn cache(lf: LazyFrame) -> LazyFrame {
        lf.cache()
    }

    #[rhai_fn(name = "describe_plan", pure, global, return_raw)]
    pub fn describe_plan(lf: &mut LazyFrame) -> RTResult<String> {
        lf.describe_plan().map_err(to_rt_err)
    }

    #[rhai_fn(name = "describe_optimized_plan", pure, global, return_raw)]
    pub fn describe_optimized_plan(lf: &mut LazyFrame) -> RTResult<String> {
        lf.describe_optimized_plan().map_err(to_rt_err)
    }

    #[rhai_fn(name = "with_row_index", global)]
    pub fn with_row_index(lf: LazyFrame, name: &str) -> LazyFrame {
        lf.with_row_index(name, None)
    }

    #[rhai_fn(name = "with_row_index", global)]
    pub fn with_row_index_n(lf: LazyFrame, name: &str, offset: i64) -> LazyFrame {
        lf.with_row_index(name, Some(offset as IdxSize))
    }

    #[rhai_fn(name = "rename", global, return_raw)]
    pub fn rename(lf: LazyFrame, existing: Array, new: Array) -> RTResult<LazyFrame> {
        let old = existing
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let new_names = new
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lf.rename(old, new_names, true))
    }

    #[rhai_fn(name = "first", global)]
    pub fn first(lf: LazyFrame) -> LazyFrame {
        lf.first()
    }

    #[rhai_fn(name = "last", global)]
    pub fn last(lf: LazyFrame) -> LazyFrame {
        lf.last()
    }

    #[rhai_fn(name = "max", global)]
    pub fn max(lf: LazyFrame) -> LazyFrame {
        lf.max()
    }

    #[rhai_fn(name = "min", global)]
    pub fn min(lf: LazyFrame) -> LazyFrame {
        lf.min()
    }

    #[rhai_fn(name = "sum", global)]
    pub fn sum(lf: LazyFrame) -> LazyFrame {
        lf.sum()
    }

    #[rhai_fn(name = "mean", global)]
    pub fn mean(lf: LazyFrame) -> LazyFrame {
        lf.mean()
    }

    #[rhai_fn(name = "median", global)]
    pub fn median(lf: LazyFrame) -> LazyFrame {
        lf.median()
    }

    #[rhai_fn(name = "std", global)]
    pub fn std(lf: LazyFrame, ddof: i64) -> LazyFrame {
        lf.std(ddof as u8)
    }

    #[rhai_fn(name = "variance", global)]
    pub fn var(lf: LazyFrame, ddof: i64) -> LazyFrame {
        lf.var(ddof as u8)
    }

    #[rhai_fn(name = "quantile", global)]
    pub fn quantile(lf: LazyFrame, quantile: f64) -> LazyFrame {
        lf.quantile(lit(quantile), Default::default())
    }

    #[rhai_fn(name = "count", global)]
    pub fn count(lf: LazyFrame) -> LazyFrame {
        lf.count()
    }

    #[rhai_fn(name = "null_count", global)]
    pub fn null_count(lf: LazyFrame) -> LazyFrame {
        lf.null_count()
    }
}
