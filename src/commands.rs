use std::time::Duration;

use poise::CreateReply;

/// Replies with the current latency and uptime of Patient Zero.
#[poise::command(slash_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn ping(
    ctx: crate::Context<'_>,
    #[description = "Whether to show uptime"] detailed: Option<bool>,
) -> Result<(), crate::Error> {
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
