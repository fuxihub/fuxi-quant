use crate::model::Model;
use anyhow::Result;
use std::sync::Arc;

pub trait Agent {
    fn new(model: Arc<Model>, sys_prompt: Option<String>, ctx_len: u32) -> impl Agent;
    fn chat<F>(&mut self, message: &str, on_token: Option<F>) -> Result<String>
    where
        F: FnMut(&str);
    fn reset(&mut self);
}
