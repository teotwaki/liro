use super::{Context, Error, Result};
use poise::{CreateReply, serenity_prelude::EditInteractionResponse};
use std::{future::Future, sync::atomic::Ordering};

/// Acknowledge a slash command before polling work that can block on external services.
pub(super) async fn respond(
    ctx: Context<'_>,
    work: impl Future<Output = Result<CreateReply>>,
) -> Result<()> {
    if let Context::Application(application) = ctx {
        application.defer_response(ctx.command().ephemeral).await?;
    }
    let reply = work.await?;
    send_reply(ctx, reply).await
}

pub(in crate::bot) async fn send_command_error(ctx: Context<'_>, error: &Error) -> Result<()> {
    error!(
        "Error handling {}: {}",
        ctx.command().name,
        error.diagnostic()
    );
    send_reply(ctx, error_reply()).await
}

fn error_reply() -> CreateReply {
    CreateReply::default().content("Internal bot error. Please try again later.")
}

/// Once deferred, edit the original rather than sending a follow-up. This keeps its visibility.
async fn send_reply(ctx: Context<'_>, reply: CreateReply) -> Result<()> {
    if let Context::Application(application) = ctx
        && application.has_sent_initial_response.load(Ordering::SeqCst)
    {
        let edit = original_edit(ctx.reply_builder(reply));
        application
            .interaction
            .edit_response(ctx.http(), edit)
            .await?;
    } else {
        ctx.send(reply).await?;
    }
    Ok(())
}

fn original_edit(reply: CreateReply) -> EditInteractionResponse {
    reply.to_slash_initial_response_edit(EditInteractionResponse::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use poise::serenity_prelude::{CreateAllowedMentions, CreateEmbed};

    #[test]
    fn deferred_error_edit_has_generic_text_and_does_not_change_visibility() {
        let reply = error_reply()
            .ephemeral(true)
            .allowed_mentions(CreateAllowedMentions::new());
        let edit = serde_json::to_value(original_edit(reply)).unwrap();
        assert_eq!(
            edit["content"],
            "Internal bot error. Please try again later."
        );
        assert_eq!(edit["allowed_mentions"]["parse"], serde_json::json!([]));
        assert!(edit.get("flags").is_none());
    }

    #[test]
    fn deferred_success_edit_can_include_an_embed_without_changing_visibility() {
        let embed = CreateReply::default()
            .ephemeral(false)
            .embed(CreateEmbed::new().title("Ratings"));
        let embed = serde_json::to_value(original_edit(embed)).unwrap();
        assert_eq!(embed["embeds"][0]["title"], "Ratings");
        assert!(embed.get("flags").is_none());
    }
}
