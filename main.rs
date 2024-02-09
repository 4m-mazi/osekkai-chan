#[cfg(feature = "dotenv")]
use dotenvy::dotenv;
use poise::serenity_prelude::{
    self as serenity, CacheHttp, ChannelId, Mentionable as _, RoleId, UserId,
};

use std::{env, str::FromStr, time::Duration};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() {
    #[cfg(feature = "dotenv")]
    dotenv().expect(".env file not found");
    let token = get_env("DISCORD_TOKEN");

    // Specify intents requesting events about things like new and updated
    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::GUILD_VOICE_STATES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .setup(|_, _, _| Box::pin(async { Ok(()) }))
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, ()| {
                Box::pin(event_handler(ctx, event, framework))
            },
            ..Default::default()
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

fn get_env(key: &str) -> String {
    env::var(key).expect("Missing `{key}` env var, see README for more information.")
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, (), Error>,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }

        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            // old があるかつ old channel と new channel が一致していたら何もしない
            if old
                .as_ref()
                .is_some_and(|old| old.channel_id == new.channel_id)
            {
                return Ok(());
            }

            let Some(new_channel_id) = new.channel_id else {
                return Ok(());
            };

            // 指定したチャンネル以外のボイスチャンネルに入ったら何もしない
            if get_env("VOICE_CHANNEL_ID") != new_channel_id.to_string() {
                return Ok(());
            }

            let Some(new_channel) = new_channel_id
                .to_channel(&ctx.http)
                .await
                .ok()
                .and_then(|c| c.guild())
            else {
                return Ok(());
            };

            // ボイスチャンネルの人数が1人の場合処理を続ける
            match new_channel.members(&ctx.cache) {
                Ok(m) => {
                    if m.len() != 1 {
                        return Ok(());
                    }
                }
                _ => return Ok(()),
            }

            // 10秒後にまだそのユーザーが参加していたらメッセージを送信する
            tokio::time::sleep(Duration::from_secs(10)).await;

            // ユーザーが指定したチャンネルのボイスチャンネルに入ってるか確認
            match new_channel.members(&ctx.cache) {
                Ok(m) => {
                    if m.iter().all(|m| m.user.id != new.user_id) {
                        return Ok(());
                    }
                }
                _ => return Ok(()),
            }

            create_join_message(new.user_id, &ctx.http).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn create_join_message(user_id: UserId, cache_http: impl CacheHttp) -> Result<(), Error> {
    let text_channel_id = ChannelId::from_str(get_env("TEXT_CHANNEL_ID").as_str()).unwrap();
    let role_id = RoleId::from_str(get_env("ROLE_ID").as_str()).unwrap();
    let message = format!(
        "{}の皆さん{}は暇です！ 誰かカモン〜ヌ！",
        role_id.mention(),
        user_id.mention()
    );
    text_channel_id.say(cache_http, message).await?;

    Ok(())
}
