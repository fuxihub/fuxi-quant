use polars::prelude::*;
use rhai::plugin::*;

#[export_module]
pub mod module {

    pub const MEAN: FillNullStrategy = FillNullStrategy::Mean;
    pub const MIN: FillNullStrategy = FillNullStrategy::Min;
    pub const MAX: FillNullStrategy = FillNullStrategy::Max;
    pub const ZERO: FillNullStrategy = FillNullStrategy::Zero;
    pub const ONE: FillNullStrategy = FillNullStrategy::One;

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(this: &mut FillNullStrategy) -> String {
        format!("{this:?}")
    }
}
