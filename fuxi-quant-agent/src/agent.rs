use crate::model::Model;
use anyhow::Result;
use serde::Serialize;

/// 流式响应事件
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamEvent {
    ThinkBegin,
    ThinkEnd,
    Token(String),
    ToolCall(String),
    ToolResult(String),
    Done,
    Error(String),
}

pub trait Agent {
    fn new(model: &'static Model, sys_prompt: Option<String>, ctx_len: u32) -> Result<impl Agent>;

    fn chat<F>(&mut self, message: &str, on_event: F) -> Result<()>
    where
        F: FnMut(StreamEvent);
}
