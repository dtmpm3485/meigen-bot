use crate::{
    commands,
    cooldown::Cooldowns,
    database::{Database, Quote},
    detector,
};
use anyhow::Result;
use serenity::{all::*, async_trait};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{watch, Mutex, Semaphore};

pub struct State {
    pub db: Database,
    pub mutations: Mutex<()>,
    pub commands: Semaphore,
    cooldowns: Mutex<Cooldowns>,
    messages: Semaphore,
    registered: AtomicBool,
}
struct Handler(Arc<State>);
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!(username=%ready.user.name,"logged in");
        if !self.0.registered.swap(true, Ordering::SeqCst) {
            // Upsert each command, preserving commands belonging to other integrations.
            for command in commands::definitions() {
                if let Err(e) = Command::create_global_command(&ctx.http, command).await {
                    self.0.registered.store(false, Ordering::SeqCst);
                    tracing::warn!(error=%e,"failed to register slash command");
                }
            }
        }
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(c) = interaction {
            commands::handle(&ctx, &c, &self.0).await;
        }
    }
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot
            || msg.webhook_id.is_some()
            || msg.guild_id.is_none()
            || !matches!(msg.kind, MessageType::Regular | MessageType::InlineReply)
        {
            return;
        }
        let a = detector::analyze_message(&msg.content);
        if a.score < 65 {
            return;
        }
        let Ok(_permit) = self.0.messages.try_acquire() else {
            return;
        };
        if let Err(e) = process(&ctx, &msg, a, &self.0).await {
            tracing::warn!(error=%e,"message processing failed");
        }
    }
}
async fn process(
    ctx: &Context,
    msg: &Message,
    a: detector::AnalysisResult,
    state: &State,
) -> Result<()> {
    let Some(guild) = msg.guild_id else {
        return Ok(());
    };
    let _guard = state.mutations.lock().await;
    let settings = state.db.settings(guild.get()).await?;
    if !settings.enabled
        || a.score < settings.threshold
        || settings.excluded_channels.contains(&msg.channel_id.get())
    {
        return Ok(());
    }
    let source = match msg.channel_id.to_channel(&ctx.http).await? {
        Channel::Guild(c) => c,
        _ => return Ok(()),
    };
    // Threads can inherit membership-sensitive access. This release deliberately excludes them.
    if !matches!(source.kind, ChannelType::Text | ChannelType::News) {
        return Ok(());
    }
    let normalized = detector::normalize(&msg.content);
    if !state.cooldowns.lock().await.claim(
        guild.get(),
        msg.author.id.get(),
        msg.id.get(),
        normalized.clone(),
        settings.user_cooldown,
        Instant::now(),
    ) {
        return Ok(());
    }
    let q = Quote::from_analysis(
        [
            msg.id.get(),
            guild.get(),
            msg.channel_id.get(),
            msg.author.id.get(),
        ],
        msg.author.name.clone(),
        msg.content.clone(),
        msg.timestamp.unix_timestamp(),
        a,
    );
    let mut output = msg.channel_id;
    if let Some(id) = settings
        .output_channel_id
        .filter(|id| *id != msg.channel_id.get())
    {
        let partial = guild.to_partial_guild(&ctx.http).await?;
        let everyone = partial.roles.get(&RoleId::new(guild.get()));
        let mut public = everyone.is_some_and(|r| {
            r.permissions
                .contains(Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY)
        });
        for overwrite in &source.permission_overwrites {
            if overwrite
                .deny
                .intersects(Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY)
            {
                public = false;
            }
        }
        if public {
            output = ChannelId::new(id);
        }
    }
    if !state.db.insert(q.clone(), normalized).await? {
        return Ok(());
    }
    let mut builder = CreateMessage::new()
        .content(format!(
            "名言を検出しました\n「{}」",
            commands::safe_text(&q.content)
        ))
        .allowed_mentions(CreateAllowedMentions::new().replied_user(false));
    if output == msg.channel_id {
        builder = builder.reference_message(MessageReference::from(msg).fail_if_not_exists(false));
    }
    if let Err(e) = output.send_message(&ctx.http, builder).await {
        tracing::warn!(error=%e,"failed to send meigen embed; quote remains saved");
    }
    Ok(())
}
pub async fn start(
    token: String,
    database: PathBuf,
    mut stop: watch::Receiver<bool>,
) -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,meigen_core=info".into()),
        )
        .try_init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting meigen-bot");
    let setup = async {
        let db = Database::open(database).await?;
        let state = Arc::new(State {
            db,
            mutations: Mutex::new(()),
            commands: Semaphore::new(8),
            cooldowns: Mutex::new(Cooldowns::default()),
            messages: Semaphore::new(16),
            registered: AtomicBool::new(false),
        });
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;
        let client = Client::builder(token, intents)
            .event_handler(Handler(state.clone()))
            .await?;
        Ok::<_, anyhow::Error>((client, state))
    };
    let (mut client, state) = tokio::select! {r=setup=>r?,_ = stop.changed()=>return Ok(())};
    if *stop.borrow() {
        return Ok(());
    }
    tracing::info!("connecting to Discord");
    let shards = client.shard_manager.clone();
    let cleanup = async {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            state.cooldowns.lock().await.prune(Instant::now());
        }
    };
    tokio::select! {
        result=client.start_autosharded()=>{result?;}
        _=stop.changed()=>{
            let _=tokio::time::timeout(Duration::from_secs(3),shards.shutdown_all()).await;
        }
        _=cleanup=>{}
    }
    Ok(())
}
