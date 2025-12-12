//! MCP (Model Context Protocol) 客户端模块
//!
//! 支持连接 MCP 服务器并将其工具转换为 Agent 可用的 Tool 格式

use crate::tool::{
    Tool, ToolCall, ToolFunction, ToolParameterProperty, ToolParameters, ToolResult,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// MCP 服务器配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// 启动命令
    pub command: String,
    /// 命令参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// MCP 配置（支持多个服务器）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP 服务器配置映射 (服务器名 -> 配置)
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

impl McpConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一个 MCP 服务器配置
    pub fn with_server(mut self, name: impl Into<String>, config: McpServerConfig) -> Self {
        self.mcp_servers.insert(name.into(), config);
        self
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        for (name, config) in &self.mcp_servers {
            if name.is_empty() || config.command.is_empty() {
                return false;
            }
        }
        !self.mcp_servers.is_empty()
    }
}

/// MCP JSON-RPC 请求
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// MCP JSON-RPC 响应
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// MCP 客户端连接（基于子进程 stdio）
struct McpConnection {
    #[allow(dead_code)]
    server_name: String,
    process: Child,
    request_id: u64,
}

impl McpConnection {
    fn new(server_name: &str, config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let process = cmd.spawn()?;

        Ok(Self {
            server_name: server_name.to_string(),
            process,
            request_id: 0,
        })
    }

    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        self.request_id += 1;
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: self.request_id,
            method: method.to_string(),
            params,
        };

        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let request_str = serde_json::to_string(&request)?;
        writeln!(stdin, "{}", request_str)?;
        stdin.flush()?;

        let stdout = self
            .process
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: JsonRpcResponse = serde_json::from_str(&line)?;

        if let Some(error) = response.error {
            return Err(anyhow!("MCP error: {}", error.message));
        }

        response
            .result
            .ok_or_else(|| anyhow!("Empty result from MCP server"))
    }

    fn initialize(&mut self) -> Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "fuxi-quant-agent",
                "version": "1.0.0"
            }
        });

        self.send_request("initialize", Some(params))?;
        self.send_request("notifications/initialized", None)?;
        Ok(())
    }

    fn list_tools(&mut self) -> Result<Vec<McpToolInfo>> {
        let result = self.send_request("tools/list", None)?;
        let tools: McpToolsListResult = serde_json::from_value(result)?;
        Ok(tools.tools)
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<String> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params))?;
        let call_result: McpToolCallResult = serde_json::from_value(result)?;

        let content = call_result
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(content)
    }
}

impl Drop for McpConnection {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[derive(Deserialize)]
struct McpToolsListResult {
    tools: Vec<McpToolInfo>,
}

#[derive(Deserialize)]
struct McpToolInfo {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "inputSchema", default)]
    input_schema: Value,
}

#[derive(Deserialize)]
struct McpToolCallResult {
    content: Vec<McpContent>,
}

#[derive(Deserialize)]
struct McpContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// MCP 管理器
///
/// 管理多个 MCP 服务器连接，并提供工具转换和调用功能
pub struct McpManager {
    connections: Arc<Mutex<HashMap<String, McpConnection>>>,
    tools: Arc<Mutex<Vec<Tool>>>,
    tool_to_server: Arc<Mutex<HashMap<String, String>>>,
}

impl McpManager {
    /// 创建新的 MCP 管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            tools: Arc::new(Mutex::new(Vec::new())),
            tool_to_server: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 根据配置初始化所有 MCP 服务器连接
    pub fn init(&self, config: &McpConfig) -> Result<Vec<Tool>> {
        if !config.is_valid() {
            return Err(anyhow!("Invalid MCP configuration"));
        }

        let mut all_tools = Vec::new();
        let mut conns = self.connections.lock().unwrap();
        let mut t2s = self.tool_to_server.lock().unwrap();

        for (server_name, server_config) in &config.mcp_servers {
            match Self::connect_server(server_name, server_config) {
                Ok((mut conn, server_tools)) => {
                    for tool in &server_tools {
                        t2s.insert(tool.function.name.clone(), server_name.clone());
                    }
                    all_tools.extend(server_tools);

                    if let Err(e) = conn.initialize() {
                        eprintln!(
                            "Warning: Failed to initialize MCP server '{}': {}",
                            server_name, e
                        );
                    }

                    conns.insert(server_name.clone(), conn);
                }
                Err(e) => {
                    eprintln!("Failed to connect to MCP server '{}': {}", server_name, e);
                }
            }
        }

        *self.tools.lock().unwrap() = all_tools.clone();
        Ok(all_tools)
    }

    /// 连接到单个 MCP 服务器
    fn connect_server(
        server_name: &str,
        config: &McpServerConfig,
    ) -> Result<(McpConnection, Vec<Tool>)> {
        let mut conn = McpConnection::new(server_name, config)?;
        conn.initialize()?;

        let mcp_tools = conn.list_tools()?;
        let tools = Self::convert_mcp_tools(server_name, &mcp_tools);

        Ok((conn, tools))
    }

    /// 将 MCP 工具转换为 Agent Tool 格式
    fn convert_mcp_tools(server_name: &str, mcp_tools: &[McpToolInfo]) -> Vec<Tool> {
        mcp_tools
            .iter()
            .map(|mcp_tool| Self::convert_single_tool(server_name, mcp_tool))
            .collect()
    }

    /// 转换单个 MCP 工具
    fn convert_single_tool(server_name: &str, mcp_tool: &McpToolInfo) -> Tool {
        let tool_name = format!("{}-{}", server_name, mcp_tool.name);
        let description = mcp_tool
            .description
            .clone()
            .unwrap_or_else(|| format!("Tool from MCP server '{}'", server_name));

        let parameters = Self::convert_input_schema(&mcp_tool.input_schema);

        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: tool_name,
                description,
                parameters,
            },
        }
    }

    /// 转换 MCP 工具的输入 schema 为 ToolParameters
    fn convert_input_schema(schema: &Value) -> ToolParameters {
        let mut params = ToolParameters::default();

        if let Some(obj) = schema.as_object() {
            if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
                for (name, prop_value) in props {
                    if let Some(prop) = Self::convert_property(prop_value) {
                        params.properties.insert(name.clone(), prop);
                    }
                }
            }

            if let Some(required) = obj.get("required").and_then(|v| v.as_array()) {
                params.required = required
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }

        params
    }

    /// 转换单个属性
    fn convert_property(value: &Value) -> Option<ToolParameterProperty> {
        let obj = value.as_object()?;

        let param_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string")
            .to_string();

        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let enum_values = obj.get("enum").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        });

        let default = obj.get("default").cloned();

        Some(ToolParameterProperty {
            param_type,
            description,
            enum_values,
            default,
        })
    }

    /// 获取所有已加载的工具
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools.lock().unwrap().clone()
    }

    /// 调用 MCP 工具
    pub fn call_tool(&self, call: &ToolCall) -> Result<ToolResult> {
        let tool_name = &call.name;

        let t2s = self.tool_to_server.lock().unwrap();
        let server_name = t2s
            .get(tool_name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", tool_name))?
            .clone();
        drop(t2s);

        let original_tool_name = tool_name
            .strip_prefix(&format!("{}-", server_name))
            .unwrap_or(tool_name);

        let mut conns = self.connections.lock().unwrap();
        let conn = conns
            .get_mut(&server_name)
            .ok_or_else(|| anyhow!("Server '{}' not connected", server_name))?;

        let content = conn.call_tool(original_tool_name, &call.arguments)?;

        Ok(ToolResult {
            name: tool_name.clone(),
            content: serde_json::json!({ "result": content }),
        })
    }

    /// 关闭所有连接
    pub fn shutdown(&self) {
        let mut conns = self.connections.lock().unwrap();
        conns.clear();
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 创建 MCP 工具执行器
///
/// 返回一个闭包，可以用于 `Agent::chat_with_tools` 的 `tool_executor` 参数
pub fn create_mcp_executor(
    manager: Arc<McpManager>,
) -> impl FnMut(&ToolCall) -> Option<ToolResult> {
    move |call: &ToolCall| match manager.call_tool(call) {
        Ok(result) => Some(result),
        Err(e) => {
            eprintln!("MCP tool call error: {}", e);
            Some(ToolResult {
                name: call.name.clone(),
                content: serde_json::json!({ "error": e.to_string() }),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config() {
        let config = McpConfig::new().with_server(
            "sqlite",
            McpServerConfig {
                command: "uvx".to_string(),
                args: vec![
                    "mcp-server-sqlite".to_string(),
                    "--db-path".to_string(),
                    "test.db".to_string(),
                ],
                env: HashMap::new(),
            },
        );

        assert!(config.is_valid());
        assert!(config.mcp_servers.contains_key("sqlite"));
    }

    #[test]
    fn test_empty_config_invalid() {
        let config = McpConfig::new();
        assert!(!config.is_valid());
    }

    #[test]
    fn test_convert_input_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "SQL query to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Query timeout in seconds",
                    "default": 30
                }
            },
            "required": ["query"]
        });

        let params = McpManager::convert_input_schema(&schema);
        assert_eq!(params.properties.len(), 2);
        assert!(params.properties.contains_key("query"));
        assert!(params.required.contains(&"query".to_string()));
    }
}
