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

/// 生成带工具的 System Prompt
///
/// 格式参考 Qwen3 官方 chat template (Hermes 风格)
pub fn build_tool_system_prompt(system_content: Option<&str>, tools: &[Tool]) -> String {
    let mut prompt = String::new();

    // 添加用户自定义的 system 内容
    if let Some(content) = system_content {
        prompt.push_str(content);
        prompt.push_str("\n\n");
    }

    // 添加工具说明
    prompt.push_str("# Tools\n\n");
    prompt.push_str("You have access to the following tools. **You MUST use tools when needed** - do not make up information.\n\n");
    prompt.push_str("For example:\n");
    prompt.push_str("- When asked about current time/date, you MUST call `get_current_time`\n");
    prompt.push_str("- Never guess or hallucinate real-time information\n\n");
    prompt.push_str(
        "You are provided with function signatures within <tools></tools> XML tags:\n<tools>",
    );

    // 添加每个工具的 JSON 定义
    for tool in tools {
        prompt.push('\n');
        if let Ok(json) = serde_json::to_string(tool) {
            prompt.push_str(&json);
        }
    }

    prompt.push_str("\n</tools>\n\n");
    prompt.push_str("For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n");
    prompt.push_str("<tool_call>\n");
    prompt.push_str(r#"{"name": <function-name>, "arguments": <args-json-object>}"#);
    prompt.push_str("\n</tool_call>");

    prompt
}

/// 解析模型输出中的工具调用
///
/// 查找所有 `<tool_call>...</tool_call>` 块并解析
pub fn parse_tool_calls(output: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut search_start = 0;

    while let Some(start) = output[search_start..].find("<tool_call>") {
        let abs_start = search_start + start + "<tool_call>".len();
        if let Some(end) = output[abs_start..].find("</tool_call>") {
            let abs_end = abs_start + end;
            let json_str = output[abs_start..abs_end].trim();

            if let Ok(call) = serde_json::from_str::<ToolCall>(json_str) {
                calls.push(call);
            }

            search_start = abs_end + "</tool_call>".len();
        } else {
            break;
        }
    }

    calls
}

/// 检查输出是否包含工具调用
pub fn has_tool_call(output: &str) -> bool {
    output.contains("<tool_call>")
}

/// 生成工具结果消息内容
///
/// 格式: `<tool_response>\n{json}\n</tool_response>`
pub fn format_tool_response(result: &ToolResult) -> String {
    let json = serde_json::to_string(&result.content).unwrap_or_default();
    format!("<tool_response>\n{}\n</tool_response>", json)
}

/// 批量格式化多个工具结果
pub fn format_tool_responses(results: &[ToolResult]) -> String {
    results
        .iter()
        .map(format_tool_response)
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从输出中提取非工具调用的文本内容
pub fn extract_content_without_tool_calls(output: &str) -> String {
    let mut result = output.to_string();
    while let Some(start) = result.find("<tool_call>") {
        if let Some(end) = result[start..].find("</tool_call>") {
            let abs_end = start + end + "</tool_call>".len();
            result = format!("{}{}", &result[..start], &result[abs_end..]);
        } else {
            break;
        }
    }
    result.trim().to_string()
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
        assert!(prompt.contains("# Tools"));
        assert!(prompt.contains("<tools>"));
        assert!(prompt.contains("get_weather"));
        assert!(prompt.contains("<tool_call>"));
    }

    #[test]
    fn test_parse_tool_calls() {
        let output = r#"Let me check the weather.
<tool_call>
{"name": "get_weather", "arguments": {"city": "Beijing"}}
</tool_call>
<tool_call>
{"name": "get_weather", "arguments": {"city": "Shanghai"}}
</tool_call>"#;

        let calls = parse_tool_calls(output);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[1].arguments["city"], "Shanghai");
    }

    #[test]
    fn test_format_tool_response() {
        let result = ToolResult {
            name: "get_weather".to_string(),
            content: serde_json::json!({"temp": 25, "weather": "sunny"}),
        };

        let response = format_tool_response(&result);
        assert!(response.contains("<tool_response>"));
        assert!(response.contains("</tool_response>"));
        assert!(response.contains("sunny"));
    }
}
