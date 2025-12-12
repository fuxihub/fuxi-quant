use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 工具参数属性定义
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

/// 工具定义（OpenAI 兼容格式）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

impl Tool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.into(),
                description: description.into(),
                parameters: ToolParameters::default(),
            },
        }
    }

    pub fn with_param(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.function.parameters.properties.insert(
            name.clone(),
            ToolParameterProperty {
                param_type: param_type.into(),
                description: description.into(),
                enum_values: None,
                default: None,
            },
        );
        if required {
            self.function.parameters.required.push(name);
        }
        self
    }

    pub fn with_enum_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        enum_values: Vec<String>,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.function.parameters.properties.insert(
            name.clone(),
            ToolParameterProperty {
                param_type: "string".to_string(),
                description: description.into(),
                enum_values: Some(enum_values),
                default: None,
            },
        );
        if required {
            self.function.parameters.required.push(name);
        }
        self
    }
}

/// 模型输出的工具调用
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

/// 生成单个工具的 ReAct 格式描述
fn format_tool_desc(tool: &Tool) -> String {
    let params_json = serde_json::to_string(&tool.function.parameters).unwrap_or_default();
    format!(
        "{}: Call this tool to interact with the {} API. What is the {} API useful for? {} Parameters: {}",
        tool.function.name,
        tool.function.name,
        tool.function.name,
        tool.function.description,
        params_json
    )
}

/// 生成带工具的 System Prompt (ReAct 模式)
pub fn build_tool_system_prompt(system_content: Option<&str>, tools: &[Tool]) -> String {
    let mut prompt = String::new();

    // 添加用户自定义的 system 内容
    if let Some(content) = system_content {
        prompt.push_str(content);
        prompt.push_str("\n\n");
    }

    // 生成工具描述列表
    let tool_descs: Vec<String> = tools.iter().map(format_tool_desc).collect();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();

    // ReAct 提示词模板
    prompt.push_str("Answer the following questions as best you can. You have access to the following tools:\n\n");
    prompt.push_str(&tool_descs.join("\n"));
    prompt.push_str("\n\nUse the following format:\n\n");
    prompt.push_str("Question: the input question you must answer\n");
    prompt.push_str("Thought: you should always think about what to do\n");
    prompt.push_str(&format!(
        "Action: the action to take, should be one of [{}]\n",
        tool_names.join(", ")
    ));
    prompt.push_str("Action Input: the input to the action\n");
    prompt.push_str("Observation: the result of the action\n");
    prompt.push_str(
        "... (this Thought/Action/Action Input/Observation can be repeated zero or more times)\n",
    );
    prompt.push_str("Thought: I now know the final answer\n");
    prompt.push_str("Final Answer: the final answer to the original input question\n\n");
    prompt.push_str("Begin!");

    prompt
}

/// 解析模型输出中的工具调用 (ReAct 格式)
///
/// 查找 `Action:` 和 `Action Input:` 并解析
pub fn parse_tool_calls(output: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // 查找最后一个 Action/Action Input 对
    if let Some(action_pos) = output.rfind("Action:") {
        let action_line_start = action_pos + "Action:".len();
        let action_line_end = output[action_line_start..]
            .find('\n')
            .map(|p| action_line_start + p)
            .unwrap_or(output.len());
        let action_name = output[action_line_start..action_line_end].trim();

        // 查找对应的 Action Input
        if let Some(input_pos) = output[action_line_end..].find("Action Input:") {
            let input_start = action_line_end + input_pos + "Action Input:".len();
            // Action Input 可能是单行或多行 JSON
            let input_end = output[input_start..]
                .find("\nObservation:")
                .or_else(|| output[input_start..].find("\nThought:"))
                .map(|p| input_start + p)
                .unwrap_or(output.len());
            let input_str = output[input_start..input_end].trim();

            // 解析 Action Input 为 JSON
            let arguments: Value = if input_str.starts_with('{') {
                serde_json::from_str(input_str).unwrap_or(Value::Object(Default::default()))
            } else {
                // 如果不是 JSON，包装为字符串参数
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

/// 检查输出是否包含工具调用 (ReAct 格式)
pub fn has_tool_call(output: &str) -> bool {
    output.contains("Action:") && output.contains("Action Input:")
}

/// 生成工具结果消息内容 (ReAct 格式)
///
/// 格式: `Observation: {result}`
pub fn format_tool_response(result: &ToolResult) -> String {
    let json = serde_json::to_string_pretty(&result.content).unwrap_or_default();
    format!("Observation: {}", json)
}

/// 批量格式化多个工具结果 (ReAct 格式)
pub fn format_tool_responses(results: &[ToolResult]) -> String {
    results
        .iter()
        .map(format_tool_response)
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从输出中提取 Final Answer (ReAct 格式)
pub fn extract_content_without_tool_calls(output: &str) -> String {
    // 查找 Final Answer
    if let Some(pos) = output.find("Final Answer:") {
        let start = pos + "Final Answer:".len();
        return output[start..].trim().to_string();
    }
    // 如果没有 Final Answer，返回去掉 Action 部分的内容
    if let Some(pos) = output.find("Action:") {
        return output[..pos].trim().to_string();
    }
    output.trim().to_string()
}

/// 内置工具模块
pub mod builtin {
    use super::{Tool, ToolCall, ToolResult};
    use chrono::{Local, Utc};
    use chrono_tz::Tz;
    use serde_json::json;

    /// 获取当前时间工具定义
    pub fn get_current_time_tool() -> Tool {
        Tool::new("get_current_time", "获取当前时间，支持指定时区")
            .with_param(
                "timezone",
                "string",
                "时区名称，如 Asia/Shanghai、UTC、America/New_York。默认为本地时区。",
                false,
            )
            .with_param(
                "format",
                "string",
                "时间格式，如 %Y-%m-%d %H:%M:%S。默认为 ISO 8601 格式。",
                false,
            )
    }

    /// 执行获取当前时间
    pub fn execute_get_current_time(call: &ToolCall) -> ToolResult {
        let timezone = call.arguments.get("timezone").and_then(|v| v.as_str());
        let format = call
            .arguments
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let (time_str, tz_name) = match timezone {
            Some(tz_str) => {
                if let Ok(tz) = tz_str.parse::<Tz>() {
                    let now = Utc::now().with_timezone(&tz);
                    (now.format(format).to_string(), tz_str.to_string())
                } else {
                    // 无效时区，使用本地时间
                    let now = Local::now();
                    (
                        now.format(format).to_string(),
                        format!("Local (invalid timezone: {})", tz_str),
                    )
                }
            }
            None => {
                let now = Local::now();
                (now.format(format).to_string(), "Local".to_string())
            }
        };

        ToolResult {
            name: call.name.clone(),
            content: json!({
                "current_time": time_str,
                "timezone": tz_name,
                "unix_timestamp": Utc::now().timestamp(),
            }),
        }
    }

    /// 内置工具执行器
    ///
    /// 根据工具名称执行对应的内置工具，返回 None 表示不是内置工具
    pub fn execute_builtin(call: &ToolCall) -> Option<ToolResult> {
        match call.name.as_str() {
            "get_current_time" => Some(execute_get_current_time(call)),
            _ => None,
        }
    }

    /// 获取所有内置工具
    pub fn all_builtin_tools() -> Vec<Tool> {
        vec![get_current_time_tool()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tool_system_prompt() {
        let tools = vec![
            Tool::new("get_weather", "Get the current weather").with_param(
                "city",
                "string",
                "The city name",
                true,
            ),
        ];

        let prompt = build_tool_system_prompt(Some("You are a helpful assistant."), &tools);
        assert!(prompt.contains("Answer the following questions"));
        assert!(prompt.contains("get_weather"));
        assert!(prompt.contains("Action:"));
        assert!(prompt.contains("Action Input:"));
        assert!(prompt.contains("Observation:"));
        assert!(prompt.contains("Final Answer:"));
    }

    #[test]
    fn test_parse_tool_calls_react() {
        let output = r#"Thought: I need to check the weather in Beijing.
Action: get_weather
Action Input: {"city": "Beijing"}
"#;

        let calls = parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["city"], "Beijing");
    }

    #[test]
    fn test_parse_tool_calls_simple_input() {
        let output = r#"Thought: I need to search for something.
Action: search
Action Input: rust programming
"#;

        let calls = parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search");
        assert_eq!(calls[0].arguments["input"], "rust programming");
    }

    #[test]
    fn test_format_tool_response() {
        let result = ToolResult {
            name: "get_weather".to_string(),
            content: serde_json::json!({"temp": 25, "weather": "sunny"}),
        };

        let response = format_tool_response(&result);
        assert!(response.starts_with("Observation:"));
        assert!(response.contains("sunny"));
    }

    #[test]
    fn test_extract_final_answer() {
        let output = r#"Thought: I now know the final answer.
Final Answer: The weather in Beijing is sunny with 25°C."#;

        let answer = extract_content_without_tool_calls(output);
        assert_eq!(answer, "The weather in Beijing is sunny with 25°C.");
    }
}
