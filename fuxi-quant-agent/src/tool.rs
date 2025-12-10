//! 工具定义模块 - 支持 Qwen3 Function Calling 和 MCP
//!
//! Qwen3 使用 Hermes-style tool use format，工具定义需要符合 JSON Schema 格式。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工具类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Function,
}

/// 工具定义（OpenAI 兼容格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub function: FunctionDef,
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 工具调用请求（模型输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Tool {
    /// 从类型自动生成工具定义
    pub fn from_schema<T: JsonSchema>(name: &str, description: &str) -> Self {
        let schema = schemars::schema_for!(T);
        Self {
            tool_type: ToolType::Function,
            function: FunctionDef {
                name: name.to_string(),
                description: description.to_string(),
                parameters: serde_json::to_value(schema).unwrap_or_default(),
            },
        }
    }

    /// 手动创建工具定义
    pub fn new(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: ToolType::Function,
            function: FunctionDef {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }
}

/// 工具注册表
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册工具
    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.function.name.clone(), tool);
    }

    /// 获取所有工具定义
    pub fn tools(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    /// 生成 Qwen3 的工具提示词
    pub fn to_tool_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let tools_json: Vec<_> = self.tools.values().collect();
        let tools_str = serde_json::to_string_pretty(&tools_json).unwrap_or_default();

        format!(
            r#"# Tools

You may call one or more functions to assist with the user query.

You are provided with function signatures within <tools></tools> XML tags:
<tools>
{tools_str}
</tools>

For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:
<tool_call>
{{"name": "<function-name>", "arguments": <args-json-object>}}
</tool_call>"#
        )
    }

    /// 查找工具
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }
}

/// 解析模型输出中的工具调用
pub fn parse_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // 查找 <tool_call>...</tool_call> 块
    let mut remaining = response;
    while let Some(start) = remaining.find("<tool_call>") {
        if let Some(end) = remaining[start..].find("</tool_call>") {
            let content = &remaining[start + 11..start + end];
            if let Ok(call) = serde_json::from_str::<ToolCall>(content.trim()) {
                calls.push(call);
            }
            remaining = &remaining[start + end + 12..];
        } else {
            break;
        }
    }

    calls
}

/// 格式化工具调用结果（供下一轮对话使用）
pub fn format_tool_results(results: &[ToolResult]) -> String {
    let mut output = String::new();
    for result in results {
        if let Some(err) = &result.error {
            output.push_str(&format!(
                "<tool_response>\nTool: {}\nError: {}\n</tool_response>\n",
                result.name, err
            ));
        } else {
            output.push_str(&format!(
                "<tool_response>\nTool: {}\nResult: {}\n</tool_response>\n",
                result.name, result.content
            ));
        }
    }
    output
}

// ============ 量化交易相关工具参数定义 ============

/// 获取股票价格参数
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetStockPriceParams {
    /// 股票代码，如 "AAPL", "600519.SH"
    pub symbol: String,
}

/// 获取 K 线数据参数
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetKlineParams {
    /// 股票代码
    pub symbol: String,
    /// K线周期: "1m", "5m", "15m", "1h", "1d"
    pub interval: String,
    /// 获取条数，默认 100
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

/// 计算技术指标参数
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalcIndicatorParams {
    /// 股票代码
    pub symbol: String,
    /// 指标类型: "ma", "ema", "macd", "rsi", "bollinger"
    pub indicator: String,
    /// 指标参数（如 MA 周期）
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

/// 执行回测参数
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunBacktestParams {
    /// 策略代码
    pub strategy_code: String,
    /// 股票代码列表
    pub symbols: Vec<String>,
    /// 开始日期 (YYYY-MM-DD)
    pub start_date: String,
    /// 结束日期 (YYYY-MM-DD)
    pub end_date: String,
    /// 初始资金
    #[serde(default = "default_capital")]
    pub initial_capital: f64,
}

fn default_capital() -> f64 {
    100000.0
}

/// 创建量化交易工具注册表
pub fn create_quant_tools() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    registry.register(Tool::from_schema::<GetStockPriceParams>(
        "get_stock_price",
        "获取指定股票的当前价格和基本行情信息",
    ));

    registry.register(Tool::from_schema::<GetKlineParams>(
        "get_kline",
        "获取指定股票的 K 线数据，支持多种时间周期",
    ));

    registry.register(Tool::from_schema::<CalcIndicatorParams>(
        "calc_indicator",
        "计算技术指标，如均线(MA)、MACD、RSI、布林带等",
    ));

    registry.register(Tool::from_schema::<RunBacktestParams>(
        "run_backtest",
        "执行策略回测，返回收益率、最大回撤等统计指标",
    ));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_prompt() {
        let registry = create_quant_tools();
        let prompt = registry.to_tool_prompt();
        println!("{}", prompt);
        assert!(prompt.contains("get_stock_price"));
        assert!(prompt.contains("<tools>"));
    }

    #[test]
    fn test_parse_tool_call() {
        let response = r#"
我需要查询股票价格。

<tool_call>
{"name": "get_stock_price", "arguments": {"symbol": "AAPL"}}
</tool_call>
"#;

        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_stock_price");
    }
}
