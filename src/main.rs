use savethebeat::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    savethebeat::run(config).await
}
