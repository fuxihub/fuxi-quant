use crate::model::Model;
use anyhow::Result;

pub trait Agent: Send + Sync {
    fn new(sys_prompt: Option<String>, ctx_len: u32) -> impl Agent;
    fn chat<F>(&self, model: &Model, message: &str, on_token: Option<F>) -> Result<String>
    where
        F: FnMut(&str);
}
