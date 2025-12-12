//! MCP 工具使用示例
//!
//! 本示例展示如何配置和使用 MCP 服务器工具
//!
//! 运行前需要确保已安装相应的 MCP 服务器，例如:
//! - uvx mcp-server-sqlite
//! - npx @modelcontextprotocol/server-filesystem
//!
//! 运行示例:
//! ```bash
//! cargo run --example mcp_example
//! ```

use fuxi_quant_agent::mcp::{McpConfig, McpManager, McpServerConfig, create_mcp_executor};
use fuxi_quant_agent::tool::builtin;
use std::collections::HashMap;
use std::sync::Arc;

fn main() {
    // 1. 创建 MCP 配置
    let mcp_config = McpConfig::new()
        // 添加 SQLite MCP 服务器
        .with_server(
            "sqlite",
            McpServerConfig {
                command: "uvx".to_string(),
                args: vec![
                    "mcp-server-sqlite".to_string(),
                    "--db-path".to_string(),
                    "/tmp/test.db".to_string(),
                ],
                env: HashMap::new(),
            },
        );
    // 可以继续添加更多服务器
    // .with_server(
    //     "filesystem",
    //     McpServerConfig {
    //         command: "npx".to_string(),
    //         args: vec![
    //             "-y".to_string(),
    //             "@modelcontextprotocol/server-filesystem".to_string(),
    //             "/tmp".to_string(),
    //         ],
    //         env: HashMap::new(),
    //     },
    // );

    // 2. 创建 MCP 管理器并初始化
    let manager = Arc::new(McpManager::new());

    match manager.init(&mcp_config) {
        Ok(tools) => {
            println!("已加载 {} 个 MCP 工具:", tools.len());
            for tool in &tools {
                println!("  - {}: {}", tool.function.name, tool.function.description);
            }
        }
        Err(e) => {
            eprintln!("MCP 初始化失败: {}", e);
            return;
        }
    }

    // 3. 获取所有工具（包括内置工具和 MCP 工具）
    let mut all_tools = builtin::all_builtin_tools();
    all_tools.extend(manager.get_tools());

    println!("\n所有可用工具:");
    for tool in &all_tools {
        println!("  - {}", tool.function.name);
    }

    // 4. 创建 MCP 工具执行器
    let mcp_executor = create_mcp_executor(manager.clone());

    // 5. 组合执行器（内置 + MCP）
    let _combined_executor = create_combined_executor(mcp_executor);

    println!("\nMCP 工具已准备就绪，可以在 Agent 中使用");

    // 示例: 如何在 Agent 中使用
    // let config = AgentConfig::new()
    //     .with_tools(all_tools)
    //     .with_mcp_config(mcp_config);
    // let mut agent = Agent::new(&MODEL, config)?;
    // agent.chat_with_tools("查询数据库中的用户表", |event| {...}, combined_executor)?;
}

/// 创建组合执行器，同时支持内置工具和 MCP 工具
fn create_combined_executor<F>(
    mut mcp_executor: F,
) -> impl FnMut(&fuxi_quant_agent::tool::ToolCall) -> Option<fuxi_quant_agent::tool::ToolResult>
where
    F: FnMut(&fuxi_quant_agent::tool::ToolCall) -> Option<fuxi_quant_agent::tool::ToolResult>,
{
    move |call| {
        // 先尝试内置工具
        if let Some(result) = builtin::execute_builtin(call) {
            return Some(result);
        }
        // 再尝试 MCP 工具
        mcp_executor(call)
    }
}
