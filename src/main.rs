use std::{
    collections::HashMap,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre::WrapErr};
use helpers::MessageBuffer;
use poise::serenity_prelude as serenity;
use serenity::GatewayIntents;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tracing::Level;
use tracing_subscriber::{filter, prelude::*};

#[macro_use]
extern crate tracing;

mod commands;
mod config;
mod handlers;
mod helpers;

struct Data {
    started_at: u64,
    /// map of channel IDs to the ID of the last user to message there
    channels: Mutex<HashMap<u64, MessageBuffer<10>>>,
    game_config: config::GameConfig,
    db_pool: SqlitePool,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!(
                "Logged in as {}#{}",
                data_about_bot.user.name,
                data_about_bot
                    .user
                    .discriminator
                    .map(|d| d.get())
                    .unwrap_or(0)
            );
        }
        serenity::FullEvent::Message { new_message } => {
            handlers::new_message(ctx, data, new_message).await?
        }
        _ => (),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;

    let filter = filter::Targets::new().with_target("patient_zero", Level::TRACE);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(filter)
        .init();

    let config = config::load(&Path::new("./pzero.toml")).expect("pzero.toml not found!");

    let pool = SqlitePool::connect(&config.bot.db_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MODERATION;

    let framework = poise::Framework::<Data, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::ping()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    started_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    game_config: config.game,
                    channels: Mutex::new(HashMap::new()),
                    db_pool: pool,
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(config.bot.token, intents)
        .framework(framework)
        .await
        .unwrap();

    Ok(client.start().await?)
}
