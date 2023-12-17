#[cfg(feature = "dotenv")]
use dotenvy::dotenv;
use std::{env, error::Error, sync::Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_mention::Mention;
use twilight_model::id::{Id, marker::RoleMarker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "dotenv")]
    dotenv().expect(".env file not found");
    let token: String = env::var("DISCORD_TOKEN")?;

    // Specify intents requesting events about things like new and updated
    // messages in a guild and direct messages.
    let intents = Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;

    // Create a single shard.
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);

    // The http client is separate from the gateway, so startup a new
    // one, also use Arc such that it can be cloned to other threads.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about messages, make the cache only process messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Startup the event loop to process each event in the event stream as they
    // come in.
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");

                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };
        // Update the cache.
        cache.update(&event);

        // Spawn a new task to handle the event
        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) if msg.content == "!ping" => {
            http.create_message(msg.channel_id).content("Pong!")?.await?;
        }
        Event::Ready(_) => {
            println!("Shard is ready");
        }
        Event::VoiceStateUpdate(state) => {
            let voice_channel_id: String = env::var("VOICE_CHANNEL_ID")?;
            if state.channel_id.expect("Channel ID not found.").to_string() != voice_channel_id {
                return Ok(());
            }
            let text_channel_id = Id::new(env::var("TEXT_CHANNEL_ID")?.parse().unwrap());
            let role_id: Id<RoleMarker> = Id::new(env::var("ROLE_ID")?.parse().unwrap());
            let message: String = format!("{}の皆さん{}は暇です！ 誰かカモン〜ヌ！", role_id.mention(), state.user_id.mention());
            http.create_message(text_channel_id).content(&message)?.await?;
        }
        _ => {}
    }

    Ok(())
}
