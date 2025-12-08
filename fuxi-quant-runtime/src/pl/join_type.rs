use polars::prelude::*;
use rhai::plugin::*;

#[export_module]
pub mod module {

    pub const INNER: JoinType = JoinType::Inner;
    pub const LEFT: JoinType = JoinType::Left;
    pub const RIGHT: JoinType = JoinType::Right;
    pub const FULL: JoinType = JoinType::Full;
    pub const SEMI: JoinType = JoinType::Semi;
    pub const ANTI: JoinType = JoinType::Anti;
    pub const IEJOIN: JoinType = JoinType::IEJoin;
    pub const CROSS: JoinType = JoinType::Cross;

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(this: &mut JoinType) -> String {
        this.to_string()
    }
}
