#[cfg(feature = "dotenv")]
use dotenvy::dotenv;
use std::{env, error::Error, sync::Arc, time::Duration};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_mention::Mention;
use twilight_model::id::{
    marker::{ChannelMarker, RoleMarker, UserMarker},
    Id,
};

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
    let cache = Arc::new(
        InMemoryCache::builder()
            .resource_types(ResourceType::VOICE_STATE)
            .build(),
    );

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
        tokio::spawn(handle_event(event, Arc::clone(&http), Arc::clone(&cache)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
    cache: Arc<InMemoryCache>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::Ready(_) => {
            println!("Shard is ready");
        }
        Event::VoiceStateUpdate(state) => {
            let Some(channel_id) = state.channel_id else {
                return Ok(());
            };
            let voice_channel_id: Id<ChannelMarker> =
                Id::new(env::var("VOICE_CHANNEL_ID")?.parse().unwrap());
            // 指定したチャンネル以外のボイスチャンネルに入ったら何もしない
            if channel_id != voice_channel_id {
                return Ok(());
            }
            // ボイスチャンネルの人数が1人の場合処理を続ける
            let member_count = cache
                .voice_channel_states(voice_channel_id)
                .unwrap()
                .count();
            if member_count != 1 {
                return Ok(());
            }

            // 10秒後にまだそのユーザーが参加していたらメッセージを送信する
            tokio::time::sleep(Duration::from_secs(10)).await;
            let Some(guild_id) = state.guild_id else {
                return Ok(());
            };
            let Some(current_state) = cache.voice_state(state.0.user_id, guild_id) else {
                return Ok(());
            };
            // ユーザーが指定したチャンネルのボイスチャンネルに入ってるか確認
            if current_state.channel_id() != voice_channel_id {
                return Ok(());
            };

            create_join_message(state.0.user_id, http).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn create_join_message(
    user_id: Id<UserMarker>,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let text_channel_id: Id<ChannelMarker> = Id::new(env::var("TEXT_CHANNEL_ID")?.parse().unwrap());
    let role_id: Id<RoleMarker> = Id::new(env::var("ROLE_ID")?.parse().unwrap());
    let message: String = format!(
        "{}の皆さん{}は暇です！ 誰かカモン〜ヌ！",
        role_id.mention(),
        user_id.mention()
    );
    http.create_message(text_channel_id)
        .content(&message)?
        .await?;
    Ok(())
}
