use anyhow::{Result, anyhow};
use fuxi_quant_core::types::*;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

/// 策略脚本
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Script {
    File(PathBuf),
    Source(String),
}

impl Default for Script {
    fn default() -> Self {
        Self::File(PathBuf::from("script.fx"))
    }
}

pub struct ScriptStrategy {
    runtime: Runtime,
}

impl ScriptStrategy {
    pub fn new(script: &Script, gas_max: u64) -> Result<Self> {
        let runtime = Runtime::new(script, gas_max)?;
        Ok(Self { runtime })
    }

    fn call(&mut self, name: &str, args: impl rhai::FuncArgs) -> Result<()> {
        let gas_usage_before = self.runtime.gas_usage.load(Ordering::Relaxed);

        self.runtime
            .engine
            .call_fn_with_options::<()>(
                rhai::CallFnOptions::new().bind_this_ptr(&mut self.runtime.this),
                &mut self.runtime.scope,
                &self.runtime.ast,
                name,
                args,
            )
            .map_err(|err| {
                let position = err.position();
                let msg = match err.unwrap_inner() {
                    rhai::EvalAltResult::ErrorTerminated(msg, ..) => msg.to_string(),
                    _ => err.to_string(),
                };
                if position.is_none() {
                    anyhow!("{msg}")
                } else {
                    anyhow!("{msg} {position}")
                }
            })?;

        let gas_usage_after = self.runtime.gas_usage.load(Ordering::Relaxed);

        tracing::debug!(
            "⛽ {name} used gas: {}, total used gas: {}",
            gas_usage_after - gas_usage_before,
            gas_usage_after
        );

        Ok(())
    }
}

impl Strategy for ScriptStrategy {
    #[tracing::instrument(skip_all)]
    fn on_start(&mut self, engine: &mut dyn Engine) -> Result<()> {
        let engine = EngineProvider::new(engine);

        self.runtime
            .this
            .as_map_mut()
            .map_err(|err| anyhow!(err))?
            .insert("api".into(), rhai::Dynamic::from(engine));

        self.call("on_start", ())?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn on_stop(&mut self, _: &mut dyn Engine) -> Result<()> {
        self.call("on_stop", ())?;

        self.runtime
            .this
            .as_map_mut()
            .map_err(|err| anyhow!(err))?
            .remove("api");

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn on_bar(&mut self, _: &mut dyn Engine, code: &str) -> Result<()> {
        self.call("on_bar", (code.to_string(),))
    }

    #[tracing::instrument(skip_all)]
    fn on_signal(&mut self, _: &mut dyn Engine) -> Result<()> {
        self.call("on_signal", ())
    }

    #[tracing::instrument(skip_all)]
    fn on_timer(&mut self, _: &mut dyn Engine, timer: Timer, time: Time) -> Result<()> {
        self.call("on_timer", (timer, time))
    }

    #[tracing::instrument(skip_all)]
    fn on_order(&mut self, _: &mut dyn Engine, order_id: &str) -> Result<()> {
        self.call("on_order", (order_id.to_string(),))
    }

    #[tracing::instrument(skip_all)]
    fn on_position(&mut self, _: &mut dyn Engine, code: &str) -> Result<()> {
        self.call("on_position", (code.to_string(),))
    }
}

pub struct Runtime {
    pub engine: rhai::Engine,
    pub scope: rhai::Scope<'static>,
    pub this: rhai::Dynamic,
    pub ast: rhai::AST,
    pub gas_max: u64,
    pub gas_usage: Arc<AtomicU64>,
}

impl Runtime {
    pub fn new(script: &Script, gas_max: u64) -> Result<Self> {
        let gas_usage = Arc::new(AtomicU64::new(0));

        let mut engine = rhai::Engine::new();

        {
            let gas_usage = gas_usage.clone();
            engine.on_progress(move |_| {
                if gas_max == 0 {
                    return None;
                }

                let usage = gas_usage.load(Ordering::Relaxed);
                if usage > gas_max {
                    return Some(format!("gas usage exceeds the limit: {gas_max}").into());
                }

                gas_usage.fetch_add(1, Ordering::Relaxed);

                None
            });
        }

        engine.on_print(move |s| tracing::info!("{s}"));

        crate::pl::register(&mut engine);
        crate::builtin::register(&mut engine);
        crate::time::register(&mut engine);

        let ast = match script {
            Script::File(path) => engine
                .compile_file(path.to_owned())
                .map_err(|e| anyhow!("{e}"))?,
            Script::Source(code) => engine.compile(code).map_err(|e| anyhow!("{e}"))?,
        };

        Ok(Self {
            engine,
            scope: rhai::Scope::new(),
            this: rhai::Dynamic::from(rhai::Map::new()),
            ast,
            gas_usage,
            gas_max,
        })
    }
}
