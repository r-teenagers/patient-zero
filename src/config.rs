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
pub struct GameConfig {
    /// The server to run the game in
    pub server_id: u64,
    /// The role to assign to infected players
    pub infected_role: u64,
    /// Roles that cannot be infected
    pub immune_roles: Option<Vec<u64>>,
    /// Roles that always count as being infected
    pub carrier_roles: Option<Vec<u64>>,
    /// The number of messages that must be sent while infected to be cured
    pub cure_threshold: u32,
    /// The minimum amount of time between messages being counted towards curing
    pub message_cooldown: u32,
    /// The minimum amount of time between infections from one person
    pub infection_cooldown: u32,
}

pub fn load(path: &std::path::Path) -> Result<Config> {
    let config =
        std::fs::read_to_string(path).wrap_err("Couldn't load config at the given path")?;
    Ok(toml::from_str(&config)?)
}
