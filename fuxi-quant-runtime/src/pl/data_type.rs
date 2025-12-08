use polars::prelude::*;
use rhai::plugin::*;

#[export_module]
pub mod module {

    pub const NULL: DataType = DataType::Null;
    pub const BOOL: DataType = DataType::Boolean;
    pub const INT: DataType = DataType::Int64;
    pub const FLOAT: DataType = DataType::Float64;
    pub const STR: DataType = DataType::String;
    pub const TIME: DataType = DataType::Datetime(
        TimeUnit::Milliseconds,
        Some(unsafe { TimeZone::from_static("Asia/Shanghai") }),
    );

    #[rhai_fn(name = "to_string", pure, global)]
    pub fn to_string(this: &mut DataType) -> String {
        this.to_string()
    }
}
