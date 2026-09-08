use crate::{config::GuildSettings, detector::AnalysisResult};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Connection) + Send + 'static>;
#[derive(Clone)]
pub struct Database {
    sender: mpsc::Sender<Job>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub message_id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub score: u8,
    pub deep_score: u8,
    pub funny_score: u8,
    pub persuasion_score: u8,
    pub degenerate_score: u8,
    pub genres: Vec<String>,
    pub created_at: i64,
}
impl Quote {
    pub fn link(&self) -> String {
        format!(
            "https://discord.com/channels/{}/{}/{}",
            self.guild_id, self.channel_id, self.message_id
        )
    }
    pub fn from_analysis(
        ids: [u64; 4],
        username: String,
        content: String,
        created_at: i64,
        a: AnalysisResult,
    ) -> Self {
        Self {
            message_id: ids[0].to_string(),
            guild_id: ids[1].to_string(),
            channel_id: ids[2].to_string(),
            user_id: ids[3].to_string(),
            username,
            content,
            created_at,
            score: a.score,
            deep_score: a.deep_score,
            funny_score: a.funny_score,
            persuasion_score: a.persuasion_score,
            degenerate_score: a.degenerate_score,
            genres: a.genres,
        }
    }
}
#[derive(Debug)]
pub struct Profile {
    pub count: u64,
    pub best: u8,
    pub average: f64,
    pub deep: f64,
    pub funny: f64,
    pub degenerate: f64,
    pub genres: Vec<(String, u64)>,
}
impl Profile {
    pub fn title(&self) -> &'static str {
        if self.best >= 95 {
            "伝説の語録保持者"
        } else if self.count >= 100 {
            "歩く名言集"
        } else if self.genres.first().is_some_and(|g| g.0 == "学校") && self.degenerate >= 60.0 {
            "学校よりログボを選んだ者"
        } else if self.genres.first().is_some_and(|g| g.0 == "ゲーム") {
            "電子世界の哲学者"
        } else if self.deep >= 65.0 {
            "深夜の哲学者"
        } else if self.funny >= 65.0 {
            "迷言製造機"
        } else if self.count == 0 {
            "これからの名言家"
        } else {
            "日常の語り部"
        }
    }
}
#[derive(Clone, Copy)]
pub enum Order {
    Recent,
    Top,
    Random,
}
const COLUMNS:&str = "message_id,guild_id,channel_id,user_id,username,content,score,deep_score,funny_score,persuasion_score,degenerate_score,genres,created_at";
fn row_quote(r: &Row<'_>) -> rusqlite::Result<Quote> {
    let genres: String = r.get(11)?;
    Ok(Quote {
        message_id: r.get(0)?,
        guild_id: r.get(1)?,
        channel_id: r.get(2)?,
        user_id: r.get(3)?,
        username: r.get(4)?,
        content: r.get(5)?,
        score: r.get(6)?,
        deep_score: r.get(7)?,
        funny_score: r.get(8)?,
        persuasion_score: r.get(9)?,
        degenerate_score: r.get(10)?,
        genres: serde_json::from_str(&genres).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(e))
        })?,
        created_at: r.get(12)?,
    })
}
fn initialize(path: PathBuf) -> Result<Connection> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path).context("cannot open SQLite file")?;
    conn.busy_timeout(Duration::from_secs(3))?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    anyhow::ensure!(
        version <= 1,
        "database schema is newer than this bot supports"
    );
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON;")?;
    if version == 0 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS quotes (
            message_id TEXT PRIMARY KEY, guild_id TEXT NOT NULL, channel_id TEXT NOT NULL,
            user_id TEXT NOT NULL, username TEXT NOT NULL, content TEXT NOT NULL,
            score INTEGER NOT NULL CHECK(score BETWEEN 0 AND 100),
            deep_score INTEGER NOT NULL, funny_score INTEGER NOT NULL,
            persuasion_score INTEGER NOT NULL, degenerate_score INTEGER NOT NULL,
            genres TEXT NOT NULL, created_at INTEGER NOT NULL,
            normalized TEXT NOT NULL, received_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE INDEX IF NOT EXISTS idx_quotes_guild_time ON quotes(guild_id,created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_quotes_guild_user ON quotes(guild_id,user_id);
        CREATE INDEX IF NOT EXISTS idx_quotes_guild_score ON quotes(guild_id,score DESC);
        CREATE INDEX IF NOT EXISTS idx_quotes_user ON quotes(user_id);
        CREATE INDEX IF NOT EXISTS idx_quotes_channel ON quotes(guild_id,channel_id);
        CREATE INDEX IF NOT EXISTS idx_quotes_dedup ON quotes(guild_id,normalized,received_at);
        CREATE TABLE IF NOT EXISTS settings (
            guild_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 1,
            threshold INTEGER NOT NULL DEFAULT 75 CHECK(threshold BETWEEN 65 AND 95),
            user_cooldown INTEGER NOT NULL DEFAULT 60 CHECK(user_cooldown BETWEEN 0 AND 86400),
            output_channel_id TEXT, excluded_channels TEXT NOT NULL DEFAULT '[]'
        );
        PRAGMA user_version=1;",
        )?;
        tx.commit()?;
    }
    Ok(conn)
}
impl Database {
    pub async fn open(path: PathBuf) -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel::<Job>(128);
        let (ready_tx, ready_rx) = oneshot::channel();
        std::thread::Builder::new()
            .name("meigen-sqlite".into())
            .spawn(move || match initialize(path) {
                Ok(mut conn) => {
                    let _ = ready_tx.send(Ok(()));
                    while let Some(job) = receiver.blocking_recv() {
                        job(&mut conn);
                    }
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })?;
        ready_rx
            .await
            .context("database worker stopped during initialization")??;
        tracing::info!("database initialized");
        Ok(Self { sender })
    }
    pub async fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .try_send(Box::new(move |conn| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(conn)))
                    .unwrap_or_else(|_| Err(anyhow!("database operation panicked")));
                let _ = tx.send(result);
            }))
            .map_err(|_| anyhow!("database queue busy or closed"))?;
        rx.await.context("database worker stopped")?
    }
    pub async fn settings(&self, guild: u64) -> Result<GuildSettings> {
        self.call(move |c| {
            let raw=c.query_row("SELECT enabled,threshold,user_cooldown,output_channel_id,excluded_channels FROM settings WHERE guild_id=?1",
                [guild.to_string()], |r| Ok((r.get::<_,bool>(0)?,r.get::<_,u8>(1)?,r.get::<_,u64>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,String>(4)?))).optional()?;
            match raw {
                Some((enabled,threshold,user_cooldown,output,excluded))=> {
                    let s=GuildSettings{enabled,threshold,user_cooldown,output_channel_id:output.map(|s|s.parse()).transpose()?,excluded_channels:serde_json::from_str(&excluded)?};
                    s.validate()?; Ok(s)
                }
                None=>Ok(GuildSettings::default())
            }
        }).await
    }
    pub async fn save_settings(&self, guild: u64, s: GuildSettings) -> Result<()> {
        s.validate()?;
        self.call(move |c| {
            c.execute("INSERT INTO settings(guild_id,enabled,threshold,user_cooldown,output_channel_id,excluded_channels)
                VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(guild_id) DO UPDATE SET enabled=excluded.enabled,
                threshold=excluded.threshold,user_cooldown=excluded.user_cooldown,output_channel_id=excluded.output_channel_id,
                excluded_channels=excluded.excluded_channels",params![guild.to_string(),s.enabled,s.threshold,s.user_cooldown,
                s.output_channel_id.map(|n|n.to_string()),serde_json::to_string(&s.excluded_channels)?])?;
            Ok(())
        }).await
    }
    pub async fn insert(&self, q: Quote, normalized: String) -> Result<bool> {
        self.call(move |c| {
            let tx=c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM quotes WHERE message_id=?1 OR (guild_id=?2 AND normalized=?3 AND received_at>=unixepoch()-300))",
                params![q.message_id,q.guild_id,normalized],|r|r.get(0))?;
            if duplicate { return Ok(false); }
            tx.execute("INSERT INTO quotes(message_id,guild_id,channel_id,user_id,username,content,score,deep_score,funny_score,persuasion_score,degenerate_score,genres,created_at,normalized)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![q.message_id,q.guild_id,q.channel_id,q.user_id,q.username,q.content,q.score,
                q.deep_score,q.funny_score,q.persuasion_score,q.degenerate_score,serde_json::to_string(&q.genres)?,q.created_at,normalized])?;
            tx.commit()?; Ok(true)
        }).await
    }
    pub async fn quotes(
        &self,
        guild: u64,
        channels: Vec<String>,
        user: Option<u64>,
        order: Order,
    ) -> Result<Vec<Quote>> {
        self.call(move |c| {
            let order=match order {Order::Recent=>"created_at DESC,message_id DESC",Order::Top=>"score DESC,created_at DESC",Order::Random=>"random()"};
            let limit=if matches!(order,"random()") {1} else {5};
            let sql=format!("SELECT {COLUMNS} FROM quotes WHERE guild_id=?1 AND channel_id IN (SELECT value FROM json_each(?2)) AND (?3 IS NULL OR user_id=?3) ORDER BY {order} LIMIT {limit}");
            let mut stmt=c.prepare(&sql)?;
            let rows=stmt.query_map(params![guild.to_string(),serde_json::to_string(&channels)?,user.map(|n|n.to_string())],row_quote)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await
    }
    pub async fn ranking(&self, guild: u64, channels: Vec<String>) -> Result<Vec<(String, u64)>> {
        self.call(move |c| {
            let mut stmt=c.prepare("SELECT user_id,COUNT(*) AS n FROM quotes WHERE guild_id=?1 AND channel_id IN (SELECT value FROM json_each(?2)) GROUP BY user_id ORDER BY n DESC,user_id LIMIT 10")?;
            let rows=stmt.query_map(params![guild.to_string(),serde_json::to_string(&channels)?],|r|Ok((r.get(0)?,r.get(1)?)))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await
    }
    pub async fn profile(&self, guild: u64, channels: Vec<String>, user: u64) -> Result<Profile> {
        self.call(move |c| {
            let channels=serde_json::to_string(&channels)?;
            let mut p=c.query_row("SELECT COUNT(*),COALESCE(MAX(score),0),COALESCE(AVG(score),0),COALESCE(AVG(deep_score),0),COALESCE(AVG(funny_score),0),COALESCE(AVG(degenerate_score),0) FROM quotes WHERE guild_id=?1 AND user_id=?2 AND channel_id IN (SELECT value FROM json_each(?3))",
                params![guild.to_string(),user.to_string(),channels],|r|Ok(Profile{count:r.get(0)?,best:r.get(1)?,average:r.get(2)?,deep:r.get(3)?,funny:r.get(4)?,degenerate:r.get(5)?,genres:vec![]}))?;
            let mut stmt=c.prepare("SELECT j.value,COUNT(*) AS n FROM quotes q,json_each(q.genres) j WHERE q.guild_id=?1 AND q.user_id=?2 AND q.channel_id IN (SELECT value FROM json_each(?3)) GROUP BY j.value ORDER BY n DESC,j.value LIMIT 3")?;
            p.genres=stmt.query_map(params![guild.to_string(),user.to_string(),channels],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(p)
        }).await
    }
    pub async fn delete_user(&self, user: u64) -> Result<usize> {
        self.call(move |c| {
            let n = c.execute("DELETE FROM quotes WHERE user_id=?1", [user.to_string()])?;
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            Ok(n)
        })
        .await
    }
}
