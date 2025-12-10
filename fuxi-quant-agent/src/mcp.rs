//! MCP (Model Context Protocol) 服务器实现
//!
//! 将量化交易功能暴露为 MCP 工具，供 AI 客户端调用。
//!
//! TODO: 完善 rmcp 集成，当前版本提供基础框架

/// MCP 服务器配置
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "fuxi-quant".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "量化交易 MCP 服务器".to_string(),
        }
    }
}

/// 量化交易 MCP 服务器（占位实现）
///
/// 完整的 MCP 服务器实现需要根据 rmcp 版本调整 API
pub struct QuantMcpServer {
    config: McpServerConfig,
}

impl QuantMcpServer {
    pub fn new() -> Self {
        Self {
            config: McpServerConfig::default(),
        }
    }

    pub fn with_config(config: McpServerConfig) -> Self {
        Self { config }
    }

    /// 获取服务器信息
    pub fn info(&self) -> &McpServerConfig {
        &self.config
    }
}

impl Default for QuantMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let server = QuantMcpServer::new();
        assert_eq!(server.info().name, "fuxi-quant");
    }
}
