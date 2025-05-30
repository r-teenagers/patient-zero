use ::serenity::all::{CacheHttp, GuildId, UserId};
use color_eyre::Result;
use poise::serenity_prelude as serenity;

use crate::{
    helpers::{self},
    models::{InfectionEvent, InfectionRecord},
};

pub async fn new_message(
    ctx: &serenity::Context,
    data: &crate::Data,
    msg: &serenity::Message,
) -> Result<()> {
    if msg.author.bot {
        return Ok(());
    }

    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };

    let player_id = msg.author.id.to_string();

    // inserts a new player or adds to the previous player's message count **only if** the cooldown
    // has passed
    // maybe should just be handled in rust for cleanliness' sake?
    let player = sqlx::query!(
        r#"
        INSERT INTO players (id, total_messages, sanitized_messages) VALUES (?, 1, 1)
        ON CONFLICT (id) DO UPDATE SET
            total_messages = total_messages + 1,
            sanitized_messages =
                CASE WHEN unixepoch() - last_action > ?
                THEN sanitized_messages + 1
                ELSE sanitized_messages END,
            last_action =
                CASE WHEN unixepoch() - last_action > ?
                THEN unixepoch()
                ELSE last_action END
        RETURNING total_messages, sanitized_messages
        "#,
        player_id,
        data.game_config.message_cooldown,
        data.game_config.message_cooldown,
    )
    .fetch_one(&data.db_pool)
    .await?;

    trace!(
        "player {} has {} messages ({} sanitized)",
        player_id, player.total_messages, player.sanitized_messages,
    );

    // TODO: add a cache for player infection state?
    let player_is_infected = sqlx::query!("SELECT infected FROM players WHERE id = ?", player_id)
        .fetch_optional(&data.db_pool)
        .await?
        .is_some_and(|p| p.infected);

    if player_is_infected {
        trace!("player is already infected, checking if they need to be cured");
        return check_cure(ctx, data, msg.author.id, guild_id).await;
    }

    trace!("player is not infected, checking if they should be");

    let last_message = {
        let buf = data.channels.get_or_insert(&msg.channel_id.get()).await;
        let mut buf = buf.lock().await;
        let last_message = buf.get_last_message();
        buf.push(
            msg.author.id.get(),
            msg.id.get(),
            // why can't people settle on a standard type for unix timestamps :/
            msg.timestamp.unix_timestamp().try_into().unwrap(),
        );
        last_message
    };

    // last_message may not actually exist if the message was sent before the bot started;
    // if not, they cannot possibly be infected anyway
    let author_data = match last_message {
        Some(m) => {
            let a = m.0.to_string();
            sqlx::query!("SELECT id, infected FROM players WHERE id = ?", a)
                .fetch_optional(&data.db_pool)
                .await?
        }
        None => None,
    };

    // only infect the player if the previous message is infected *and* they haven't infected
    // anyone within the cooldown
    let should_infect = match author_data {
        Some(ref a) if a.infected => sqlx::query!(
            "SELECT recorded_at FROM infection_records WHERE source = ? ORDER BY recorded_at DESC",
            a.id
        )
        .fetch_optional(&data.db_pool)
        .await?
        .is_some_and(|r| {
            helpers::now() - r.recorded_at as u64 > data.game_config.infection_cooldown as u64
        }),
        _ => false,
    };

    if should_infect {
        let author_data = author_data.unwrap();
        info!("Player {} infected by {}", player_id, author_data.id);

        // TODO: possibly just combine with the above query for updating message count
        sqlx::query!("UPDATE players SET infected = true WHERE id = ?", player_id)
            .execute(&data.db_pool)
            .await?;

        InfectionRecord {
            event: InfectionEvent::Infected,
            target: player_id,
            source: Some(author_data.id.clone()),
            reason: Some(format!("Infected by proximity to <@{}>", author_data.id)),
            recorded_at: helpers::now() as i64,
            target_total_messages: player.total_messages,
            target_sanitized_messages: player.sanitized_messages,
        }
        .save(&data.db_pool)
        .await?;

        guild_id
            .member(ctx.http(), msg.author.id)
            .await?
            .add_role(ctx.http(), data.game_config.infected_role)
            .await?;
    }

    Ok(())
}

async fn check_cure(
    ctx: &serenity::Context,
    data: &crate::Data,
    player_id: UserId,
    guild_id: GuildId,
) -> Result<()> {
    let player_id_str = player_id.to_string();
    let player = sqlx::query!("SELECT * FROM players WHERE id = ?", player_id_str)
        .fetch_one(&data.db_pool)
        .await?;

    let action = sqlx::query!(
        "SELECT recorded_at, target_sanitized_messages FROM infection_records WHERE target = ? AND event = 'infected' ORDER BY recorded_at DESC",
        player_id_str
    )
    .fetch_one(&data.db_pool)
    .await?;

    // FIXME: move timeout checking out of this function - just sweep every few minutes instead?
    let cure_reason = if player.sanitized_messages - action.target_sanitized_messages
        > data.game_config.cure_threshold.into()
    {
        format!(
            "Sent {} messages while infected",
            data.game_config.cure_threshold
        )
    } else if data
        .game_config
        .cure_timeout
        .is_some_and(|t| helpers::now() - action.recorded_at as u64 > t)
    {
        format!(
            "Was infected for more than {} seconds",
            data.game_config.cure_timeout.unwrap()
        )
    } else {
        return Ok(());
    };

    info!("Player {} cured", player_id);

    // TODO: possibly just combine with the above query for updating message count
    sqlx::query!(
        "UPDATE players SET infected = true WHERE id = ?",
        player_id_str
    )
    .execute(&data.db_pool)
    .await?;

    InfectionRecord {
        event: InfectionEvent::Infected,
        target: player_id_str,
        source: None,
        reason: Some(cure_reason),
        recorded_at: helpers::now() as i64,
        target_total_messages: player.total_messages,
        target_sanitized_messages: player.sanitized_messages,
    }
    .save(&data.db_pool)
    .await?;

    guild_id
        .member(ctx.http(), player_id)
        .await?
        .add_role(ctx.http(), data.game_config.infected_role)
        .await?;

    Ok(())
}
