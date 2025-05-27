use color_eyre::{Result, eyre::WrapErr};

#[derive(serde::Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub game: GameConfig,
}

#[derive(serde::Deserialize)]
pub struct BotConfig {
    pub token: String,
    pub db_url: String,
}

#[derive(serde::Deserialize)]
pub struct GameConfig {}

pub fn load(path: &std::path::Path) -> Result<Config> {
    let config =
        std::fs::read_to_string(path).wrap_err("Couldn't load config at the given path")?;
    Ok(toml::from_str(&config)?)
}
