fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.yaml".to_string());

    let content = std::fs::read_to_string(config_path)?;

    let config: fuxi_quant::types::Config = serde_yml::from_str(&content)?;

    fuxi_quant::run(config)
}
