pub enum Command<'a> {
    Rating(u64),
    ConnectAccount(u64),
    Unknown(&'a str),
    Ignore,
}

impl<'a> Command<'a> {
    pub async fn parse(msg: &'a serenity::model::channel::Message) -> Command<'a> {
        if msg.content.starts_with("ohnomy") {
            let command = &msg.content[7..];

            match command {
                "rating" => {
                    let discord_id = *msg.author.id.as_u64();
                    debug!(
                        "Successfully parsed rating command (discord_id={})",
                        discord_id
                    );
                    Command::Rating(discord_id)
                }
                "account" => {
                    let discord_id = *msg.author.id.as_u64();
                    debug!(
                        "Successfully parsed account command (discord_id={})",
                        discord_id
                    );
                    Command::ConnectAccount(discord_id)
                }
                _ => {
                    debug!("Could not understand command `{}`", msg.content);
                    Command::Unknown(command)
                }
            }
        } else {
            debug!("Ignoring message that doesn't look like a command");
            Command::Ignore
        }
    }
}
