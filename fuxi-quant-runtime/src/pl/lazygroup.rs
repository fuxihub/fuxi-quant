use crate::common::*;
use polars::prelude::*;
use rhai::{Array, plugin::*};

#[export_module]
pub mod module {
    use super::*;

    #[rhai_fn(name = "agg", global, return_raw)]
    pub fn agg(lgb: LazyGroupBy, exprs: Array) -> RTResult<LazyFrame> {
        let exprs = exprs
            .into_iter()
            .map(|d| d.try_cast_result::<Expr>().map_err(to_rt_err))
            .collect::<RTResult<Vec<_>>>()?;
        Ok(lgb.agg(exprs))
    }
}
