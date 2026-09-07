use crate::{
    bot::State,
    database::{Order, Quote},
};
use anyhow::{Context as _, Result};
use serenity::all::*;

pub fn definitions() -> Vec<CreateCommand> {
    let base = |name: &str, description: &str| {
        CreateCommand::new(name)
            .description(description)
            .contexts(vec![InteractionContext::Guild])
    };
    let user = || {
        CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "対象ユーザー（省略時は自分）",
        )
    };
    vec![
        base("meigen", "最近の名言（閲覧できるチャンネルのみ）"),
        base("meigen-user", "指定ユーザーの最近の名言").add_option(user().required(true)),
        base("meigen-random", "ランダムに名言を表示"),
        base("meigen-top", "名言度の歴代トップ5"),
        base("ranking", "名言数ランキング"),
        base("profile", "名言プロフィール").add_option(user()),
        base(
            "delete-my-data",
            "このBotの全サーバーに保存された自分の名言を削除",
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "confirm",
                "全サーバーの自分の保存データを削除する",
            )
            .required(true),
        ),
        base(
            "meigen-settings",
            "管理者向け検出設定（省略した項目は維持）",
        )
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "enabled",
            "検出ON/OFF",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "threshold", "反応閾値65〜95")
                .min_int_value(65)
                .max_int_value(95),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "cooldown",
                "ユーザークールダウン秒0〜86400",
            )
            .min_int_value(0)
            .max_int_value(86400),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "output",
                "公開チャンネルの名言の出力先",
            )
            .channel_types(vec![ChannelType::Text]),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "reset-output",
            "出力先を元チャンネルに戻す",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "exclude",
                "検出から除外するチャンネル",
            )
            .channel_types(vec![ChannelType::Text, ChannelType::News]),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "include",
                "除外を解除するチャンネル",
            )
            .channel_types(vec![ChannelType::Text, ChannelType::News]),
        ),
    ]
}
pub fn safe_text(s: &str) -> String {
    // Quote user text literally without letting it create links, headings or mentions.
    let mut out = String::new();
    for c in s.chars() {
        if ['\\', '*', '_', '~', '`', '[', ']', '(', ')', '>', '#', '|'].contains(&c) {
            out.push('\\');
        }
        out.push(c);
        if c == '@' {
            out.push('\u{200b}');
        }
    }
    out
}
pub fn quote_embed(q: &Quote, avatar: Option<&str>) -> CreateEmbed {
    let band = match q.score {
        95..=100 => "🌟 伝説の名言",
        85..=94 => "📜 強い名言",
        75..=84 => "📜 名言を検出しました",
        _ => "📜 名言候補",
    };
    let mut e = CreateEmbed::new()
        .title(band)
        .description(format!("「{}」", safe_text(&q.content)))
        .color(0xd9aa52)
        .field("名言度", q.score.to_string(), true)
        .field("深そう度", q.deep_score.to_string(), true)
        .field("ネタ度", q.funny_score.to_string(), true)
        .field("説得力", q.persuasion_score.to_string(), true)
        .field("人生終わってる度", q.degenerate_score.to_string(), true)
        .field("ジャンル", q.genres.join(" / "), false)
        .field("元メッセージ", format!("[ジャンプ]({})", q.link()), false)
        .author(CreateEmbedAuthor::new(format!("— {}", q.username)))
        .footer(CreateEmbedFooter::new(
            "ルールによる遊びの評価です / 保存データ削除: /delete-my-data",
        ));
    if let Some(url) = avatar {
        e = e.thumbnail(url);
    }
    if let Ok(ts) = Timestamp::from_unix_timestamp(q.created_at) {
        e = e.timestamp(ts);
    }
    e
}
fn option<'a>(c: &'a CommandInteraction, name: &str) -> Option<&'a CommandDataOptionValue> {
    c.data
        .options
        .iter()
        .find(|o| o.name == name)
        .map(|o| &o.value)
}
async fn respond(ctx: &Context, c: &CommandInteraction, text: String) -> Result<()> {
    c.edit_response(
        &ctx.http,
        EditInteractionResponse::new()
            .content(text)
            .allowed_mentions(CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}
async fn execute(ctx: &Context, c: &CommandInteraction, state: &State) -> Result<()> {
    let guild = c.guild_id.context("サーバー内で実行してください")?;
    let user = option(c, "user")
        .and_then(CommandDataOptionValue::as_user_id)
        .unwrap_or(c.user.id);
    if c.data.name == "delete-my-data" {
        if option(c, "confirm").and_then(CommandDataOptionValue::as_bool) != Some(true) {
            return respond(
                ctx,
                c,
                "削除しませんでした。削除する場合は confirm:True を選んでください。".into(),
            )
            .await;
        }
        // Serialize with ingestion so a message already being processed cannot reappear after deletion.
        let _guard = state.mutations.lock().await;
        let n = state.db.delete_user(c.user.id.get()).await?;
        return respond(ctx,c,format!("全サーバーの自分の保存済み名言を{n}件削除しました。BotがDiscordへ投稿済みのEmbedや管理者のバックアップは対象外です。今後の新しい名言は再び保存されます。")).await;
    }
    let partial = guild.to_partial_guild(&ctx.http).await?;
    let member = guild.member(&ctx.http, c.user.id).await?;
    let channels = guild.channels(&ctx.http).await?;
    if c.data.name == "meigen-settings" {
        let admin = partial.owner_id == c.user.id
            || member.roles.iter().any(|id| {
                partial
                    .roles
                    .get(id)
                    .is_some_and(|r| r.permissions.administrator())
            });
        anyhow::ensure!(admin, "サーバー管理者のみ変更できます");
        let _guard = state.mutations.lock().await;
        let mut s = state.db.settings(guild.get()).await?;
        if let Some(v) = option(c, "enabled").and_then(CommandDataOptionValue::as_bool) {
            s.enabled = v;
        }
        if let Some(v) = option(c, "threshold").and_then(CommandDataOptionValue::as_i64) {
            anyhow::ensure!((65..=95).contains(&v), "閾値は65〜95です");
            s.threshold = v as u8;
        }
        if let Some(v) = option(c, "cooldown").and_then(CommandDataOptionValue::as_i64) {
            anyhow::ensure!((0..=86400).contains(&v), "秒数は0〜86400です");
            s.user_cooldown = v as u64;
        }
        if let Some(id) = option(c, "output").and_then(CommandDataOptionValue::as_channel_id) {
            let channel = channels
                .get(&id)
                .context("同じサーバーのチャンネルを選択してください")?;
            anyhow::ensure!(
                channel.kind == ChannelType::Text,
                "テキストチャンネルを選択してください"
            );
            let bot = ctx.http.get_current_user().await?;
            let bot_member = guild.member(&ctx.http, bot.id).await?;
            let perms = partial.user_permissions_in(channel, &bot_member);
            anyhow::ensure!(
                perms.contains(
                    Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::EMBED_LINKS
                ),
                "出力先でBotの閲覧・送信・埋め込み権限が必要です"
            );
            s.output_channel_id = Some(id.get());
        }
        if option(c, "reset-output").and_then(CommandDataOptionValue::as_bool) == Some(true) {
            s.output_channel_id = None;
        }
        if let Some(id) = option(c, "exclude").and_then(CommandDataOptionValue::as_channel_id) {
            anyhow::ensure!(
                channels.contains_key(&id),
                "同じサーバーのチャンネルを選択してください"
            );
            if !s.excluded_channels.contains(&id.get()) {
                s.excluded_channels.push(id.get());
            }
        }
        if let Some(id) = option(c, "include").and_then(CommandDataOptionValue::as_channel_id) {
            s.excluded_channels.retain(|x| *x != id.get());
        }
        state.db.save_settings(guild.get(), s.clone()).await?;
        return respond(ctx,c,format!("検出: {}\n閾値: {}\nクールダウン: {}秒\n出力先: {}\n除外: {}件\n別チャンネル転送は公開元チャンネルのみ。非公開元は転送せず、その場で返信します。",
            if s.enabled {"ON"} else {"OFF"},s.threshold,s.user_cooldown,
            s.output_channel_id.map_or("元チャンネル".into(),|n|format!("<#{n}>")),s.excluded_channels.len())).await;
    }
    // Resolve current permissions, never reuse stale visibility from when the quote was stored.
    let visible: Vec<String> = channels
        .values()
        .filter(|channel| {
            partial
                .user_permissions_in(channel, &member)
                .contains(Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY)
        })
        .map(|c| c.id.to_string())
        .collect();
    // Serialize retrieval with deletion until the response has been delivered.
    let _guard = state.mutations.lock().await;
    match c.data.name.as_str() {
        "ranking" => {
            let rows = state.db.ranking(guild.get(), visible).await?;
            let text = if rows.is_empty() {
                "名言はまだありません。".into()
            } else {
                rows.iter()
                    .enumerate()
                    .map(|(i, (id, n))| format!("{}. <@{}> — {}件", i + 1, id, n))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            respond(ctx, c, text).await
        }
        "profile" => {
            let p = state.db.profile(guild.get(), visible, user.get()).await?;
            respond(ctx,c,format!("<@{}>\n総名言数: {}\n最高名言度: {}\n平均名言度: {:.1}\n得意ジャンル: {}\n称号: {}\n※閲覧できるチャンネルのみ集計",
                user,p.count,p.best,p.average,p.genres.iter().map(|g|g.0.clone()).collect::<Vec<_>>().join(" / "),p.title())).await
        }
        "meigen" | "meigen-user" | "meigen-random" | "meigen-top" => {
            let order = match c.data.name.as_str() {
                "meigen-random" => Order::Random,
                "meigen-top" => Order::Top,
                _ => Order::Recent,
            };
            let quotes = state
                .db
                .quotes(
                    guild.get(),
                    visible,
                    if c.data.name == "meigen-user" {
                        Some(user.get())
                    } else {
                        None
                    },
                    order,
                )
                .await?;
            if quotes.is_empty() {
                return respond(ctx, c, "閲覧できる名言はまだありません。".into()).await;
            }
            // Compact entries keep five 300-character quotes below Discord's aggregate embed limit.
            let mut e = CreateEmbed::new().title("📜 名言集").color(0xd9aa52);
            for q in quotes {
                e = e.field(
                    format!("{}点 — {}", q.score, q.username),
                    format!("{}\n[元メッセージ]({})", safe_text(&q.content), q.link()),
                    false,
                );
            }
            c.edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(e)
                    .allowed_mentions(CreateAllowedMentions::new()),
            )
            .await?;
            Ok(())
        }
        _ => respond(ctx, c, "このコマンドは利用できません。".into()).await,
    }
}
pub async fn handle(ctx: &Context, c: &CommandInteraction, state: &State) {
    let Ok(_permit) = state.commands.try_acquire() else {
        let _ = c
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("処理が混み合っています。少し待ってお試しください。")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    };
    if c.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true)),
    )
    .await
    .is_err()
    {
        return;
    }
    if let Err(e) = execute(ctx, c, state).await {
        tracing::warn!(error=%e,"command failed");
        let _ = respond(
            ctx,
            c,
            "処理できませんでした。権限・設定・DBの状態を確認してください。".into(),
        )
        .await;
    }
}
