use std::time::Duration;

use color_eyre::Result;
use poise::CreateReply;
use serenity::all::{Member, UserId};

use crate::{
    helpers,
    models::{InfectionEvent, InfectionRecord},
};

/// Replies with the current latency and uptime of Patient Zero.
#[poise::command(slash_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn ping(
    ctx: crate::Context<'_>,
    #[description = "Whether to show uptime"] detailed: Option<bool>,
) -> Result<()> {
    let detailed = detailed.unwrap_or(false);

    let ping = match ctx.ping().await {
        Duration::ZERO => "not yet available".to_string(),
        n => format!("{}ms", n.as_millis()),
    };

    let details = match detailed {
        true => format!(
            " Process started at <t:{0}:f>, <t:{0}:R>.",
            ctx.data().started_at
        ),
        false => "".to_string(),
    };

    ctx.send(
        CreateReply::default()
            .content(format!("Pong! My ping is {}.{}", ping, details))
            .reply(true)
            .ephemeral(detailed),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn infect(ctx: crate::Context<'_>, target: Member) -> Result<()> {
    let data = ctx.data();

    // set last action to now so we don't have a chain reaction of infections
    let player_id = target.user.id.get().to_string();
    let player = sqlx::query!(
        r#"
        INSERT INTO players (id, infected) VALUES (?, true)
        ON CONFLICT (id) DO UPDATE SET infected = true, last_action = unixepoch()
        RETURNING total_messages, sanitized_messages
        "#,
        player_id,
    )
    .fetch_one(&data.db_pool)
    .await?;

    let author_id = ctx.author().id.get().to_string();
    InfectionRecord {
        event: InfectionEvent::Infected,
        target: player_id,
        source: author_id.clone(),
        reason: format!("Manually infected by <@{}>", author_id),
        recorded_at: helpers::now() as i64,
        target_messages: player.total_messages,
        target_sanitized_messages: player.sanitized_messages,
    }
    .save(&data.db_pool)
    .await?;

    target
        .add_role(ctx.http(), data.game_config.infected_role)
        .await?;

    Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn cure(ctx: crate::Context<'_>, target: Member) -> Result<()> {
    let data = ctx.data();

    let player_id = target.user.id.get().to_string();
    let player = sqlx::query!(
        r#"
        INSERT INTO players (id, infected) VALUES (?, false)
        ON CONFLICT (id) DO UPDATE SET infected = false, last_action = unixepoch()
        RETURNING total_messages, sanitized_messages
        "#,
        player_id,
    )
    .fetch_one(&data.db_pool)
    .await?;

    let author_id = ctx.author().id.get().to_string();
    InfectionRecord {
        event: InfectionEvent::Cured,
        target: player_id,
        source: author_id.clone(),
        reason: format!("Manually cured by <@{}>", author_id),
        recorded_at: helpers::now() as i64,
        target_messages: player.total_messages,
        target_sanitized_messages: player.sanitized_messages,
    }
    .save(&data.db_pool)
    .await?;

    target
        .remove_role(ctx.http(), data.game_config.infected_role)
        .await?;

    Ok(())
}
