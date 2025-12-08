mod data_type;
mod dataframe;
mod expr;
mod expr_dt;
mod expr_str;
mod full_null;
mod join_type;
mod lazyframe;
mod lazygroup;
mod series;

use rhai::{Engine, exported_module};

pub fn register(engine: &mut Engine) {
    engine.register_static_module("DataType", exported_module!(data_type::module).into());
    engine.register_static_module("FullNull", exported_module!(full_null::module).into());
    engine.register_static_module("JoinType", exported_module!(join_type::module).into());
    engine.register_global_module(exported_module!(series::module).into());
    engine.register_global_module(exported_module!(dataframe::module).into());
    engine.register_global_module(exported_module!(lazyframe::module).into());
    engine.register_global_module(exported_module!(lazygroup::module).into());
    engine.register_global_module(exported_module!(expr::module).into());
    engine.register_global_module(exported_module!(expr_str::module).into());
    engine.register_global_module(exported_module!(expr_dt::module).into());
}
