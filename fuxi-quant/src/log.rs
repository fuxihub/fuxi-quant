use std::borrow::Cow;
use std::fmt;
use std::fmt::Write;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::registry::LookupSpan;

struct LogFormat {
    /// 是否显示 span 的 begin/end 时间信息
    pub show_span_timing: bool,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self {
            show_span_timing: true,
        }
    }
}

impl LogFormat {
    pub fn new(show_span_timing: bool) -> Self {
        Self { show_span_timing }
    }
}

impl<S, N> FormatEvent<S, N> for LogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();

        // 检查是否是 span 事件（new/close）
        if meta.is_span() {
            // 如果关闭了 span timing，直接跳过
            if !self.show_span_timing {
                return Ok(());
            }

            if ctx.lookup_current().is_some() {
                let mut visitor = SpanEventVisitor::default();
                event.record(&mut visitor);

                if let Some(msg) = &visitor.message {
                    // 只在需要输出时构建 span_path
                    if msg == "close" {
                        let idle = visitor.time_idle.as_deref().unwrap_or("?");
                        let busy = visitor.time_busy.as_deref().unwrap_or("?");
                        write!(writer, "      ")?;
                        self.write_span_path(ctx, &mut writer)?;
                        return writeln!(
                            writer,
                            ": └────────────────────────────────────────────────────── idle: {idle}, busy: {busy}"
                        );
                    }
                }
            }
            return Ok(());
        }

        // 普通事件
        write!(writer, "{:5} ", meta.level())?;
        if let Some(scope) = ctx.event_scope() {
            let mut first = true;
            for span in scope.from_root() {
                if !first {
                    write!(writer, " -> ")?;
                }
                // 优先使用 ________topic________ 字段，否则用 span name
                let name = get_span_display_name::<S, N>(&span);
                write!(writer, "{}", name)?;
                first = false;
            }
            if !first {
                // 开启 span timing 时缩进，否则不缩进
                if self.show_span_timing {
                    write!(writer, ": │")?;
                } else {
                    write!(writer, ": ")?;
                }
            }
        }
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

impl LogFormat {
    // 直接写入 writer，避免中间 String 分配
    fn write_span_path<S, N>(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut Writer<'_>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        if let Some(scope) = ctx.event_scope() {
            let mut first = true;
            for span in scope.from_root() {
                if !first {
                    write!(writer, " -> ")?;
                }
                // 优先使用 ________topic________ 字段，否则用 span name
                let name = get_span_display_name::<S, N>(&span);
                write!(writer, "{}", name)?;
                first = false;
            }
        }
        Ok(())
    }
}

const TOPIC_FIELD_PREFIX: &str = "________topic________=";

/// 获取 span 的显示名称：优先使用 ________topic________ 字段，否则用 span name
fn get_span_display_name<'a, S, N>(
    span: &tracing_subscriber::registry::SpanRef<'a, S>,
) -> Cow<'a, str>
where
    S: LookupSpan<'a>,
    N: for<'b> FormatFields<'b> + 'static,
{
    span.extensions()
        .get::<FormattedFields<N>>()
        .and_then(|fields| extract_field(&fields.fields))
        .map(Cow::Owned)
        .unwrap_or_else(|| Cow::Borrowed(span.name()))
}

/// 从格式化的字段字符串中提取 topic 字段的值
fn extract_field(fields: &str) -> Option<String> {
    if let Some(start) = fields.find(TOPIC_FIELD_PREFIX) {
        let value_start = start + TOPIC_FIELD_PREFIX.len();
        let rest = &fields[value_start..];
        // 找到值的结束位置（空格、逗号或字符串结尾）
        let end = rest.find([' ', ',']).unwrap_or(rest.len());
        let value = &rest[..end];
        return Some(value.trim_matches('"').to_string());
    }
    None
}

#[derive(Default)]
struct SpanEventVisitor {
    message: Option<String>,
    time_busy: Option<String>,
    time_idle: Option<String>,
}

impl tracing::field::Visit for SpanEventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        match field.name() {
            "message" => {
                let mut s = String::new();
                let _ = write!(s, "{:?}", value);
                // 去掉首尾引号（使用切片避免 O(n) 的 remove）
                self.message = Some(if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                });
            }
            "time.busy" => {
                let mut s = String::new();
                let _ = write!(s, "{:?}", value);
                self.time_busy = Some(s);
            }
            "time.idle" => {
                let mut s = String::new();
                let _ = write!(s, "{:?}", value);
                self.time_idle = Some(s);
            }
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" => self.message = Some(value.to_string()),
            "time.busy" => self.time_busy = Some(value.to_string()),
            "time.idle" => self.time_idle = Some(value.to_string()),
            _ => {}
        }
    }
}

pub fn new_subscriber(level: LevelFilter, show_span_timing: bool) -> impl Subscriber + Send + Sync {
    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_file(false)
                .with_line_number(false)
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_span_events(if show_span_timing {
                    FmtSpan::NEW | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .event_format(LogFormat::new(show_span_timing)),
        )
        .with(
            Targets::new()
                .with_target("fuxi_quant", level)
                .with_target("fuxi_quant_core", level)
                .with_target("fuxi_quant_runtime", level),
        )
}
