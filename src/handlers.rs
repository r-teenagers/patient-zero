use poise::serenity_prelude as serenity;

use crate::{
    helpers::{self, MessageBuffer},
    models::{InfectionEvent, InfectionRecord, Player},
};

pub async fn new_message(
    ctx: &serenity::Context,
    data: &crate::Data,
    msg: &serenity::Message,
) -> Result<(), crate::Error> {
    if msg.author.bot {
        return Ok(());
    }

    let player_id = msg.author.id.to_string();

    // If the player's already cached to avoid hitting the db
    // TODO: extract to functions to ensure sync between cache and db or remove?
    let player_is_infected = match data.player_cache.get(&msg.author.id.get()).await {
        Some(b) => *b.lock().await,
        None => sqlx::query!("SELECT (infected) FROM players WHERE id = ?", player_id)
            .fetch_optional(&data.db_pool)
            .await?
            .map_or(false, |p| p.infected),
    };

    if player_is_infected {
        trace!("player is already infected, checking if they need to be cured");
        return handle_cure(data, player_id);
    }

    trace!("player is not infected, checking if they should be");

    let last_author = {
        let buf = data.channels.get_or_insert(&msg.channel_id.get()).await;
        let mut buf = buf.lock().await;
        let last_author = buf.get_last();
        buf.push(msg.author.id.get(), msg.id.get());
        last_author
    };

    // last_author may not actually exist if the message was sent before the bot started;
    // if not, they cannot possibly be infected anyway
    let author_data = match last_author {
        Some(a) => {
            let a = a.to_string();
            sqlx::query_as!(Player, "SELECT * FROM players WHERE id = ?", a)
                .fetch_optional(&data.db_pool)
                .await?
        }
        None => None,
    };

    let should_infect = match author_data {
        Some(ref a) => a.infected,
        None => false,
    };

    let total_messages = sqlx::query!(
        r#"
        INSERT INTO players (id, infected, total_messages) VALUES (?, ?, 1)
        ON CONFLICT (id) DO 
        UPDATE SET total_messages = total_messages + 1 WHERE id = ?
        RETURNING (total_messages)
        "#,
        player_id,
        should_infect,
        player_id,
    )
    .fetch_one(&data.db_pool)
    .await?
    .total_messages;

    trace!("player {} has {} messages", player_id, total_messages);

    if should_infect {
        trace!("infecting player {}", player_id);
        let author_data = author_data.unwrap();
        InfectionRecord {
            event: InfectionEvent::Infected,
            target: player_id,
            source: author_data.id,
            reason: "Talked below an infected player".to_string(),
            recorded_at: helpers::now() as i64,
            target_messages: total_messages,
        }
        .save(&data.db_pool)
        .await?;
    }

    Ok(())
}

async fn handle_cure(data: &crate::Data, player_id: String) -> Result<(), crate::Error> {
    warn!("TODO: check for cure");
    Ok(())
}
