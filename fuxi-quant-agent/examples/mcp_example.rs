//! 智能体使用示例
//!
//! 运行示例:
//! ```bash
//! cargo run --example mcp_example -- /path/to/model.gguf
//! ```

use fuxi_quant_agent::{Agent, McpConfig, McpServerConfig, Model};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <model_path>", args[0]);
        std::process::exit(1);
    }

    // 1. 加载模型
    let model = Model::load(&args[1])?;
    let model: &'static Model = Box::leak(Box::new(std::sync::Arc::try_unwrap(model).unwrap()));

    // 2. 创建 MCP 配置（可选）
    let mcp_config = McpConfig::new().with_server(
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

    // 3. 创建智能体（两个参数：上下文长度 + MCP 配置）
    let mut agent = Agent::new(model, 8192, Some(mcp_config))?;

    // 4. 查看可用工具
    println!("可用工具:");
    for tool in agent.tools() {
        println!("  - {}: {}", tool.function.name, tool.function.description);
    }

    // 5. 对话
    println!("\n开始对话...\n");
    let response = agent.chat("你好，请介绍一下你自己。", |token| {
        print!("{}", token);
    })?;
    println!("\n\n最终回复: {}", response);

    // 6. 重置对话
    agent.reset()?;
    println!("\n对话已重置");

    Ok(())
}
