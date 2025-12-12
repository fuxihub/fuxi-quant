//! fuxi-quant-agent - 智能体模块
//!
//! 提供基于 MCP 协议的智能体实现，支持工具调用。
//!
//! # 示例
//!
//! ```ignore
//! use fuxi_quant_agent::{Agent, Model, McpConfig, McpServerConfig};
//! use std::collections::HashMap;
//!
//! // 加载模型
//! let model = Model::load("path/to/model.gguf")?;
//! let model: &'static Model = Box::leak(Box::new(model));
//!
//! // 配置 MCP
//! let mcp_config = McpConfig::new().with_server(
//!     "sqlite",
//!     McpServerConfig {
//!         command: "uvx".to_string(),
//!         args: vec!["mcp-server-sqlite".to_string()],
//!         env: HashMap::new(),
//!     },
//! );
//!
//! // 创建智能体
//! let mut agent = Agent::new(model, 8192, Some(mcp_config))?;
//!
//! // 对话
//! let response = agent.chat("你好", |token| print!("{}", token))?;
//!
//! // 重置对话
//! agent.reset()?;
//! ```

mod agent;
mod mcp;
mod model;
mod tool;

pub use agent::Agent;
pub use mcp::{McpConfig, McpServerConfig};
pub use model::Model;
pub use tool::{Tool, ToolCall, ToolResult};
