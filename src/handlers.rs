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

    info!("new message {}", msg.content);

    // FIXME: maybe don't lock on every message if possible? or have per-channel locks?
    // this probably isn't slow enough to actually matter it's just really gross
    let last_author = {
        let mut channels = data.channels.lock().await;

        // gets a mutable reference or inserts and returns one
        // FIXME: i'm sleep deprived
        let buf: &mut MessageBuffer<10> = match channels.get_mut(&msg.channel_id.get()) {
            Some(buf) => buf,
            None => {
                let mb = MessageBuffer::new();
                channels.insert(msg.channel_id.get(), mb);
                channels.get_mut(&msg.channel_id.get()).unwrap()
            }
        };

        let last_author = buf.get_last();
        buf.push(msg.author.id.get(), msg.id.get());
        trace!("buffer for {}: {:?}", msg.channel_id, buf);
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

    let player_id = msg.author.id.to_string();
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

    debug!("player {} has {} messages", player_id, total_messages);

    if should_infect {
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
