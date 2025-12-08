use crate::common::*;
use polars::prelude::*;
use rhai::{Array, plugin::*};

#[export_module]
pub mod module {

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(e: &mut Expr) -> String {
        e.to_string()
    }

    #[rhai_fn(name = "alias", global)]
    pub fn alias(e: Expr, name: &str) -> Expr {
        e.alias(name)
    }

    // --- Comparison ---

    #[rhai_fn(name = "eq", global)]
    pub fn eq(e: Expr, other: Expr) -> Expr {
        e.eq(other)
    }

    #[rhai_fn(name = "neq", global)]
    pub fn neq(e: Expr, other: Expr) -> Expr {
        e.neq(other)
    }

    #[rhai_fn(name = "gt", global)]
    pub fn gt(e: Expr, other: Expr) -> Expr {
        e.gt(other)
    }

    #[rhai_fn(name = "gte", global)]
    pub fn gte(e: Expr, other: Expr) -> Expr {
        e.gt_eq(other)
    }

    #[rhai_fn(name = "lt", global)]
    pub fn lt(e: Expr, other: Expr) -> Expr {
        e.lt(other)
    }

    #[rhai_fn(name = "lte", global)]
    pub fn lte(e: Expr, other: Expr) -> Expr {
        e.lt_eq(other)
    }

    // --- Logical ---

    #[rhai_fn(name = "and", global)]
    pub fn and(e: Expr, other: Expr) -> Expr {
        e.and(other)
    }

    #[rhai_fn(name = "or", global)]
    pub fn or(e: Expr, other: Expr) -> Expr {
        e.or(other)
    }

    #[rhai_fn(name = "not", global)]
    pub fn not(e: Expr) -> Expr {
        e.not()
    }

    // --- Arithmetic ---

    #[rhai_fn(name = "+", global)]
    pub fn add(e: Expr, other: Expr) -> Expr {
        e + other
    }

    #[rhai_fn(name = "-", global)]
    pub fn sub(e: Expr, other: Expr) -> Expr {
        e - other
    }

    #[rhai_fn(name = "*", global)]
    pub fn mul(e: Expr, other: Expr) -> Expr {
        e * other
    }

    #[rhai_fn(name = "/", global)]
    pub fn div(e: Expr, other: Expr) -> Expr {
        e / other
    }

    #[rhai_fn(name = "%", global)]
    pub fn modulo(e: Expr, other: Expr) -> Expr {
        e % other
    }

    #[rhai_fn(name = "-", global)]
    pub fn neg(e: Expr) -> Expr {
        -e
    }

    #[rhai_fn(name = "abs", global)]
    pub fn abs(e: Expr) -> Expr {
        e.abs()
    }

    // --- Math ---

    #[rhai_fn(name = "sqrt", global)]
    pub fn sqrt(e: Expr) -> Expr {
        e.sqrt()
    }

    #[rhai_fn(name = "pow", global)]
    pub fn pow(e: Expr, exp: f64) -> Expr {
        e.pow(exp)
    }

    #[rhai_fn(name = "floor", global)]
    pub fn floor(e: Expr) -> Expr {
        e.floor()
    }

    #[rhai_fn(name = "ceil", global)]
    pub fn ceil(e: Expr) -> Expr {
        e.ceil()
    }

    #[rhai_fn(name = "round", global)]
    pub fn round(e: Expr, decimals: i64) -> Expr {
        e.round(decimals as u32, Default::default())
    }

    #[rhai_fn(name = "clip", global)]
    pub fn clip(e: Expr, min: f64, max: f64) -> Expr {
        e.clip(lit(min), lit(max))
    }

    // --- Sort ---

    #[rhai_fn(name = "sort", global)]
    pub fn sort(e: Expr, desc: bool) -> Expr {
        e.sort(SortOptions::default().with_order_descending(desc))
    }

    #[rhai_fn(name = "reverse", global)]
    pub fn reverse(e: Expr) -> Expr {
        e.reverse()
    }

    #[rhai_fn(name = "arg_sort", global)]
    pub fn arg_sort(e: Expr, desc: bool) -> Expr {
        e.arg_sort(desc, false)
    }

    // --- Slice ---

    #[rhai_fn(name = "head", global)]
    pub fn head(e: Expr, n: i64) -> Expr {
        e.head(Some(n as usize))
    }

    #[rhai_fn(name = "tail", global)]
    pub fn tail(e: Expr, n: i64) -> Expr {
        e.tail(Some(n as usize))
    }

    #[rhai_fn(name = "slice", global)]
    pub fn slice(e: Expr, offset: i64, len: i64) -> Expr {
        e.slice(lit(offset), lit(len))
    }

    // --- Shift ---

    #[rhai_fn(name = "shift", global)]
    pub fn shift(e: Expr, periods: i64) -> Expr {
        e.shift(lit(periods))
    }

    #[rhai_fn(name = "shift_and_fill", global)]
    pub fn shift_and_fill(e: Expr, periods: i64, fill: f64) -> Expr {
        e.shift_and_fill(lit(periods), lit(fill))
    }

    #[rhai_fn(name = "diff", global)]
    pub fn diff(e: Expr, n: i64) -> Expr {
        e.diff(lit(n), Default::default())
    }

    // --- Cast ---

    #[rhai_fn(name = "cast", global)]
    pub fn cast(e: Expr, dt: DataType) -> Expr {
        e.cast(dt)
    }

    // --- Aggregation ---

    #[rhai_fn(name = "sum", global)]
    pub fn sum(e: Expr) -> Expr {
        e.sum()
    }

    #[rhai_fn(name = "mean", global)]
    pub fn mean(e: Expr) -> Expr {
        e.mean()
    }

    #[rhai_fn(name = "min", global)]
    pub fn min(e: Expr) -> Expr {
        e.min()
    }

    #[rhai_fn(name = "max", global)]
    pub fn max(e: Expr) -> Expr {
        e.max()
    }

    #[rhai_fn(name = "median", global)]
    pub fn median(e: Expr) -> Expr {
        e.median()
    }

    #[rhai_fn(name = "std", global)]
    pub fn std(e: Expr, ddof: i64) -> Expr {
        e.std(ddof as u8)
    }

    #[rhai_fn(name = "variance", global)]
    pub fn var(e: Expr, ddof: i64) -> Expr {
        e.var(ddof as u8)
    }

    #[rhai_fn(name = "count", global)]
    pub fn count(e: Expr) -> Expr {
        e.count()
    }

    #[rhai_fn(name = "n_unique", global)]
    pub fn n_unique(e: Expr) -> Expr {
        e.n_unique()
    }

    #[rhai_fn(name = "first", global)]
    pub fn first(e: Expr) -> Expr {
        e.first()
    }

    #[rhai_fn(name = "last", global)]
    pub fn last(e: Expr) -> Expr {
        e.last()
    }

    #[rhai_fn(name = "quantile", global)]
    pub fn quantile(e: Expr, q: f64) -> Expr {
        e.quantile(lit(q), Default::default())
    }

    #[rhai_fn(name = "arg_min", global)]
    pub fn arg_min(e: Expr) -> Expr {
        e.arg_min()
    }

    #[rhai_fn(name = "arg_max", global)]
    pub fn arg_max(e: Expr) -> Expr {
        e.arg_max()
    }

    #[rhai_fn(name = "product", global)]
    pub fn product(e: Expr) -> Expr {
        e.product()
    }

    // --- Cumulative ---

    #[rhai_fn(name = "cum_sum", global)]
    pub fn cum_sum(e: Expr, reverse: bool) -> Expr {
        e.cum_sum(reverse)
    }

    #[rhai_fn(name = "cum_prod", global)]
    pub fn cum_prod(e: Expr, reverse: bool) -> Expr {
        e.cum_prod(reverse)
    }

    #[rhai_fn(name = "cum_min", global)]
    pub fn cum_min(e: Expr, reverse: bool) -> Expr {
        e.cum_min(reverse)
    }

    #[rhai_fn(name = "cum_max", global)]
    pub fn cum_max(e: Expr, reverse: bool) -> Expr {
        e.cum_max(reverse)
    }

    #[rhai_fn(name = "cum_count", global)]
    pub fn cum_count(e: Expr, reverse: bool) -> Expr {
        e.cum_count(reverse)
    }

    // --- Rolling ---

    #[rhai_fn(name = "rolling_sum", global)]
    pub fn rolling_sum(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_sum(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_mean", global)]
    pub fn rolling_mean(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_mean(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_std", global)]
    pub fn rolling_std(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_std(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_var", global)]
    pub fn rolling_var(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_var(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_min", global)]
    pub fn rolling_min(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_min(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_max", global)]
    pub fn rolling_max(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_max(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    #[rhai_fn(name = "rolling_median", global)]
    pub fn rolling_median(e: Expr, window: i64, min_periods: i64) -> Expr {
        e.rolling_median(RollingOptionsFixedWindow {
            window_size: window as usize,
            min_periods: min_periods as usize,
            ..Default::default()
        })
    }

    // --- Null handling ---

    #[rhai_fn(name = "is_null", global)]
    pub fn is_null(e: Expr) -> Expr {
        e.is_null()
    }

    #[rhai_fn(name = "is_not_null", global)]
    pub fn is_not_null(e: Expr) -> Expr {
        e.is_not_null()
    }

    #[rhai_fn(name = "is_nan", global)]
    pub fn is_nan(e: Expr) -> Expr {
        e.is_nan()
    }

    #[rhai_fn(name = "is_not_nan", global)]
    pub fn is_not_nan(e: Expr) -> Expr {
        e.is_not_nan()
    }

    #[rhai_fn(name = "is_finite", global)]
    pub fn is_finite(e: Expr) -> Expr {
        e.is_finite()
    }

    #[rhai_fn(name = "fill_null", global)]
    pub fn fill_null(e: Expr, v: Expr) -> Expr {
        e.fill_null(v)
    }

    #[rhai_fn(name = "fill_nan", global)]
    pub fn fill_nan(e: Expr, v: Expr) -> Expr {
        e.fill_nan(v)
    }

    #[rhai_fn(name = "drop_nulls", global)]
    pub fn drop_nulls(e: Expr) -> Expr {
        e.drop_nulls()
    }

    #[rhai_fn(name = "drop_nans", global)]
    pub fn drop_nans(e: Expr) -> Expr {
        e.drop_nans()
    }

    // --- Advanced ---

    #[rhai_fn(name = "rank", global)]
    pub fn rank(e: Expr, method: &str, desc: bool) -> Expr {
        let method = match method {
            "average" => RankMethod::Average,
            "min" => RankMethod::Min,
            "max" => RankMethod::Max,
            "dense" => RankMethod::Dense,
            "ordinal" => RankMethod::Ordinal,
            "random" => RankMethod::Random,
            _ => RankMethod::Ordinal,
        };
        e.rank(
            RankOptions {
                method,
                descending: desc,
            },
            None,
        )
    }

    #[rhai_fn(name = "over", global, return_raw)]
    pub fn over(e: Expr, partition_by: Array) -> RTResult<Expr> {
        let cols = partition_by
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(e.over(cols))
    }

    #[rhai_fn(name = "unique", global)]
    pub fn unique(e: Expr) -> Expr {
        e.unique()
    }

    #[rhai_fn(name = "unique_stable", global)]
    pub fn unique_stable(e: Expr) -> Expr {
        e.unique_stable()
    }

    #[rhai_fn(name = "is_first_distinct", global)]
    pub fn is_first_distinct(e: Expr) -> Expr {
        e.is_first_distinct()
    }

    #[rhai_fn(name = "is_last_distinct", global)]
    pub fn is_last_distinct(e: Expr) -> Expr {
        e.is_last_distinct()
    }

    #[rhai_fn(name = "is_in", global)]
    pub fn is_in(e: Expr, other: Expr) -> Expr {
        e.is_in(other, false)
    }

    #[rhai_fn(name = "is_between", global)]
    pub fn is_between(e: Expr, lower: Expr, upper: Expr, closed: &str) -> Expr {
        let closed = match closed {
            "left" => ClosedInterval::Left,
            "right" => ClosedInterval::Right,
            "both" => ClosedInterval::Both,
            "none" => ClosedInterval::None,
            _ => ClosedInterval::Both,
        };
        e.is_between(lower, upper, closed)
    }

    #[rhai_fn(name = "implode", global)]
    pub fn implode(e: Expr) -> Expr {
        e.implode()
    }

    #[rhai_fn(name = "explode", global)]
    pub fn explode(e: Expr) -> Expr {
        e.explode()
    }

    #[rhai_fn(name = "flatten", global)]
    pub fn flatten(e: Expr) -> Expr {
        e.flatten()
    }

    #[rhai_fn(name = "replace", global)]
    pub fn replace(e: Expr, old: Expr, new: Expr) -> Expr {
        e.replace(old, new)
    }

    #[rhai_fn(name = "sort_by", global, return_raw)]
    pub fn sort_by(e: Expr, by: Array, desc: Array) -> RTResult<Expr> {
        let by_exprs = by
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        let descs = desc
            .into_iter()
            .map(|d| d.as_bool().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(e.sort_by(
            by_exprs,
            SortMultipleOptions::default().with_order_descending_multi(descs),
        ))
    }

    #[rhai_fn(name = "interpolate", global)]
    pub fn interpolate(e: Expr, method: &str) -> Expr {
        let method = match method {
            "linear" => InterpolationMethod::Linear,
            "nearest" => InterpolationMethod::Nearest,
            _ => InterpolationMethod::Linear,
        };
        e.interpolate(method)
    }

    #[rhai_fn(name = "lower_bound", global)]
    pub fn lower_bound(e: Expr) -> Expr {
        e.lower_bound()
    }

    #[rhai_fn(name = "upper_bound", global)]
    pub fn upper_bound(e: Expr) -> Expr {
        e.upper_bound()
    }

    #[rhai_fn(name = "null_count", global)]
    pub fn null_count(e: Expr) -> Expr {
        e.null_count()
    }

    #[rhai_fn(name = "len", global)]
    pub fn len(e: Expr) -> Expr {
        e.len()
    }

    #[rhai_fn(name = "exclude", global, return_raw)]
    pub fn exclude(e: Expr, columns: Array) -> RTResult<Expr> {
        let names = columns
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        match e {
            Expr::Selector(s) => Ok(Expr::Selector(s.exclude_cols(names))),
            _ => Ok(Expr::Selector(all().exclude_cols(names))),
        }
    }

    // ================================================================ //
    // When/Then/ChainedWhen/ChainedThen
    // ================================================================ //

    #[rhai_fn(name = "then", global)]
    pub fn when_then(w: When, expr: Expr) -> Then {
        w.then(expr)
    }

    #[rhai_fn(name = "when", global)]
    pub fn then_when(t: Then, cond: Expr) -> ChainedWhen {
        t.when(cond)
    }

    #[rhai_fn(name = "otherwise", global)]
    pub fn then_otherwise(t: Then, expr: Expr) -> Expr {
        t.otherwise(expr)
    }

    #[rhai_fn(name = "then", global)]
    pub fn chained_when_then(cw: ChainedWhen, expr: Expr) -> ChainedThen {
        cw.then(expr)
    }

    #[rhai_fn(name = "when", global)]
    pub fn chained_then_when(ct: ChainedThen, cond: Expr) -> ChainedWhen {
        ct.when(cond)
    }

    #[rhai_fn(name = "otherwise", global)]
    pub fn chained_then_otherwise(ct: ChainedThen, expr: Expr) -> Expr {
        ct.otherwise(expr)
    }

    // ================================================================ //
    // Global Functions
    // ================================================================ //

    #[rhai_fn(name = "col", global)]
    pub fn global_col(name: &str) -> Expr {
        col(name)
    }

    #[rhai_fn(name = "lit", global)]
    pub fn global_lit_f64(v: f64) -> Expr {
        lit(v)
    }

    #[rhai_fn(name = "lit", global)]
    pub fn global_lit_i64(v: i64) -> Expr {
        lit(v)
    }

    #[rhai_fn(name = "lit", global)]
    pub fn global_lit_str(v: &str) -> Expr {
        lit(v)
    }

    #[rhai_fn(name = "lit", global)]
    pub fn global_lit_bool(v: bool) -> Expr {
        lit(v)
    }

    #[rhai_fn(name = "when", global)]
    pub fn global_when(cond: Expr) -> When {
        when(cond)
    }

    #[rhai_fn(name = "all", global)]
    pub fn global_all() -> Expr {
        all().as_expr()
    }

    #[rhai_fn(name = "cols", global, return_raw)]
    pub fn global_cols(names: Array) -> RTResult<Expr> {
        let names = names
            .into_iter()
            .map(|d| d.into_string().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(Expr::Selector(cols(names)))
    }

    #[rhai_fn(name = "concat_lazyframe", global, return_raw)]
    pub fn global_concat_lf(frames: Array) -> RTResult<LazyFrame> {
        let lfs = frames
            .into_iter()
            .map(|d| d.try_cast_result::<LazyFrame>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        concat(lfs, UnionArgs::default()).map_err(to_rt_err)
    }
}
