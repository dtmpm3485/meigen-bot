use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuildSettings {
    pub enabled: bool,
    pub threshold: u8,
    pub user_cooldown: u64,
    pub output_channel_id: Option<u64>,
    pub excluded_channels: Vec<u64>,
}
impl Default for GuildSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 75,
            user_cooldown: 60,
            output_channel_id: None,
            excluded_channels: Vec::new(),
        }
    }
}
impl GuildSettings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (65..=95).contains(&self.threshold),
            "threshold must be 65..95"
        );
        anyhow::ensure!(
            self.user_cooldown <= 86400,
            "cooldown must be 0..86400 seconds"
        );
        anyhow::ensure!(
            self.excluded_channels.len() <= 500,
            "too many excluded channels"
        );
        Ok(())
    }
}
