use crate::model::Model;
use anyhow::Result;

pub trait Agent {
    fn new(model: &'static Model, sys_prompt: Option<String>, ctx_len: u32) -> Result<impl Agent>;

    fn chat<F>(&mut self, message: &str, on_token: F) -> Result<()>
    where
        F: FnMut(&str);
}
