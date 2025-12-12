//! 工具定义模块

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 工具参数属性
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameterProperty {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

/// 工具参数定义
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: HashMap<String, ToolParameterProperty>,
    #[serde(default)]
    pub required: Vec<String>,
}

impl Default for ToolParameters {
    fn default() -> Self {
        Self {
            param_type: "object".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }
}

/// 工具函数定义
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: ToolParameters,
}

/// 工具定义
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

/// 工具调用
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// 工具调用结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub content: Value,
}

/// 检查输出是否包含工具调用
pub fn has_tool_call(output: &str) -> bool {
    output.contains("Action:") && output.contains("Action Input:")
}

/// 解析工具调用
pub fn parse_tool_calls(output: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(action_pos) = output.rfind("Action:") {
        let action_line_start = action_pos + "Action:".len();
        let action_line_end = output[action_line_start..]
            .find('\n')
            .map(|p| action_line_start + p)
            .unwrap_or(output.len());
        let action_name = output[action_line_start..action_line_end].trim();

        if let Some(input_pos) = output[action_line_end..].find("Action Input:") {
            let input_start = action_line_end + input_pos + "Action Input:".len();
            let input_end = output[input_start..]
                .find("\nObservation:")
                .or_else(|| output[input_start..].find("\nThought:"))
                .map(|p| input_start + p)
                .unwrap_or(output.len());
            let input_str = output[input_start..input_end].trim();

            let arguments: Value = if input_str.starts_with('{') {
                serde_json::from_str(input_str).unwrap_or(Value::Object(Default::default()))
            } else {
                serde_json::json!({ "input": input_str })
            };

            if !action_name.is_empty() {
                calls.push(ToolCall {
                    name: action_name.to_string(),
                    arguments,
                });
            }
        }
    }

    calls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls() {
        let output = r#"Thought: I need to check the weather.
Action: get_weather
Action Input: {"city": "Beijing"}
"#;

        let calls = parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["city"], "Beijing");
    }

    #[test]
    fn test_has_tool_call() {
        assert!(has_tool_call("Action: test\nAction Input: {}"));
        assert!(!has_tool_call("No tool call here"));
    }
}
