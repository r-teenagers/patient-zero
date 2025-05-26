use poise::serenity_prelude as serenity;

use crate::helpers::MessageBuffer;

pub async fn new_message(
    ctx: &serenity::Context,
    data: &crate::Data,
    msg: &serenity::Message,
) -> Result<(), crate::Error> {
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
        debug!("{:?}", buf);
        last_author
    };

    Ok(())
}
